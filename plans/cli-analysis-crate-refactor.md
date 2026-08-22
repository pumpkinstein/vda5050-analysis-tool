# Move CLI analysis into `vda5050-analysis`

Status: Ready for implementation.

## Objective

Make `cli` an orchestration and terminal-presentation layer. All statistics and
cross-frame analysis derived from `VdaAnalysisResult` should live in
`vda5050-analysis`, while the CLI preserves its current arguments, section
order, labels, DataFrame schemas/samples, failure diagnostics, and timing
output.

The migration must not make the warm-cache Ghostrun materially slower. The
candidate median end-to-end time may be at most 5 ms slower than the baseline
median under the same release build, CPU flags, output sink, and CLI arguments.

## Current boundary and problems

`log-file-parser` correctly owns ingestion, parsing, the six canonical
DataFrames, parse examples, and per-stage ingestion timings.
`vda5050-analysis` already owns the HMI's reusable summary and robot analysis.
The CLI predates that crate and still derives data itself:

- `total_chunks - num_parsed` for the ignored-record count;
- sorted parse-failure counts;
- row counts for the index and five message DataFrames;
- a full visualization/index join and fixed projection for the example query;
- `msg_type.value_counts(...)` for the distribution table.

The CLI should continue to own only:

- argument parsing and invocation of `process_log_file`;
- wall-clock measurements whose boundary includes CLI parsing/reporting;
- rendering parser-owned ingestion timings and parse examples;
- headings, prose, newlines, number/time formatting, schemas, and row previews.

Raw DataFrame schemas and `head(5)` previews are presentation/inspection rather
than reusable statistics, so they remain in the CLI. The cross-message join is
reusable analysis and moves to `vda5050-analysis`.

`log-file-parser`, its schemas, builders, and parsing algorithms are out of
scope. The HMI's use of `analyze`/`summarize` must remain unchanged.

## Compatibility requirements

For valid parser output, preserve the current stdout apart from naturally
variable durations and Polars' randomly generated categorical mapping IDs.
In particular, preserve:

- every heading, label, blank line, and section order;
- the distinction between `num_parsed` and ignored records;
- descending failure counts and the current verbose failure examples;
- each frame's schema, five-row preview, and empty-frame behavior;
- the visualization projection and column order:
  `manufacturer`, `serial_number`, `timestamp`, `x`, `y`, `theta`, `map_id`;
- the three-row limit applied after the current inner join/projection;
- the distribution table's `msg_type` and `count` columns, descending counts,
  and Polars `Enum`/`UInt32` dtypes;
- the start/end points of both reported CLI timers.

Do not replace the distribution DataFrame with hand-formatted
`MessageCounts`. Although the values would match, that would change dtypes,
ordering behavior, table layout, and therefore output.

Do not call the existing broad `summarize()` or `analyze()` from the CLI. Those
APIs also scan all timestamps and deduplicate robot identities, neither of
which the CLI currently reports. On the 2.5-million-row file that would add
avoidable full-column work and jeopardize the timing requirement.

## Shared analysis API

Add small, composable, display-independent APIs rather than a CLI-specific
report object. Suggested public contract:

```rust
pub struct RecordCounts {
    pub total_records: usize,
    pub parsed_records: usize,
    pub ignored_records: usize,
}

pub struct CanonicalFrameCounts {
    pub index: usize,
    pub messages: MessageCounts,
}

pub fn record_counts(result: &VdaAnalysisResult) -> RecordCounts;

pub fn canonical_frame_counts(
    result: &VdaAnalysisResult,
) -> CanonicalFrameCounts;

pub fn failure_breakdown(
    result: &VdaAnalysisResult,
) -> Vec<FailureCount>;

pub fn message_type_distribution(
    index: &DataFrame,
) -> PolarsResult<DataFrame>;

pub fn visualization_context_sample(
    index: &DataFrame,
    visualization: &DataFrame,
    limit: usize,
) -> PolarsResult<DataFrame>;
```

Names can be adjusted during implementation if an equally clear existing
convention is found, but the separation and cost model should remain:

- record and frame counts are O(1), except for the tiny failure-map traversal;
- failure sorting allocates only the small result vector;
- the distribution performs exactly the existing `value_counts` operation;
- the visualization query performs the existing join/projection once and does
  not clone either source DataFrame;
- no API performs unrelated summary work as a side effect.

`ignored_records` should use a defensive subtraction while preserving current
results for valid parser output. Missing canonical frames count as zero in the
count API, matching the tolerant conventions of the analysis crate. The two
Polars-native query functions should return errors for missing/incompatible
columns rather than panic. Public types and behavior must be documented.

Refactor `summarize()` internally to reuse `record_counts`, message/frame count
logic, and `failure_breakdown`, but preserve every existing `AnalysisSummary`
field and semantic. In particular, `AnalysisSummary::parse_failures` remains
the sum of the parser's classified failure map; it must not silently become the
ignored-record count. Existing public types and functions remain source- and
behavior-compatible.

Suggested module placement:

- `vda5050-analysis/src/statistics.rs`: cheap count/failure primitives and the
  message-distribution query;
- `vda5050-analysis/src/queries.rs`: visualization/index context query;
- `vda5050-analysis/src/summary.rs`: broad summary composed from the shared
  primitives;
- `vda5050-analysis/src/lib.rs`: documented re-exports.

Keep Polars as the supported tabular boundary; do not introduce a second table
abstraction or UI/CLI formatting types.

## Implementation phases

### 1. Freeze the observable baseline

Before changing Rust code:

1. Build the current CLI once in release mode with
   `RUSTFLAGS="-C target-feature=+avx2"` and retain that binary as the baseline.
2. Capture stdout from both `log-file-parser/test-data/sample.log -v` and the
   Ghostrun command from the request.
3. Normalize only duration values and categorical UUIDs when diffing output.
   Everything else is part of the compatibility contract.
4. Run at least one fixture with parse failures and empty message frames;
   `sample.log` currently exercises both.
5. Record at least seven warm-cache Ghostrun measurements using the same output
   sink for baseline and candidate, discarding the first warm-up run.

A three-run reconnaissance on the current checkout produced:

| Run | Parser duration | End-to-end duration |
| --- | ---: | ---: |
| 1 | 386.53 ms | 479.81 ms |
| 2 | 380.14 ms | 469.46 ms |
| 3 | 374.63 ms | 466.79 ms |

The reconnaissance median is 380.14 ms for parsing and 469.46 ms end to end.
These numbers are context, not a substitute for the same-session before/after
benchmark in phase 5.

### 2. Add and test narrow analysis primitives

Implement the public count and failure functions in the analysis crate:

- derive record totals from parser metadata without scanning a DataFrame;
- derive canonical frame counts with `DataFrame::height` exactly once per
  frame;
- move the existing descending failure sort without changing tie semantics;
- make `summarize()` consume those helpers so there is one definition of each
  statistic.

Add analysis-crate unit tests for:

- empty results and missing frames;
- normal total/parsed/ignored counts;
- defensive behavior for inconsistent total/parsed metadata;
- all six canonical frame counts;
- descending failure counts, including equal-count entries without promising a
  new tie order;
- unchanged `summarize()` values and existing HMI-facing semantics.

Do not add `.unwrap()` or `.expect()` to library code. No new dependency is
needed.

### 3. Move the two Polars analyses

Move the current expressions, initially without optimization:

1. `message_type_distribution` performs the same materialized-series
   `value_counts(true, false, "count", false)` call on `index.msg_type`.
2. `visualization_context_sample` performs the same inner join on `row_id`, the
   same projection, and `head(Some(limit))` after that projection.

Tests should assert:

- distribution column names, dtypes, counts, and descending order;
- distribution behavior for an empty/malformed index is an error or empty
  result according to the documented contract, never a panic;
- exact visualization projection order and row values;
- limit behavior and preservation of current left-side row order for canonical
  parser data;
- empty and missing-column inputs return cleanly;
- duplicate/unmatched `row_id` fixtures document the existing inner-join
  behavior rather than relying on an untested optimization assumption.

Do not move `head(limit)` ahead of the join during the compatibility refactor.
That can be considered later only with parser-invariant tests proving identical
results. The first goal is moving ownership with zero additional work.

### 4. Thin and make the CLI report testable

Update `cli/Cargo.toml`:

- add the existing workspace crate `vda5050-analysis`;
- remove the unused direct `vda5050-data-types` dependency;
- remove the direct `polars` dependency once all aggregate/join trait usage has
  moved and the crate compiles without it;
- keep all current CLI features and arguments unchanged.

Split terminal rendering into a small private `cli/src/report.rs` module with a
`Write`-based function. Keep `main.rs` responsible for argument parsing,
calling the parser, and establishing timing boundaries. This allows output to
be captured in tests without changing the production text.

In the reporter:

- use `RecordCounts` for the processed and ignored values;
- use `CanonicalFrameCounts` for every printed frame record count and for the
  empty-frame branches;
- use the shared failure breakdown while continuing to render parser-owned
  examples exactly as today;
- print raw schemas and `head(5)` previews directly from the parser frames;
- call the shared visualization sample query with a limit of three;
- print the shared distribution DataFrame directly;
- retain every existing string and newline.

Replace frame lookup panics with propagated, contextual CLI errors if this can
be done without changing successful output. This is not permission to make
parser/schema changes.

Add CLI tests around the writer using fixed durations. At minimum verify:

- headings and sections occur in the same order;
- all record counts and stable table content for `sample.log` match the
  baseline;
- empty Order and InstantActions frames omit `Sample:` exactly as today;
- verbose failure summaries and both examples are present (compare equal-count
  entries without assuming HashMap iteration order);
- the visualization and distribution tables retain their exact columns and
  values;
- final elapsed time is still measured only after reporting has completed.

### 5. Validate output and performance

Run the smallest checks first, then the cross-crate checks required by the API
change:

```text
cargo fmt --all -- --check
cargo check -p vda5050-analysis
cargo test -p vda5050-analysis
cargo clippy -p vda5050-analysis -- -D warnings
cargo check -p cli
cargo test -p cli
cargo clippy -p cli -- -D warnings
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Then:

1. Diff baseline and candidate stdout for `sample.log -v` and Ghostrun after
   normalizing only durations and categorical UUIDs.
2. Build the candidate once with the same release flags.
3. Run baseline and candidate against the cached Ghostrun file at least seven
   times each, using the same terminal or redirected output sink and avoiding
   concurrent load.
4. Compare medians, not the fastest individual run. Accept only if candidate
   end-to-end median is no more than 5 ms slower and parser timing shows no
   systematic regression.
5. Confirm with source inspection that the CLI does not call `summarize`,
   `analyze`, `value_counts`, or DataFrame joins and that neither source frame
   is cloned.

If the performance gate fails, temporarily time the distribution and
visualization-query functions separately and remove accidental allocation or
duplicate execution. Do not weaken the gate by folding unused broad-summary
work into the comparison. Any query optimization that changes operation order
requires output-equivalence tests before adoption.

## Completion criteria

- `cli` calculates no VDA/data-derived counts, failure ordering,
  distributions, or cross-frame joins.
- `vda5050-analysis` exposes documented, reusable, display-independent APIs for
  every moved operation.
- Existing `AnalysisSummary`, `AnalysisSnapshot`, and HMI behavior remain
  compatible.
- Parser ownership, schemas, row IDs, timestamps, and serialized VDA 5050 data
  are unchanged.
- Stable CLI stdout matches the baseline for the small failure fixture and the
  Ghostrun file.
- The warm-cache candidate median is no more than 5 ms slower than its
  same-session baseline.
- The implementation adds no external dependencies and no library panics.
- Formatting, targeted tests/checks/clippy, and workspace validation pass.

## Deliberately deferred

- Changing or expanding the CLI's report content to show unique robots, time
  range, parse success rate, or other HMI summary fields.
- Changing DataFrame schemas or replacing Polars output with custom tables.
- Optimizing the full visualization join by relying on stronger row-ID
  assumptions.
- Parser performance, batching, failure classification, or ingestion timing
  changes.
