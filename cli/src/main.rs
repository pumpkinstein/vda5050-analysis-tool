use anyhow::Result;
use clap::Parser;
use polars::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, fs::File, path::PathBuf, sync::Mutex, time::Instant};

mod parser;
use parser::{io::VdaIterator, models::ParsedRecord, process::parse_record};

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
    println!("Building DataFrame...");

    // 4. Convert the Vec<ParsedRecord> into a Polars DataFrame.
    let mut manufacturers = Vec::with_capacity(num_parsed);
    let mut serial_numbers = Vec::with_capacity(num_parsed);
    let mut msg_types = Vec::with_capacity(num_parsed);
    let mut header_ids = Vec::with_capacity(num_parsed);
    let mut timestamps_us = Vec::with_capacity(num_parsed);
    let mut versions_packed = Vec::with_capacity(num_parsed);
    let mut operating_modes = Vec::with_capacity(num_parsed);
    let mut battery_charges = Vec::with_capacity(num_parsed);
    let mut has_errors = Vec::with_capacity(num_parsed);
    let mut xs = Vec::with_capacity(num_parsed);
    let mut ys = Vec::with_capacity(num_parsed);
    let mut thetas = Vec::with_capacity(num_parsed);
    let mut map_ids = Vec::with_capacity(num_parsed);

    for record in records {
        manufacturers.push(record.manufacturer);
        serial_numbers.push(record.serial_number);
        msg_types.push(record.msg_type);
        header_ids.push(record.header_id);
        timestamps_us.push(record.timestamp_us);
        versions_packed.push(record.version_packed);
        operating_modes.push(record.operating_mode);
        battery_charges.push(record.battery_charge);
        has_errors.push(record.has_errors);
        xs.push(record.x);
        ys.push(record.y);
        thetas.push(record.theta);
        map_ids.push(record.map_id);
    }

    // As per the project plan, use Categorical encoding for repetitive strings.
    let df = df!(
        "manufacturer" => manufacturers,
        "serial_number" => serial_numbers,
        "msg_type" => msg_types,
        "header_id" => header_ids,
        "timestamp" => Series::new("timestamp".into(), timestamps_us).cast(&DataType::Datetime(TimeUnit::Microseconds, None))?,
        "version_packed" => versions_packed,
        "operating_mode" => operating_modes,
        "battery_charge" => battery_charges,
        "has_errors" => has_errors,
        "x" => xs,
        "y" => ys,
        "theta" => thetas,
        "map_id" => map_ids,
    )?;
    let build_duration = start_time.elapsed() - parsing_duration;
    println!("DataFrame built in {:.2?}.", build_duration);

    // 5. Display results.
    println!("\n--- DataFrame Schema ---");
    println!("{:?}", df.schema());
    println!("\n--- DataFrame Sample ---");
    println!("{}", df.head(Some(5)));

    Ok(())
}
