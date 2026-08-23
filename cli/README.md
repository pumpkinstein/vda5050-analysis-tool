# VDA 5050 analysis CLI

The `cli` crate provides three explicit modes for inspecting and checking VDA
5050 log files:

- `quick-view` prints a human-readable DataFrame report;
- `check` validates parser output and optionally compares it with a reviewed
  expectation manifest; and
- `snapshot` generates a source-bound YAML expectation manifest for review.

A subcommand is required. The former mode-less `cli --file ...` invocation is
not supported.

## Running the CLI

From the workspace root:

```sh
cargo run -p cli -- <SUBCOMMAND> [OPTIONS]
```

To build and run the release binary:

```sh
cargo build -p cli --release
./target/release/cli <SUBCOMMAND> [OPTIONS]
```

Use `cli --help` or `cli <SUBCOMMAND> --help` for the generated command help.

## Common arguments

All three subcommands accept these arguments:

| Argument | Required | Default | Description |
| --- | --- | --- | --- |
| `-f, --file <FILE>` | Yes | — | Path to the VDA 5050 log file. |
| `--root-topic <ROOT_TOPIC>` | No | `uagv/v1` | MQTT root topic used to recognize VDA 5050 records. A trailing slash is accepted. |
| `-b, --batch-size <BATCH_SIZE>` | No | `4000` | Number of records processed per parallel batch. It must be greater than zero. |

Batch size affects processing only; it is not stored in expectation manifests
and should not change semantic results. Use `--release` to run the release binary which is ~20x faster than the debug binary.

## `quick-view`

```sh
cargo run --release -p cli -- quick-view --file path/to/messages.log
cargo run --release -p cli -- quick-view --file path/to/messages.log --verbose
```

`quick-view` prints parser-stage timings, parse failures, canonical DataFrame
schemas and previews, the visualization/index join example, message-type
distribution, and end-to-end CLI time.

Its mode-specific option is:

| Argument | Description |
| --- | --- |
| `-v, --verbose` | Include an example for each classified parse-failure type. |

## `check`

Run structural validation without an expectation manifest:

```sh
cargo run -p cli -- check --file path/to/messages.log
```

Compare the result with a reviewed manifest:

```sh
cargo run -p cli -- check \
  --file path/to/messages.log \
  --expect path/to/messages.vda5050-expected.yaml
```

Structural checking validates the six canonical DataFrames, parser counts,
required schemas and data types, and links from message-specific rows to the
index DataFrame.

With `--expect`, the command additionally compares:

- source byte length and SHA-256;
- parser root topic;
- total, parsed, and ignored record counts;
- canonical frame and classified failure counts;
- message-type distribution and unique robot count; and
- the optional UTC timestamp range.

`check` never modifies the expectation file. It prints a compact report and
returns a nonzero exit status for parser errors, structural violations, an
invalid manifest, or semantic mismatches.

## `snapshot`

```sh
cargo run -p cli -- snapshot \
  --file path/to/messages.log \
  --output path/to/messages.vda5050-expected.yaml
```

`snapshot` parses the source once, runs structural validation, and writes a
deterministic versioned YAML manifest. The manifest contains stable semantic
values and an exact source binding; it intentionally excludes durations,
absolute paths, batch size, and machine-specific values.

The output path is mandatory. An existing file is protected unless overwrite
is explicitly enabled:

```sh
cargo run -p cli -- snapshot \
  --file path/to/messages.log \
  --output path/to/messages.vda5050-expected.yaml \
  --overwrite
```

`--force` is accepted as an alias for `--overwrite`.

A generated manifest is a candidate snapshot, not independently proven ground
truth. Review it or compare it with an independent source before accepting it.
Once reviewed, use it explicitly with `check --expect`; manifests are not
auto-discovered.

## Suggested workflow

1. Generate a candidate manifest with `snapshot`.
2. Review its source binding and expected aggregates.
3. Commit the reviewed manifest next to its log fixture.
4. Run `check --expect` in local validation or CI to detect future drift.

## Performance benchmark harness

The stable Criterion target measures parser ingestion and public analysis
operations without starting the CLI process or rendering the quick-view
report. A smoke run uses the checked-in sample and reviewed expectation:

```sh
cargo test -p cli --bench pipeline
cargo test -p cli --all-targets
```

Actual measurements require both paths explicitly; they never fall back to the
small sample:

```sh
export VDA5050_BENCH_FILE=/absolute/path/to/representative.log
export VDA5050_BENCH_EXPECT=/absolute/path/to/representative.vda5050-expected.yaml
cargo bench -p cli --bench pipeline -- --save-baseline main
cargo bench -p cli --bench pipeline -- --baseline main
```

Optional controls are `VDA5050_BENCH_ROOT_TOPIC` (default `uagv/v1`) and
`VDA5050_BENCH_BATCH_SIZE` (default `4000`, never zero). Keep the source,
expectation, Rust version, target features, allocator feature, machine, power
mode, background load, `RAYON_NUM_THREADS`, root topic, batch size, and cache
policy identical between baseline and candidate. To compare the jemalloc
variant, add `--features jemalloc` to both commands; the harness reports the
selected allocator and Rayon thread count. It also reports Cargo's full target
triple and sorted build-derived target-feature set; an empty set is reported as
no explicitly enabled features.

Before Criterion starts, the harness parses once with verbose capture disabled,
runs structural validation, verifies the exact source hash and byte length
against the expectation, and prints the five parser timings as diagnostics.
The benchmark is warm-page-cache by default after this preflight and Criterion
warm-up; it is not a reproducible cold-cache measurement. Input bytes are
reported as ingestion throughput. Large owned outputs (ingestion results,
DataFrames, robot identity vectors, and summary snapshots) are black-boxed and
dropped outside the measured interval. The small failure vector uses Criterion
small-input batching with the same destruction exclusion.

Criterion writes machine-local comparison data under `target/criterion`; do
not commit it. Its confidence intervals and outlier reports are for local
human review, not an automated performance gate. A large representative file
can make a run take several minutes, depending on its size and the configured
sampling times.
