use anyhow::{Result, bail};
use clap::{Args as ClapArgs, Parser, Subcommand};
use cli::{DEFAULT_BATCH_SIZE, correctness, report};
use log_file_parser::{DEFAULT_ROOT_TOPIC, process_log_file};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// A correctness-oriented VDA 5050 log analysis tool.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Render the existing DataFrame inspection report.
    QuickView(QuickViewArgs),
    /// Validate parser structure and optionally compare a reviewed manifest.
    Check(CheckArgs),
    /// Generate a source-bound candidate expectation manifest.
    Snapshot(SnapshotArgs),
}

#[derive(Debug, ClapArgs)]
struct CommonArgs {
    /// Path to the VDA 5050 log file
    #[arg(short, long)]
    file: PathBuf,

    /// Root MQTT topic used by the VDA 5050 messages
    #[arg(long, default_value = DEFAULT_ROOT_TOPIC)]
    root_topic: String,

    /// Batch size for parallel processing (default: 4,000)
    #[arg(
        short,
        long,
        default_value_t = DEFAULT_BATCH_SIZE,
        value_parser = parse_batch_size
    )]
    batch_size: usize,
}

#[derive(Debug, ClapArgs)]
struct QuickViewArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Show example parse failures for debugging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, ClapArgs)]
struct CheckArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Compare stable aggregates with this reviewed YAML expectation manifest
    #[arg(long)]
    expect: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
struct SnapshotArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Explicit output path for the candidate YAML expectation manifest
    #[arg(long)]
    output: PathBuf,

    /// Allow replacement of an existing output file
    #[arg(long, alias = "force")]
    overwrite: bool,
}

fn parse_batch_size(value: &str) -> std::result::Result<usize, String> {
    let batch_size = value
        .parse::<usize>()
        .map_err(|_| "batch size must be a positive integer".to_string())?;
    if batch_size == 0 {
        return Err("batch size must be greater than zero".to_string());
    }
    Ok(batch_size)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::QuickView(args) => run_quick_view(args),
        Command::Check(args) => run_check(args),
        Command::Snapshot(args) => run_snapshot(args),
    }
}

fn run_quick_view(args: QuickViewArgs) -> Result<()> {
    println!("Parsing file: {:?}...", args.common.file);
    let start_time = Instant::now();

    let result = process_log_file(
        &args.common.file,
        &args.common.root_topic,
        args.common.batch_size,
        args.verbose,
    )?;
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

fn run_check(args: CheckArgs) -> Result<()> {
    let result = process_log_file(
        &args.common.file,
        &args.common.root_topic,
        args.common.batch_size,
        false,
    )?;
    let report = correctness::check_result(
        &result,
        &args.common.file,
        &args.common.root_topic,
        args.expect.as_deref(),
    )?;

    if let Some(success) = report.success_text() {
        print!("{success}");
        Ok(())
    } else {
        bail!("{}", report.failure_text());
    }
}

fn run_snapshot(args: SnapshotArgs) -> Result<()> {
    let result = process_log_file(
        &args.common.file,
        &args.common.root_topic,
        args.common.batch_size,
        false,
    )?;
    let manifest =
        correctness::build_manifest(&args.common.file, &args.common.root_topic, &result)?;
    correctness::write_manifest(&args.output, &manifest, args.overwrite)?;
    println!(
        "snapshot: wrote candidate manifest {}",
        args.output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn help_documents_all_mandatory_subcommands() {
        let error = Cli::try_parse_from(["cli", "--help"]).err();
        let help = error.map(|error| error.to_string()).unwrap_or_default();
        assert!(help.contains("quick-view"));
        assert!(help.contains("check"));
        assert!(help.contains("snapshot"));
        assert!(help.contains("Usage: cli <COMMAND>"));
    }

    #[test]
    fn mode_is_required_and_old_mode_less_form_is_rejected() {
        assert!(Cli::try_parse_from(["cli", "--file", "sample.log"]).is_err());
    }

    #[test]
    fn mode_specific_options_are_restricted() {
        assert!(
            Cli::try_parse_from(["cli", "quick-view", "--file", "sample.log", "--verbose"]).is_ok()
        );
        assert!(
            Cli::try_parse_from(["cli", "check", "--file", "sample.log", "--verbose"]).is_err()
        );
        assert!(
            Cli::try_parse_from([
                "cli",
                "snapshot",
                "--file",
                "sample.log",
                "--output",
                "expected.yaml",
                "--expect",
                "other.yaml"
            ])
            .is_err()
        );
    }

    #[test]
    fn zero_batch_size_is_rejected_by_argument_parsing() {
        let error =
            Cli::try_parse_from(["cli", "check", "--file", "sample.log", "--batch-size", "0"])
                .err();
        assert!(
            error
                .map(|error| error.to_string())
                .unwrap_or_default()
                .contains("batch size must be greater than zero")
        );
    }
}
