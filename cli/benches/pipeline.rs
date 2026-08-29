use anyhow::{Context, Result, bail};
use cli::{DEFAULT_BATCH_SIZE, VISUALIZATION_CONTEXT_SAMPLE_LIMIT, correctness};
use criterion::{BatchSize, Bencher, Criterion, SamplingMode, Throughput};
use log_file_parser::{DEFAULT_ROOT_TOPIC, MessageType, VdaAnalysisResult, process_log_file};
use std::{
    env,
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{self, Command},
    time::Duration,
};
use vda5050_analysis::{
    analyze, failure_breakdown, message_type_distribution, summarize, unique_robot_identities,
    visualization_context_sample,
};

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

const SAMPLE_LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../log-file-parser/test-data/sample.log"
);
const SAMPLE_EXPECTATION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../log-file-parser/test-data/sample.vda5050-expected.yaml"
);
const ROOT_TOPIC_ENV: &str = "VDA5050_BENCH_ROOT_TOPIC";
const BATCH_SIZE_ENV: &str = "VDA5050_BENCH_BATCH_SIZE";
const FILE_ENV: &str = "VDA5050_BENCH_FILE";
const EXPECT_ENV: &str = "VDA5050_BENCH_EXPECT";
const INGESTION_SAMPLE_SIZE: usize = 10;
const INGESTION_WARM_UP: Duration = Duration::from_secs(2);
const INGESTION_MEASUREMENT: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct BenchmarkConfig {
    source_path: PathBuf,
    expectation_path: PathBuf,
    root_topic: String,
    batch_size: usize,
    input_bytes: u64,
    smoke_mode: bool,
}

#[derive(Debug)]
struct Preflight {
    result: VdaAnalysisResult,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("benchmark preflight failed: {error:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = BenchmarkConfig::from_environment()?;
    let preflight = run_preflight(&config)?;
    report_configuration(&config, &preflight.result);

    // Criterion recognizes cargo test's lack of --bench and executes each
    // routine once in test mode. The sample fixture is selected above for
    // that path, so cargo test --benches and --all-targets remain smoke tests
    // without becoming a performance baseline.
    let mut criterion = Criterion::default().configure_from_args();

    let index = preflight
        .result
        .dataframes
        .get("index")
        .context("preflight result is missing the index DataFrame")?;
    let visualization = preflight
        .result
        .dataframes
        .get(MessageType::Visualization.dataframe_name())
        .context("preflight result is missing the visualization DataFrame")?;

    benchmark_ingestion(&mut criterion, &config);
    benchmark_metadata(&mut criterion, &preflight.result);

    let mut analysis_group = criterion.benchmark_group("analysis/operations");
    analysis_group.measurement_time(Duration::from_secs(10));
    analysis_group.bench_function("message_type_distribution", |bencher| {
        measure_successful_output(bencher, "message_type_distribution", || {
            message_type_distribution(index)
        });
    });
    analysis_group.bench_function("visualization_context_sample", |bencher| {
        measure_successful_output(bencher, "visualization_context_sample", || {
            visualization_context_sample(index, visualization, VISUALIZATION_CONTEXT_SAMPLE_LIMIT)
        });
    });
    analysis_group.bench_function("unique_robot_identities", |bencher| {
        measure_owned_output(bencher, || unique_robot_identities(&preflight.result));
    });
    analysis_group.bench_function("summarize", |bencher| {
        measure_owned_output(bencher, || summarize(&preflight.result));
    });
    analysis_group.bench_function("analyze", |bencher| {
        measure_owned_output(bencher, || analyze(&preflight.result));
    });
    analysis_group.finish();

    criterion.final_summary();

    Ok(())
}

impl BenchmarkConfig {
    fn from_environment() -> Result<Self> {
        // Cargo appends --bench when it invokes a custom target through
        // `cargo bench`. Without it, Criterion is being run by cargo test and
        // the checked-in fixture is intentionally used for smoke coverage.
        let cargo_bench = env::args().any(|argument| argument == "--bench");
        let (source_path, expectation_path) = if cargo_bench {
            (required_path(FILE_ENV)?, required_path(EXPECT_ENV)?)
        } else {
            (PathBuf::from(SAMPLE_LOG), PathBuf::from(SAMPLE_EXPECTATION))
        };

        let (root_topic, batch_size) = if cargo_bench {
            let root_topic = env_or_default(ROOT_TOPIC_ENV, DEFAULT_ROOT_TOPIC)?;
            let root_topic = correctness::normalize_root_topic(&root_topic)
                .context("invalid VDA5050_BENCH_ROOT_TOPIC")?;
            let batch_size = env_or_default(BATCH_SIZE_ENV, &DEFAULT_BATCH_SIZE.to_string())?
                .parse::<usize>()
                .with_context(|| format!("{BATCH_SIZE_ENV} must be a positive integer"))?;
            if batch_size == 0 {
                bail!("{BATCH_SIZE_ENV} must be greater than zero");
            }
            (root_topic, batch_size)
        } else {
            // Smoke mode deliberately ignores all VDA5050_BENCH_* overrides:
            // it must always exercise the checked-in fixture with the same
            // production defaults used by the CLI.
            (DEFAULT_ROOT_TOPIC.to_string(), DEFAULT_BATCH_SIZE)
        };

        let input_bytes = fs::metadata(&source_path)
            .with_context(|| {
                format!(
                    "failed to inspect benchmark input {}",
                    source_path.display()
                )
            })?
            .len();

        Ok(Self {
            source_path,
            expectation_path,
            root_topic,
            batch_size,
            input_bytes,
            smoke_mode: !cargo_bench,
        })
    }
}

fn required_path(name: &str) -> Result<PathBuf> {
    let value = env::var_os(name).with_context(|| {
        format!("{name} is required for actual cargo bench runs; use the checked-in fixture only through cargo test")
    })?;
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(path)
}

fn env_or_default(name: &str, default: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(env::VarError::NotUnicode(_)) => bail!("{name} must be valid UTF-8"),
    }
}

fn run_preflight(config: &BenchmarkConfig) -> Result<Preflight> {
    let result = process_log_file(
        &config.source_path,
        &config.root_topic,
        config.batch_size,
        false,
    )
    .with_context(|| {
        format!(
            "failed to parse benchmark input {}",
            config.source_path.display()
        )
    })?;

    let report = correctness::check_result(
        &result,
        &config.source_path,
        &config.root_topic,
        Some(&config.expectation_path),
    )?;
    if !report.is_success() {
        bail!("{}", report.failure_text());
    }

    validate_analysis_operations(&result)?;

    Ok(Preflight { result })
}

fn validate_analysis_operations(result: &VdaAnalysisResult) -> Result<()> {
    let index = result.dataframes.get("index").context(
        "preflight result is missing the index DataFrame required by analysis benchmarks",
    )?;
    let visualization = result
        .dataframes
        .get(MessageType::Visualization.dataframe_name())
        .context(
            "preflight result is missing the visualization DataFrame required by the visualization_context_sample benchmark",
        )?;

    message_type_distribution(index).with_context(
        || "analysis preflight failed: message_type_distribution could not execute successfully",
    )?;
    visualization_context_sample(index, visualization, VISUALIZATION_CONTEXT_SAMPLE_LIMIT)
        .with_context(
            || "analysis preflight failed: visualization_context_sample could not execute successfully",
        )?;

    Ok(())
}

fn report_configuration(config: &BenchmarkConfig, result: &VdaAnalysisResult) {
    println!(
        "benchmark mode: {}",
        if config.smoke_mode {
            "smoke"
        } else {
            "measurement"
        }
    );
    println!("benchmark preflight: structural and expectation checks passed");
    println!("input bytes: {}", config.input_bytes);
    println!("root topic: {}", config.root_topic);
    println!("batch size: {}", config.batch_size);
    println!("rustc: {}", rustc_version());
    println!("target: {}", env!("CLI_BUILD_TARGET"));
    println!("compiled target features: {}", compiled_target_features());
    println!(
        "allocator: {}",
        if cfg!(feature = "jemalloc") {
            "jemalloc"
        } else {
            "system/default"
        }
    );
    println!("rayon threads: {}", rayon::current_num_threads());
    println!(
        "warm page cache: yes (preflight and Criterion warm-up; this is not a cold-cache result)"
    );
    println!("parser preflight timings (diagnostic only):");
    println!("  mmap setup: {:?}", result.timings.mmap_setup);
    println!(
        "  delimiter scanning: {:?}",
        result.timings.delimiter_scanning
    );
    println!(
        "  parsing and builder appends: {:?}",
        result.timings.parsing_and_builder_appends
    );
    println!(
        "  batch DataFrame construction: {:?}",
        result.timings.batch_dataframe_construction
    );
    println!(
        "  final DataFrame concatenation: {:?}",
        result.timings.final_dataframe_concatenation
    );
}

fn rustc_version() -> String {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    match Command::new(rustc).arg("--version").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => format!("unavailable (rustc exited with {})", output.status),
        Err(error) => format!("unavailable ({error})"),
    }
}

fn compiled_target_features() -> String {
    let features = env!("CLI_BUILD_TARGET_FEATURES");
    if features.is_empty() {
        "none explicitly enabled".to_string()
    } else {
        features.to_string()
    }
}

fn benchmark_ingestion(criterion: &mut Criterion, config: &BenchmarkConfig) {
    let mut group = criterion.benchmark_group("ingestion");
    group
        .sample_size(INGESTION_SAMPLE_SIZE)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(INGESTION_WARM_UP)
        .measurement_time(INGESTION_MEASUREMENT)
        .throughput(Throughput::Bytes(config.input_bytes));

    let source_path = &config.source_path;
    let root_topic = config.root_topic.as_str();
    let batch_size = config.batch_size;
    group.bench_function("process_log_file", |bencher| {
        measure_owned_output(bencher, || {
            match process_log_file(source_path, root_topic, batch_size, false) {
                Ok(result) => result,
                Err(error) => panic!("timed process_log_file failed: {error}"),
            }
        });
    });
    group.finish();
}

fn benchmark_metadata(criterion: &mut Criterion, result: &VdaAnalysisResult) {
    let mut group = criterion.benchmark_group("analysis/metadata");
    group.bench_function("failure_breakdown", |bencher| {
        // This Vec is small (parse failures contain one entry per message
        // type), so SmallInput keeps its destruction outside timing without
        // the substantial PerIteration overhead or unbounded memory growth.
        bencher.iter_batched(|| (), |_| failure_breakdown(result), BatchSize::SmallInput);
    });
    group.finish();
}

fn measure_owned_output<O, F>(bencher: &mut Bencher<'_>, mut operation: F)
where
    F: FnMut() -> O,
{
    // PerIteration keeps one potentially large returned value at a time.
    // Criterion black-boxes and drops it after the timer stops, so owned
    // output destruction is excluded without an extra timed black_box call.
    bencher.iter_batched(|| (), |_| operation(), BatchSize::PerIteration);
}

fn measure_successful_output<O, E, F>(
    bencher: &mut Bencher<'_>,
    operation_name: &str,
    mut operation: F,
) where
    E: std::fmt::Display,
    F: FnMut() -> Result<O, E>,
{
    bencher.iter_batched(
        || (),
        |_| match operation() {
            Ok(output) => output,
            Err(error) => panic!("timed {operation_name} failed after preflight: {error}"),
        },
        BatchSize::PerIteration,
    );
}
