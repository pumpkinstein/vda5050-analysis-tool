use anyhow::Result;
use clap::Parser;
use polars::prelude::*;
use std::{path::PathBuf, time::Instant};

use log_file_parser::{DEFAULT_ROOT_TOPIC, process_log_file};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// A high-performance VDA 5050 log analysis tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the VDA 5050 log file
    #[arg(short, long)]
    file: PathBuf,

    /// Root MQTT topic used by the VDA 5050 messages
    #[arg(long, default_value = DEFAULT_ROOT_TOPIC)]
    root_topic: String,

    /// Show example parse failures for debugging
    #[arg(short, long)]
    verbose: bool,

    /// Batch size for parallel processing (default: 4,000)
    #[arg(short, long, default_value = "4000")]
    batch_size: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Parsing file: {:?}...", args.file);
    let start_time = Instant::now();

    let result = process_log_file(&args.file, &args.root_topic, args.batch_size, args.verbose)?;

    let processing_duration = start_time.elapsed();

    let num_parsed = result.num_parsed;
    let num_failed = result.total_chunks - num_parsed;

    println!(
        "Processed {} records in {:.2?}. (Ignored {} entries)",
        num_parsed, processing_duration, num_failed
    );

    println!("Ingestion timing:");
    println!("  mmap setup: {:.2?}", result.timings.mmap_setup);
    println!(
        "  delimiter scanning: {:.2?}",
        result.timings.delimiter_scanning
    );
    println!(
        "  JSON parsing and builder appends: {:.2?}",
        result.timings.parsing_and_builder_appends
    );
    println!(
        "  per-batch DataFrame construction: {:.2?}",
        result.timings.batch_dataframe_construction
    );
    println!(
        "  final DataFrame concatenation: {:.2?}",
        result.timings.final_dataframe_concatenation
    );

    // Show summary of parse failures by message type
    if num_failed > 0 {
        let failures = result.parse_failures;
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
                let examples = result.parse_examples;
                println!("\nExample parse failures:");
                for (msg_type, example) in examples.iter() {
                    println!("\n  Message type: {}", msg_type);
                    println!("  {}", example);
                }
            }
        }
    }

    let index_df = result.dataframes.get("index").unwrap();
    let state_df = result.dataframes.get("state").unwrap();
    let viz_df = result.dataframes.get("visualization").unwrap();
    let conn_df = result.dataframes.get("connection").unwrap();
    let order_df = result.dataframes.get("order").unwrap();
    let ia_df = result.dataframes.get("instant_actions").unwrap();

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
        let viz_with_context = viz_df.inner_join(index_df, ["row_id"], ["row_id"])?;
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

    println!(
        "\nEnd-to-end CLI time (including reporting): {:.2?}",
        start_time.elapsed()
    );

    Ok(())
}
