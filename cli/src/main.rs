mod report;

use anyhow::Result;
use clap::Parser;
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

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

    let mut stdout = io::stdout().lock();
    report::write_report(&mut stdout, &result, args.verbose, processing_duration)?;
    writeln!(
        stdout,
        "\nEnd-to-end CLI time (including reporting): {:.2?}",
        start_time.elapsed()
    )?;

    Ok(())
}
