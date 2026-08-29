//! Reusable correctness and expectation-manifest logic for the CLI.

/// Default number of records processed per parser batch by CLI commands.
pub const DEFAULT_BATCH_SIZE: usize = 4_000;

/// Maximum number of joined visualization-context rows shown by the CLI.
pub const VISUALIZATION_CONTEXT_SAMPLE_LIMIT: usize = 3;

pub mod correctness;
pub mod report;
