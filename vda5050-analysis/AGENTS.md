# vda5050-analysis

## Scope

These instructions apply to the `vda5050-analysis` crate.

This crate provides reusable, display-independent analysis over parsed VDA 5050 log data.

## Responsibilities

- Consume `log_file_parser::VdaAnalysisResult`.
- Provide deterministic, reusable analysis functions and result types.
- Use Polars for DataFrame-oriented operations.
- Keep presentation concerns in `hmi` and command-line concerns in `cli`.
- Keep log ingestion and message parsing in `log-file-parser`.

## Data contracts

- Treat `VdaAnalysisResult` as the parser-owned source of truth.
- Treat the `index` DataFrame as the canonical table for shared message metadata.
- Preserve `row_id` relationships between `index` and message-specific DataFrames.
- Preserve the parser’s timestamp unit: nanoseconds.
- Use UTC consistently when converting timestamps to `chrono` types.
- Do not silently change the meaning of public fields or serialized values.

## Analysis behavior

- Prefer small, deterministic functions over stateful analysis objects.
- Avoid copying complete DataFrames unless there is a clear reason.
- Handle missing DataFrames and columns defensively.
- Preserve the tolerant behavior of existing APIs such as `summarize`.
- Return `None`, empty collections, or zero values where the existing API documents fallback behavior.
- Avoid panics when processing incomplete or malformed parsed data.

## Public API

- Document new public types, fields, and functions.
- Keep public result types independent of UI frameworks.
- Treat changes to public fields or semantics as API changes.
- Update tests and documentation when changing analysis behavior.

## Testing

Add or update tests for:

- Empty parser results.
- Missing DataFrames or columns.
- Invalid timestamps.
- Message counts.
- Parse success rates.
- Unique robot counting.
- Multi-day timestamp ranges.
- Parser failure ordering.
- Changes to the parser’s DataFrame schema.

Run the smallest relevant checks first:

```sh
cargo fmt --all -- --check
cargo check -p vda5050-analysis
cargo test -p vda5050-analysis
