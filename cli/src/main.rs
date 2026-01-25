use anyhow::Result;
use clap::Parser;
use polars::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Mutex, time::Instant};

mod parser;
use log_file_parser::mqtt_log_io::VdaIterator;
use parser::{models::ParsedRecord, process::parse_record};

/// A high-performance VDA 5050 log analysis tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the VDA 5050 log file
    #[arg(short, long)]
    file: PathBuf,

    /// Show example parse failures for debugging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Parsing file: {:?}...", args.file);
    let start_time = Instant::now();

    // 1. Memory-map the file for zero-copy reads.
    let file = File::open(args.file)?;
    // SAFETY: The file is read-only, and we are the only process accessing it.
    // The file will not be modified while the mmap is active.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };

    // 2. Use the custom iterator to find all VDA 5050 message chunks.
    let iterator = VdaIterator::new(&mmap);
    let chunks: Vec<_> = iterator.collect();
    let total_chunks = chunks.len();

    println!(
        "Found {} potential VDA 5050 messages. Parsing in parallel...",
        total_chunks
    );

    // 3. Process chunks in parallel using Rayon.
    // Track parse failures by message type
    let parse_failures: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    let parse_examples: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());

    let records: Vec<ParsedRecord> = chunks
        .into_par_iter()
        .filter_map(|chunk| {
            match parse_record(chunk) {
                Ok(record) => Some(record),
                Err(e) => {
                    // Extract message type from topic for failure statistics
                    let chunk_str = std::str::from_utf8(chunk).unwrap_or("");
                    if let Some(start) = chunk_str.find("uagv/v1/") {
                        let topic_part = &chunk_str[start + 8..];
                        if let Some(slash_pos) = topic_part.find('/') {
                            let rest = &topic_part[slash_pos + 1..];
                            if let Some(slash_pos2) = rest.find('/') {
                                let rest2 = &rest[slash_pos2 + 1..];
                                if let Some(space_pos) = rest2.find(' ') {
                                    let msg_type = &rest2[..space_pos];
                                    let mut failures = parse_failures.lock().unwrap();
                                    *failures.entry(msg_type.to_string()).or_insert(0) += 1;

                                    // Store first example of each failure type
                                    if args.verbose {
                                        let mut examples = parse_examples.lock().unwrap();
                                        examples.entry(msg_type.to_string()).or_insert_with(|| {
                                            let preview = if chunk_str.len() > 200 {
                                                format!("{}...", &chunk_str[..200])
                                            } else {
                                                chunk_str.to_string()
                                            };
                                            format!("Error: {}\nExample: {}", e, preview)
                                        });
                                    }
                                }
                            }
                        }
                    }
                    None
                }
            }
        })
        .collect();
    let parsing_duration = start_time.elapsed();
    let num_parsed = records.len();
    let num_failed = total_chunks - num_parsed;

    println!(
        "Parsed {} records in {:.2?}. (Ignored {} entries)",
        num_parsed, parsing_duration, num_failed
    );

    // Show summary of parse failures by message type
    if num_failed > 0 {
        let failures = parse_failures.lock().unwrap();
        if !failures.is_empty() {
            println!("\nParse failure summary:");
            let mut sorted_failures: Vec<_> = failures.iter().collect();
            sorted_failures.sort_by(|a, b| b.1.cmp(a.1));
            for (msg_type, count) in sorted_failures {
                println!(
                    "  {}: {} messages failed (missing required fields)",
                    msg_type, count
                );
            }

            // Show examples if verbose mode is enabled
            if args.verbose {
                let examples = parse_examples.lock().unwrap();
                println!("\nExample parse failures:");
                for (msg_type, example) in examples.iter() {
                    println!("\n  Message type: {}", msg_type);
                    println!("  {}", example);
                }
            }
        }
    }
    println!("Building DataFrames...");

    // 4. Split records by message type and build separate DataFrames
    // First, create index DataFrame with common fields
    let mut index_row_ids = Vec::with_capacity(num_parsed);
    let mut index_manufacturers = Vec::with_capacity(num_parsed);
    let mut index_serial_numbers = Vec::with_capacity(num_parsed);
    let mut index_msg_types = Vec::with_capacity(num_parsed);
    let mut index_header_ids = Vec::with_capacity(num_parsed);
    let mut index_timestamps = Vec::with_capacity(num_parsed);
    let mut index_versions = Vec::with_capacity(num_parsed);

    // Type-specific vectors
    let mut state_row_ids = Vec::new();
    let mut state_operating_modes = Vec::new();
    let mut state_battery_charges = Vec::new();
    let mut state_has_errors = Vec::new();

    let mut viz_row_ids = Vec::new();
    let mut viz_xs = Vec::new();
    let mut viz_ys = Vec::new();
    let mut viz_thetas = Vec::new();
    let mut viz_map_ids = Vec::new();

    let mut conn_row_ids = Vec::new();
    let mut conn_states = Vec::new();

    let mut order_row_ids = Vec::new();
    let mut order_ids = Vec::new();

    let mut ia_row_ids = Vec::new();
    let mut ia_action_counts = Vec::new();

    // Process records and split by type
    for (row_id, record) in records.into_iter().enumerate() {
        let row_id = row_id as u64;

        // Add to index
        index_row_ids.push(row_id);
        index_manufacturers.push(record.manufacturer);
        index_serial_numbers.push(record.serial_number.clone());
        index_msg_types.push(record.msg_type.clone());
        index_header_ids.push(record.header_id);
        index_timestamps.push(record.timestamp_us);
        index_versions.push(record.version_packed);

        // Add to type-specific vectors
        match record.msg_type.as_str() {
            "state" => {
                state_row_ids.push(row_id);
                state_operating_modes.push(record.operating_mode);
                state_battery_charges.push(record.battery_charge);
                state_has_errors.push(record.has_errors);
            }
            "visualization" => {
                viz_row_ids.push(row_id);
                viz_xs.push(record.x);
                viz_ys.push(record.y);
                viz_thetas.push(record.theta);
                viz_map_ids.push(record.map_id);
            }
            "connection" => {
                conn_row_ids.push(row_id);
                conn_states.push(record.operating_mode);
            }
            "order" => {
                order_row_ids.push(row_id);
                order_ids.push(record.operating_mode);
            }
            "instantActions" => {
                ia_row_ids.push(row_id);
                ia_action_counts.push(record.operating_mode);
            }
            _ => {}
        }
    }

    // Build Index DataFrame
    let index_df = df!(
        "row_id" => index_row_ids,
        "manufacturer" => index_manufacturers,
        "serial_number" => index_serial_numbers,
        "msg_type" => index_msg_types,
        "header_id" => index_header_ids,
        "timestamp" => Series::new("timestamp".into(), index_timestamps).cast(&DataType::Datetime(TimeUnit::Microseconds, None))?,
        "version_packed" => index_versions,
    )?;

    // Build State DataFrame
    let state_df = df!(
        "row_id" => state_row_ids,
        "operating_mode" => state_operating_modes,
        "battery_charge" => state_battery_charges,
        "has_errors" => state_has_errors,
    )?;

    // Build Visualization DataFrame
    let viz_df = df!(
        "row_id" => viz_row_ids,
        "x" => viz_xs,
        "y" => viz_ys,
        "theta" => viz_thetas,
        "map_id" => viz_map_ids,
    )?;

    // Build Connection DataFrame
    let conn_df = df!(
        "row_id" => conn_row_ids,
        "connection_state" => conn_states,
    )?;

    // Build Order DataFrame
    let order_df = df!(
        "row_id" => order_row_ids,
        "order_id" => order_ids,
    )?;

    // Build InstantActions DataFrame
    let ia_df = df!(
        "row_id" => ia_row_ids,
        "action_count" => ia_action_counts,
    )?;

    let build_duration = start_time.elapsed() - parsing_duration;
    println!("DataFrames built in {:.2?}.", build_duration);

    // 5. Display results
    println!("\n=== Index DataFrame ===");
    println!("Total records: {}", index_df.height());
    println!("Schema: {:?}", index_df.schema());
    println!("Sample:");
    println!("{}", index_df.head(Some(5)));

    println!("\n=== State DataFrame ===");
    println!("Records: {}", state_df.height());
    println!("Schema: {:?}", state_df.schema());
    if state_df.height() > 0 {
        println!("Sample:");
        println!("{}", state_df.head(Some(5)));
    }

    println!("\n=== Visualization DataFrame ===");
    println!("Records: {}", viz_df.height());
    println!("Schema: {:?}", viz_df.schema());
    if viz_df.height() > 0 {
        println!("Sample:");
        println!("{}", viz_df.head(Some(5)));
    }

    println!("\n=== Connection DataFrame ===");
    println!("Records: {}", conn_df.height());
    println!("Schema: {:?}", conn_df.schema());
    if conn_df.height() > 0 {
        println!("Sample:");
        println!("{}", conn_df.head(Some(5)));
    }

    println!("\n=== Order DataFrame ===");
    println!("Records: {}", order_df.height());
    println!("Schema: {:?}", order_df.schema());
    if order_df.height() > 0 {
        println!("Sample:");
        println!("{}", order_df.head(Some(5)));
    }

    println!("\n=== InstantActions DataFrame ===");
    println!("Records: {}", ia_df.height());
    println!("Schema: {:?}", ia_df.schema());
    if ia_df.height() > 0 {
        println!("Sample:");
        println!("{}", ia_df.head(Some(5)));
    }

    // 6. Show example cross-message queries
    println!("\n=== Example: Cross-Message Query ===");
    println!("Join visualization data with index to get manufacturer + position:");

    if viz_df.height() > 0 {
        // Example: Join visualization with index to get manufacturer + timestamp + position
        let viz_with_context = viz_df.inner_join(&index_df, ["row_id"], ["row_id"])?;
        println!("Sample of joined data (manufacturer + timestamp + position):");
        println!(
            "{}",
            viz_with_context
                .select([
                    "manufacturer",
                    "serial_number",
                    "timestamp",
                    "x",
                    "y",
                    "theta",
                    "map_id"
                ])?
                .head(Some(3))
        );
    } else {
        println!("No visualization data to demonstrate join.");
    }

    println!("\n=== Summary Statistics ===");
    println!("Message type distribution:");
    let msg_type_col = index_df.column("msg_type")?;
    println!(
        "{}",
        msg_type_col
            .as_materialized_series()
            .value_counts(true, false, "count".into(), false)?
    );

    Ok(())
}
