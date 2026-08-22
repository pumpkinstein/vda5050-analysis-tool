# vda5050-analysis-tool

## Workspace

This is a Rust workspace using edition 2024 and resolver 3.

Workspace crates:

- `hmi`: Dioxus desktop application
- `cli`: command-line interface
- `log-file-parser`: log parsing and I/O
- `vda5050-analysis`: data analysis library
- `vda5050-data-types`: shared VDA 5050 domain types

## General rules

- Keep changes focused and avoid unrelated refactoring.
- Prefer existing workspace dependencies.
- Do not add dependencies without explaining why.
- Preserve public APIs and serialized data formats unless the task explicitly changes them.
- Keep UI concerns out of library crates.
- Inspect the relevant crate instructions before editing it.
- Run formatting and targeted checks after Rust changes.
- Keep conversation output to a minimum.

## Validation

Use the smallest relevant checks first:

- `cargo fmt --all -- --check`
- `cargo check -p <crate>`
- `cargo test -p <crate>`

Use `cargo check --workspace` for cross-crate changes.
