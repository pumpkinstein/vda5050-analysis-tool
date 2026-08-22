# Shared VDA 5050 analysis crate

Status: Ready for implementation — HMI-only, Polars-first, compatibility-first scope.

## Objective

Move reusable data-analysis and querying logic out of the Dioxus HMI into a
separate workspace crate. The HMI should render analysis results, while the future
CLI should be able to consume the same analysis API without depending on Dioxus or
HMI code.

Polars DataFrames are a foundational representation for this tool, not an
implementation detail to hide behind a second table abstraction. The shared
analysis API may therefore expose and accept Polars types directly. This first
refactor does not change the parser or its DataFrame builders.

## Current architecture

- `vda5050-data-types` contains the deserializable VDA 5050 message models.
- `log-file-parser` owns file mapping, topic scanning, JSON parsing, parallel
  processing, and construction of six Polars DataFrames. It returns
  `VdaAnalysisResult`.
- `hmi/src/components/open_file.rs` invokes `process_log_file` and stores the
  parser result in `AppState`.
- `hmi/src/components/dashboard.rs` traverses `VdaAnalysisResult` directly to
  calculate record counts, message-type counts, unique robots, time range,
  duration, and failure breakdowns. It also formats those values for display.
- `cli/src/main.rs` currently performs its own reporting over the parser result,
  including failure summaries, DataFrame samples, joins, and value counts.

The main coupling is that the HMI owns `DashboardStats` and the
`calculate_stats` implementation, while the CLI has a separate set of analysis
and reporting calculations.

## Analysis crate contract

The new workspace member/package is named `vda5050-analysis` (Rust crate name
`vda5050_analysis`). It depends on `log-file-parser` and exposes reusable,
Polars-native analysis results. Its first input is `&VdaAnalysisResult`.

`log-file-parser` is explicitly out of scope for this refactor. It keeps file
ingestion, message parsing, parallel processing, DataFrame builders, and
`VdaAnalysisResult` construction. No parser batch API or alternate data model is
needed for the first migration.

The first-slice public entry point is intentionally small:

```rust
pub fn summarize(result: &log_file_parser::VdaAnalysisResult) -> AnalysisSummary;
```

The analysis crate has no Dioxus dependency and does not format UI strings.

## Future filtering and querying

The summary is a derived view, not a replacement for the parsed dataset. The HMI
must continue to retain the full `VdaAnalysisResult`, and future query functions
will operate on that result using Polars directly.

The existing `index` DataFrame is the canonical table for all successfully parsed
messages. Its `row_id` column links rows to the type-specific DataFrames, while
its `timestamp`, `manufacturer`, `serial_number`, and `msg_type` columns provide
the common filtering context. This relationship should be documented and tested
as part of the analysis crate rather than duplicated in future UI components.

A later query module can introduce a filter value such as a `TimeWindow` and
return a `PolarsResult<DataFrame>` (initially, a filtered index view). Queries that
need message-specific payloads can use the filtered `row_id` values to select or
join the relevant type-specific DataFrame. The exact result shape for a combined
payload view remains a later decision; the first migration should not invent a
second non-Polars dataset abstraction.

Time-window boundaries must eventually be explicit—for example, inclusive start
and exclusive end—and use the same UTC nanosecond timestamp semantics as the
existing index DataFrame. A query `TimeWindow` should remain distinct from the
observed-data `TimeRange` returned by `AnalysisSummary`.

## Performance strategy for future interactive queries

The first summary extraction should not add a second copy of the dataset. For
later queries, prefer indexing over caching arbitrary result DataFrames:

- Keep the parsed Polars frames as the source of truth.
- For a query such as robot + topic + time window, narrow the canonical `index`
  rows first, then use `row_id` to select or join the requested type-specific
  payload rows.
- Benchmark a direct Polars filter before adding an index. A compact query index
  can then be added if scans are too slow; it should store row references and
  ordering metadata, not duplicate message payload columns.
- Build that index once per loaded file, or lazily for robots that are queried,
  and discard it when the file changes.
- Do not add an unbounded cache of arbitrary filtered DataFrames. If repeated
  queries benefit from result caching, use a bounded, measurable cache with an
  explicit memory budget and invalidate it when the source file changes.

Polars can construct a `LazyFrame` from an in-memory `DataFrame`, and its lazy
optimizer can apply predicate and projection pushdown. Because this application
already materializes its DataFrames during parsing, this should be treated as a
query execution optimization—not as an automatic index or a guarantee that a
full in-memory scan is avoided. Query latency and resident memory must be
benchmarked against the current 1 GB-log/500 MB-RAM baseline before selecting an
index or cache strategy.

## Candidate target architecture

```text
vda5050-data-types
          ^
          |
log-file-parser  --->  vda5050-analysis
       ^                    ^
       |                    |
      HMI                  CLI
```

In the minimal version:

- `log-file-parser` remains responsible for ingestion, normalized tables, and
  DataFrame construction.
- `vda5050-analysis` owns summary/statistics calculations and their public Polars-
  native result types, with no Dioxus, signals, CSS, or UI-specific strings.
- The HMI calls the analysis crate and keeps only rendering concerns such as
  number separators, labels, line breaks, and loading state.
- The CLI can call the same summary functions and may continue to use parser
  DataFrames for detailed tabular output until a later reporting refactor.

The first public types are:

```rust
pub struct AnalysisSummary {
    pub total_records: usize,
    pub parsed_records: usize,
    pub parse_failures: usize,
    pub parse_success_rate: f64, // percentage in the range 0..=100
    pub unique_robots: usize,
    pub message_counts: MessageCounts,
    pub time_range: Option<TimeRange>,
    pub failure_breakdown: Vec<FailureCount>,
}

pub struct MessageCounts {
    pub state: usize,
    pub visualization: usize,
    pub connection: usize,
    pub order: usize,
    pub instant_actions: usize,
}

pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub duration: chrono::Duration,
}

pub struct FailureCount {
    pub message_type: String,
    pub count: usize,
}
```

These value types derive `Clone`, `Debug`, and `PartialEq`; applicable types also
derive `Default`. The summary function is infallible to preserve the current HMI
fallback behavior: missing message frames count as zero, missing index columns
produce zero unique robots, and an unavailable/invalid timestamp column produces
no time range.

The summary keeps the current semantics: total records come from `total_chunks`,
parsed records from `num_parsed`, parse failures are the sum of the parser’s
failure map, and success rate is calculated as `parsed_records / total_records *
100.0`, or `0.0` when there are no records. Failure entries are ordered by count
descending; ties have no promised order. The `TimeRange` uses UTC values derived
from the existing nanosecond timestamps. HMI formatting must continue to render
the current same-day/multi-day range and duration strings.

## Resolved decisions

- Polars remains the shared tabular representation and is not hidden behind a
  second abstraction layer.
- `log-file-parser` and its DataFrame builders are out of scope and remain
  unchanged.
- The shared summary type is named `AnalysisSummary`, rather than
  `DashboardStats`.
- The crate/package name is `vda5050-analysis` and its Rust crate name is
  `vda5050_analysis`.
- The public API is `summarize(&VdaAnalysisResult) -> AnalysisSummary`, supported
  by `MessageCounts`, `TimeRange`, and `FailureCount`.
- The full `VdaAnalysisResult` remains the dataset boundary so future Polars-native
  filters can be added without redesigning the first summary API.
- The first migration preserves existing dashboard values, labels, time-range
  formatting, and failure-count behavior exactly.
- The first migration does not change CLI output. The new crate must nevertheless
  be directly consumable by the CLI for a later reporting migration.

## Remaining decisions for later work

- Whether to add richer analyses beyond the initial dashboard summary.
- Whether the CLI should adopt `AnalysisSummary` for its reporting output.
- The exact future query/filter API, including whether a query returns the
  filtered `index` frame alone or a grouped/joined set of type-specific frames.
- Time-window boundary semantics and support for filters beyond time.
- Whether the first interactive-query implementation needs a global compact
  query index or can meet the latency target with Polars scans.
- The latency target, query-result size limits, and memory budget for interactive
  filtering.

## Implementation phases

### 0. Confirm the contract

- Implement the agreed crate name, dependency direction, and public API.
- Keep raw timestamps and duration in the shared result; keep display formatting
  in the HMI.
- Record compatibility expectations for empty frames, missing columns, and
  parser failures.
- Confirm that `log-file-parser` and its builders remain unchanged.
- Preserve the `index`/`row_id`/type-specific DataFrame relationship for future
  queries.

### 1. Add the workspace crate

- Add the crate to the workspace and establish its manifest dependencies.
- Use `log-file-parser`, workspace `polars`, and workspace `chrono` as the only
  initial crate dependencies; no Dioxus dependency is allowed.
- Create `src/summary.rs`, re-export its public types and `summarize` from
  `src/lib.rs`, and document the compatibility semantics.
- Make Polars an explicit, supported part of the crate API; do not introduce a
  parallel table abstraction.
- Keep all Dioxus and UI formatting out of the crate.
- Depend on the existing `log-file-parser::VdaAnalysisResult` rather than
  changing the parser boundary.

### 2. Move the dashboard analysis

- Port `calculate_stats`, unique-robot counting, time-range calculation, and
  failure ordering into the new crate as `AnalysisSummary` construction.
- Keep the existing DataFrame lookup and fallback behavior while moving the
  calculations.
- Add tests for empty input, same-day and multi-day ranges, duplicate robots,
  missing expected columns, and failure ordering.

### 3. Thin the HMI

- Add the new crate as an HMI dependency.
- Rename the cached HMI state from `dashboard_stats` to `analysis_summary` and
  store `AnalysisSummary` there.
- Compute/cache the shared summary from the loaded parser result in HMI state.
- Make `dashboard.rs` a renderer of public analysis types. Keep `format_number`
  and move/retain the exact time-range and duration display formatting in HMI.
- Leave `open_file.rs` responsible for invoking `process_log_file` and retaining
  the parser result.
- Remove HMI’s direct `polars` and `chrono` dependencies if no other component
  uses them after the dashboard migration.
- Verify that the rendered dashboard remains behaviorally identical during this
  migration.
- Preserve existing loading, error, and dashboard behavior unless phase 0
  approves a semantic change.

### 4. Prepare or migrate CLI reporting

- Do not change `cli/src/main.rs` or CLI output in the first migration.
- Add CLI adoption as a follow-up that calls `summarize` for shared reporting.
- Keep detailed DataFrame inspection and joins in CLI presentation code until a
  separate API is agreed for those operations.

### 5. Validate and document

- Run formatting, workspace checks, unit tests, and the relevant HMI/CLI builds.
- Compare old and new dashboard values against representative logs.
- Document the dependency direction, public API, table/schema assumptions, and
  extension points for future analyses and Polars-native filtering.
- Add query benchmarks before committing to materialized indexes or result
  caching; include robot/topic/time-window cases and both narrow and broad
  result sets.

## Completion criteria

- A separate workspace crate contains the agreed reusable analysis logic.
- Neither the analysis crate nor its public API depends on Dioxus.
- Polars remains the shared tabular representation and is not hidden behind a
  second abstraction layer.
- `log-file-parser` and its DataFrame builders are unchanged by this refactor.
- HMI dashboard rendering no longer implements the statistics itself.
- The CLI can consume the shared analysis API without importing HMI modules.
- Tests cover the agreed count, failure, robot, and time-range semantics.
- The plan’s resolved decisions and any deliberately deferred parser/schema work
  are recorded here for subsequent agents.
