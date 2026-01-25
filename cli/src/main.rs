use anyhow::Result;
use clap::Parser;
use polars::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Mutex, time::Instant};

mod parser;
use log_file_parser::models::ParsedMessage;
use log_file_parser::mqtt_log_io::{VdaIterator, parse_version};
use parser::{builders::AllBuilders, process::parse_record};

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

    /// Batch size for parallel processing (default: 100,000)
    #[arg(short, long, default_value = "100000")]
    batch_size: usize,
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
        "Found {} potential VDA 5050 messages. Processing in parallel batches...",
        total_chunks
    );

    // 3. Process chunks in parallel batches using Rayon with per-thread builders
    // Track parse failures by message type
    let parse_failures: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    let parse_examples: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());

    // Split work into batches for parallel processing
    let batch_size = args.batch_size;
    let num_batches = (total_chunks + batch_size - 1) / batch_size;

    println!(
        "Processing {} batches of ~{} messages each...",
        num_batches, batch_size
    );

    // Collect DataFrames from each batch
    let batch_results: Vec<_> = (0..num_batches)
        .into_par_iter()
        .map(|batch_idx| {
            let start_idx = batch_idx * batch_size;
            let end_idx = (start_idx + batch_size).min(total_chunks);
            let batch_chunks = &chunks[start_idx..end_idx];

            // Per-thread builders - column-wise append
            let mut builders = AllBuilders::with_capacity(batch_size);
            let mut row_id_offset = start_idx as u64;

            for chunk in batch_chunks {
                match parse_record(chunk) {
                    Ok(message) => {
                        let topic = message.topic();
                        let msg_type = message.msg_type();

                        // Append to builders based on message type (column-wise, no row objects)
                        match &message {
                            ParsedMessage::State { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.state.append(
                                    row_id_offset,
                                    format!("{:?}", data.operating_mode).to_uppercase(),
                                    data.battery_state.battery_charge,
                                    !data.errors.is_empty(),
                                );
                            }
                            ParsedMessage::Visualization { data, .. } => {
                                let manufacturer = data
                                    .manufacturer
                                    .clone()
                                    .unwrap_or_else(|| topic.manufacturer.clone());
                                let serial_number = data
                                    .serial_number
                                    .clone()
                                    .unwrap_or_else(|| topic.serial_number.clone());

                                builders.index.append(
                                    row_id_offset,
                                    manufacturer,
                                    serial_number,
                                    msg_type.to_string(),
                                    data.header_id.unwrap(),
                                    data.timestamp.unwrap() * 1000, // Convert µs to ns
                                    parse_version(data.version.as_ref().unwrap()),
                                );

                                // Extract position data if available
                                let (x, y, theta, map_id) = if let Some(pos) = &data.agv_position {
                                    (
                                        Some(pos.x),
                                        Some(pos.y),
                                        Some(pos.theta),
                                        Some(pos.map_id.clone()),
                                    )
                                } else {
                                    (None, None, None, None)
                                };

                                builders
                                    .visualization
                                    .append(row_id_offset, x, y, theta, map_id);
                            }
                            ParsedMessage::Connection { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.connection.append(
                                    row_id_offset,
                                    format!("{:?}", data.connection_state).to_uppercase(),
                                );
                            }
                            ParsedMessage::Order { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.order.append(row_id_offset, data.order_id.clone());
                            }
                            ParsedMessage::InstantActions { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders
                                    .instant_actions
                                    .append(row_id_offset, data.actions.len() as u32);
                            }
                        }

                        row_id_offset += 1;
                    }
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
                                            examples.entry(msg_type.to_string()).or_insert_with(
                                                || {
                                                    let preview = if chunk_str.len() > 200 {
                                                        format!("{}...", &chunk_str[..200])
                                                    } else {
                                                        chunk_str.to_string()
                                                    };
                                                    format!("Error: {}\nExample: {}", e, preview)
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Finish builders for this batch and return DataFrames
            Ok((
                builders.index.finish()?,
                builders.state.finish()?,
                builders.visualization.finish()?,
                builders.connection.finish()?,
                builders.order.finish()?,
                builders.instant_actions.finish()?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let parsing_duration = start_time.elapsed();

    // 4. Concatenate batches into final DataFrames
    println!("Concatenating {} batches...", batch_results.len());

    let mut index_batches = Vec::new();
    let mut state_batches = Vec::new();
    let mut viz_batches = Vec::new();
    let mut conn_batches = Vec::new();
    let mut order_batches = Vec::new();
    let mut ia_batches = Vec::new();

    for (index, state, viz, conn, order, ia) in batch_results {
        if index.height() > 0 {
            index_batches.push(index);
        }
        if state.height() > 0 {
            state_batches.push(state);
        }
        if viz.height() > 0 {
            viz_batches.push(viz);
        }
        if conn.height() > 0 {
            conn_batches.push(conn);
        }
        if order.height() > 0 {
            order_batches.push(order);
        }
        if ia.height() > 0 {
            ia_batches.push(ia);
        }
    }

    let index_df = concatenate_dataframes(&index_batches)?;
    let state_df = concatenate_dataframes(&state_batches)?;
    let viz_df = concatenate_dataframes(&viz_batches)?;
    let conn_df = concatenate_dataframes(&conn_batches)?;
    let order_df = concatenate_dataframes(&order_batches)?;
    let ia_df = concatenate_dataframes(&ia_batches)?;

    let num_parsed = index_df.height();
    let num_failed = total_chunks - num_parsed;

    println!(
        "Processed {} records in {:.2?}. (Ignored {} entries)",
        num_parsed, parsing_duration, num_failed
    );

    // Show summary of parse failures by message type
    if num_failed > 0 {
        let failures = parse_failures.into_inner().unwrap();
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
                let examples = parse_examples.into_inner().unwrap();
                println!("\nExample parse failures:");
                for (msg_type, example) in examples.iter() {
                    println!("\n  Message type: {}", msg_type);
                    println!("  {}", example);
                }
            }
        }
    }

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

    println!("\nTotal processing time: {:.2?}", start_time.elapsed());

    Ok(())
}

/// Concatenates multiple DataFrames into one, or returns empty DataFrame if list is empty
fn concatenate_dataframes(dfs: &[DataFrame]) -> Result<DataFrame> {
    if dfs.is_empty() {
        // Return empty DataFrame with no columns
        return Ok(DataFrame::empty());
    }

    if dfs.len() == 1 {
        return Ok(dfs[0].clone());
    }

    // Use Polars vstack to vertically stack DataFrames
    let mut result = dfs[0].clone();
    for df in &dfs[1..] {
        result.vstack_mut(df)?;
    }
    Ok(result)
}
