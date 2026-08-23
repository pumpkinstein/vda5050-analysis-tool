//! Reusable, Polars-native analysis of parsed VDA 5050 log data.
//!
//! The parser remains the owner of ingestion and the six canonical DataFrames.
//! [`summarize`] derives display-independent values from its
//! [`log_file_parser::VdaAnalysisResult`] without copying those DataFrames.
//! The `index` DataFrame remains the canonical table for successfully parsed
//! messages: its `row_id` values link to the type-specific DataFrames, while
//! `timestamp`, `manufacturer`, `serial_number`, and `msg_type` provide the
//! shared filtering context for future Polars-native queries.

mod robots;
mod summary;

pub use robots::{RobotIdentity, unique_robot_identities};
pub use summary::{
    AnalysisSnapshot, AnalysisSummary, FailureCount, MessageCounts, TimeRange, analyze, summarize,
};
