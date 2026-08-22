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
- **Focused Edits:** Keep changes narrow; strictly avoid unrelated refactoring.
- **Dependencies:** Rely on existing workspace crates. Do not add external dependencies without explicit explanation.
- **Contract Stability:** Preserve public APIs and VDA 5050 serialized data formats unless explicitly instructed.
- **Architectural Isolation:** Keep UI concerns strictly inside `hmi`. Library crates must remain UI framework-agnostic.
- **Truth Grounding:** Never invent crate APIs, signature assumptions, or VDA 5050 schema details. Check code/docs or stop and ask if ambiguous.
- **Concise Output:** Keep conversation output minimal and actionable.
- Inspect the relevant crate instructions before editing it.
- Run formatting and targeted checks after Rust changes.
- Keep crate dependencies one-way; `vda5050-data-types` must not depend on other workspace crates.
- **Error Handling:** Prohibit `.unwrap()` and `.expect()` in library crates (`log-file-parser`, `vda5050-analysis`, `vda5050-data-types`). Propagate errors cleanly.
- **Memory & Parsing:** Prefer borrowing (`&str`, `&[T]`) over unnecessary allocations or `.clone()` calls in log parsing and data processing.
- Verify a dependency's API against `Cargo.toml`/source/docs rather than recalling it from memory when unsure.
- Skip `target/` and `Cargo.lock` when searching the repo; they're generated, not informative.
- Any plan created goes into `plans` directory.


## VDA 5050 fidelity
- `vda5050-data-types` is the source of truth for message shapes. Don't invent, rename, or guess fields from general recollection of the standard.
- The standard is still evolving (versions include 2.1.0 and a newer 3.0); verify field presence/optionality against the in-repo spec or schema instead of general recollection.


## Validation
Use the smallest relevant checks first:
- `cargo fmt --all -- --check`
- `cargo check -p <crate>`
- `cargo clippy -p <crate> -- -D warnings`
- `cargo test -p <crate>`
Use `cargo check --workspace` (and `cargo clippy --workspace -- -D warnings`) for cross-crate changes.
