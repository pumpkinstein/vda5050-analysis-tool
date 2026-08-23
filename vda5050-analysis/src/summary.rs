use chrono::{DateTime, Duration, Utc};
use log_file_parser::{MessageType, VdaAnalysisResult};
use polars::prelude::{ChunkAgg, DataFrame, DataType};

use crate::robots::{
    RobotIdentity, count_unique_robot_identities_from_index, unique_robot_identities,
};

/// Display-independent statistics derived from a parsed VDA 5050 log.
///
/// The summary is intentionally a derived view. The complete
/// [`VdaAnalysisResult`] and its Polars DataFrames remain the source of truth
/// for later filtering and querying.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalysisSummary {
    /// Number of records found by the parser's chunk scanner.
    pub total_records: usize,
    /// Number of records represented in the parsed index DataFrame.
    pub parsed_records: usize,
    /// Sum of the parser's per-message-type failure counts.
    pub parse_failures: usize,
    /// Parsed records divided by total records, expressed as a percentage.
    pub parse_success_rate: f64,
    /// Number of distinct `(manufacturer, serial_number)` pairs in `index`.
    pub unique_robots: usize,
    /// Parsed message counts by type-specific DataFrame.
    pub message_counts: MessageCounts,
    /// Observed timestamp range in the `index` DataFrame, when available.
    pub time_range: Option<TimeRange>,
    /// Parser failures ordered by count descending. Ties have no promised order.
    pub failure_breakdown: Vec<FailureCount>,
}

/// Counts of successfully parsed messages by VDA 5050 message type.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MessageCounts {
    pub state: usize,
    pub visualization: usize,
    pub connection: usize,
    pub order: usize,
    pub instant_actions: usize,
}

/// Observed UTC timestamp range in the canonical `index` DataFrame.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration: Duration,
}

/// Number of parser failures for one message type.
#[derive(Clone, Debug, PartialEq)]
pub struct FailureCount {
    pub message_type: String,
    pub count: usize,
}

/// Summary values and distinct robot identities derived from one analysis
/// result.
#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisSnapshot {
    /// Aggregate values for the dashboard.
    pub summary: AnalysisSummary,
    /// Distinct robot identities for the robots view.
    pub robot_identities: Vec<RobotIdentity>,
}

/// Derive the dashboard summary and robot identities in one analysis pass.
pub fn analyze(result: &VdaAnalysisResult) -> AnalysisSnapshot {
    let robot_identities = unique_robot_identities(result);
    let summary = summarize_with_unique_robot_count(result, robot_identities.len());

    AnalysisSnapshot {
        summary,
        robot_identities,
    }
}

/// Derive the first-slice analysis summary from a parser result.
///
/// This function is infallible by design. Missing message frames and missing
/// index columns contribute zero values. A missing, uncastable, or otherwise
/// unusable `timestamp` column produces no time range. Timestamps are read as
/// nanoseconds, matching the parser's canonical `index` schema.
pub fn summarize(result: &VdaAnalysisResult) -> AnalysisSummary {
    let unique_robots = result
        .dataframes
        .get("index")
        .map(count_unique_robot_identities_from_index)
        .unwrap_or(0);
    summarize_with_unique_robot_count(result, unique_robots)
}

fn summarize_with_unique_robot_count(
    result: &VdaAnalysisResult,
    unique_robots: usize,
) -> AnalysisSummary {
    let total_records = result.total_chunks;
    let parsed_records = result.num_parsed;
    let parse_failures = result.parse_failures.values().sum();
    let parse_success_rate = if total_records > 0 {
        (parsed_records as f64 / total_records as f64) * 100.0
    } else {
        0.0
    };

    let message_counts = MessageCounts {
        state: dataframe_height(result, MessageType::State),
        visualization: dataframe_height(result, MessageType::Visualization),
        connection: dataframe_height(result, MessageType::Connection),
        order: dataframe_height(result, MessageType::Order),
        instant_actions: dataframe_height(result, MessageType::InstantActions),
    };

    let time_range = result
        .dataframes
        .get("index")
        .and_then(calculate_time_range);

    let mut failure_breakdown: Vec<_> = result
        .parse_failures
        .iter()
        .map(|(message_type, count)| FailureCount {
            message_type: message_type.clone(),
            count: *count,
        })
        .collect();
    failure_breakdown.sort_by_key(|failure| std::cmp::Reverse(failure.count));

    AnalysisSummary {
        total_records,
        parsed_records,
        parse_failures,
        parse_success_rate,
        unique_robots,
        message_counts,
        time_range,
        failure_breakdown,
    }
}

fn dataframe_height(result: &VdaAnalysisResult, message_type: MessageType) -> usize {
    result
        .dataframes
        .get(message_type.dataframe_name())
        .map(DataFrame::height)
        .unwrap_or(0)
}

fn calculate_time_range(df: &DataFrame) -> Option<TimeRange> {
    let timestamp_col = df.column("timestamp").ok()?;
    let i64_series = timestamp_col.cast(&DataType::Int64).ok()?;
    let timestamp_series = i64_series.i64().ok()?;
    let (start_ns, end_ns) = (timestamp_series.min()?, timestamp_series.max()?);

    let start = DateTime::from_timestamp_nanos(start_ns);
    let end = DateTime::from_timestamp_nanos(end_ns);

    Some(TimeRange {
        start,
        end,
        duration: Duration::nanoseconds(end_ns - start_ns),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use polars::prelude::{DataFrame, NamedFrom, Series, TimeUnit};
    use std::{collections::HashMap, time::Duration as StdDuration};

    fn result(
        dataframes: HashMap<String, DataFrame>,
        total_chunks: usize,
        num_parsed: usize,
        parse_failures: HashMap<String, usize>,
    ) -> VdaAnalysisResult {
        VdaAnalysisResult {
            dataframes,
            total_chunks,
            num_parsed,
            parse_failures,
            parse_examples: HashMap::new(),
            timings: log_file_parser::ProcessingTimings {
                mmap_setup: StdDuration::ZERO,
                delimiter_scanning: StdDuration::ZERO,
                parsing_and_builder_appends: StdDuration::ZERO,
                batch_dataframe_construction: StdDuration::ZERO,
                final_dataframe_concatenation: StdDuration::ZERO,
            },
        }
    }

    fn index_frame(
        manufacturers: &[&str],
        serial_numbers: &[&str],
        timestamps: &[i64],
    ) -> DataFrame {
        let timestamp = Series::new("timestamp".into(), timestamps.to_vec())
            .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))
            .unwrap();

        DataFrame::new_infer_height(vec![
            Series::new("manufacturer".into(), manufacturers).into(),
            Series::new("serial_number".into(), serial_numbers).into(),
            timestamp.into(),
        ])
        .unwrap()
    }

    fn rows_frame(rows: usize) -> DataFrame {
        DataFrame::new_infer_height(vec![Series::new("row_id".into(), 0..rows as u64).into()])
            .unwrap()
    }

    #[test]
    fn summarize_empty_input_returns_default_values() {
        let analysis = analyze(&result(HashMap::new(), 0, 0, HashMap::new()));

        assert_eq!(analysis.summary, AnalysisSummary::default());
        assert!(analysis.robot_identities.is_empty());
    }

    #[test]
    fn summarize_counts_robots_time_range_and_failure_order() {
        let start = Utc.with_ymd_and_hms(2024, 5, 20, 14, 35, 12).unwrap();
        let start_ns = start.timestamp_nanos_opt().unwrap();
        let end = start + Duration::seconds(65);
        let end_ns = end.timestamp_nanos_opt().unwrap();

        let mut dataframes = HashMap::new();
        dataframes.insert(
            "index".to_string(),
            index_frame(
                &["m1", "m1", "m1", "m2"],
                &["r1", "r1", "r2", "r1"],
                &[start_ns, start_ns, start_ns + 1_000_000_000, end_ns],
            ),
        );
        dataframes.insert("state".to_string(), rows_frame(2));
        dataframes.insert("visualization".to_string(), rows_frame(3));
        dataframes.insert("connection".to_string(), rows_frame(4));
        dataframes.insert("order".to_string(), rows_frame(5));
        dataframes.insert("instant_actions".to_string(), rows_frame(6));

        let mut failures = HashMap::new();
        failures.insert("order".to_string(), 1);
        failures.insert("state".to_string(), 3);
        failures.insert("connection".to_string(), 2);

        let analysis_result = result(dataframes, 5, 4, failures);
        let summary = summarize(&analysis_result);
        let analysis = analyze(&analysis_result);

        assert_eq!(summary.total_records, 5);
        assert_eq!(summary.parsed_records, 4);
        assert_eq!(summary.parse_failures, 6);
        assert_eq!(summary.parse_success_rate, 80.0);
        assert_eq!(summary.unique_robots, 3);
        assert_eq!(analysis.summary, summary);
        assert_eq!(analysis.robot_identities.len(), summary.unique_robots);
        assert_eq!(
            summary.message_counts,
            MessageCounts {
                state: 2,
                visualization: 3,
                connection: 4,
                order: 5,
                instant_actions: 6,
            }
        );
        assert_eq!(
            summary.time_range,
            Some(TimeRange {
                start,
                end,
                duration: Duration::seconds(65),
            })
        );
        assert_eq!(
            summary.failure_breakdown,
            vec![
                FailureCount {
                    message_type: "state".to_string(),
                    count: 3,
                },
                FailureCount {
                    message_type: "connection".to_string(),
                    count: 2,
                },
                FailureCount {
                    message_type: "order".to_string(),
                    count: 1,
                },
            ]
        );
    }

    #[test]
    fn summarize_preserves_multi_day_time_range() {
        let start = Utc.with_ymd_and_hms(2024, 5, 20, 23, 59, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 5, 22, 1, 1, 0).unwrap();
        let mut dataframes = HashMap::new();
        dataframes.insert(
            "index".to_string(),
            index_frame(
                &["m1", "m1"],
                &["r1", "r1"],
                &[
                    start.timestamp_nanos_opt().unwrap(),
                    end.timestamp_nanos_opt().unwrap(),
                ],
            ),
        );

        let summary = summarize(&result(dataframes, 2, 2, HashMap::new()));

        assert_eq!(summary.time_range.as_ref().unwrap().start, start);
        assert_eq!(summary.time_range.as_ref().unwrap().end, end);
        assert_eq!(
            summary.time_range.as_ref().unwrap().duration,
            Duration::hours(25) + Duration::minutes(2)
        );
    }

    #[test]
    fn summarize_missing_columns_and_invalid_timestamps_falls_back_to_zero() {
        let mut dataframes = HashMap::new();
        dataframes.insert(
            "index".to_string(),
            DataFrame::new_infer_height(vec![Series::new("other".into(), [1, 2]).into()]).unwrap(),
        );

        let summary = summarize(&result(dataframes, 2, 0, HashMap::new()));

        assert_eq!(summary.unique_robots, 0);
        assert_eq!(summary.message_counts, MessageCounts::default());
        assert_eq!(summary.time_range, None);

        let invalid_timestamp = DataFrame::new_infer_height(vec![
            Series::new("manufacturer".into(), ["m1"]).into(),
            Series::new("serial_number".into(), ["r1"]).into(),
            Series::new("timestamp".into(), ["not-a-timestamp"]).into(),
        ])
        .unwrap();
        let mut invalid_dataframes = HashMap::new();
        invalid_dataframes.insert("index".to_string(), invalid_timestamp);

        assert_eq!(
            summarize(&result(invalid_dataframes, 1, 1, HashMap::new())).time_range,
            None
        );
    }
}
