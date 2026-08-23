//! Reusable, Polars-native analysis of parsed VDA 5050 log data.
//!
//! The parser remains the owner of ingestion and the six canonical DataFrames.
//! [`summarize`] derives display-independent values from its
//! [`log_file_parser::VdaAnalysisResult`] without copying those DataFrames.
//! The `index` DataFrame remains the canonical table for successfully parsed
//! messages: its `row_id` values link to the type-specific DataFrames, while
//! `timestamp`, `manufacturer`, `serial_number`, and `msg_type` provide the
//! shared filtering context for future Polars-native queries.

mod queries;
mod robots;
mod statistics;
mod summary;

pub use queries::visualization_context_sample;
pub use robots::{RobotIdentity, unique_robot_identities};
pub use statistics::{
    CanonicalFrameCounts, FailureCount, MessageCounts, RecordCounts, canonical_frame_counts,
    failure_breakdown, message_type_distribution, record_counts,
};
pub use summary::{AnalysisSnapshot, AnalysisSummary, TimeRange, analyze, summarize};
