use log_file_parser::{MessageType, VdaAnalysisResult};
use polars::prelude::{DataFrame, PolarsResult, SeriesMethods};

/// Counts of records discovered and parsed by the log parser.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordCounts {
    /// Number of records found by the parser's chunk scanner.
    pub total_records: usize,
    /// Number of records represented in the parsed index DataFrame.
    pub parsed_records: usize,
    /// Number of records not represented in the parsed index DataFrame.
    pub ignored_records: usize,
}

/// Counts for the parser's canonical index and message DataFrames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanonicalFrameCounts {
    /// Number of rows in the canonical `index` DataFrame.
    pub index: usize,
    /// Number of rows in each message-specific canonical DataFrame.
    pub messages: MessageCounts,
}

/// Counts of successfully parsed messages by VDA 5050 message type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageCounts {
    /// Number of rows in the `state` DataFrame.
    pub state: usize,
    /// Number of rows in the `visualization` DataFrame.
    pub visualization: usize,
    /// Number of rows in the `connection` DataFrame.
    pub connection: usize,
    /// Number of rows in the `order` DataFrame.
    pub order: usize,
    /// Number of rows in the `instant_actions` DataFrame.
    pub instant_actions: usize,
}

/// Number of parser failures for one message type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureCount {
    /// The message type classified by the parser.
    pub message_type: String,
    /// Number of records classified as failures for this message type.
    pub count: usize,
}

/// Return parser record totals without scanning any DataFrame.
pub fn record_counts(result: &VdaAnalysisResult) -> RecordCounts {
    RecordCounts {
        total_records: result.total_chunks,
        parsed_records: result.num_parsed,
        ignored_records: result.total_chunks.saturating_sub(result.num_parsed),
    }
}

/// Return row counts for the canonical index and message DataFrames.
///
/// Missing canonical DataFrames contribute zero rows. Each present DataFrame
/// is queried once, and no other analysis is performed.
pub fn canonical_frame_counts(result: &VdaAnalysisResult) -> CanonicalFrameCounts {
    CanonicalFrameCounts {
        index: dataframe_height(result, "index"),
        messages: MessageCounts {
            state: dataframe_height(result, MessageType::State.dataframe_name()),
            visualization: dataframe_height(result, MessageType::Visualization.dataframe_name()),
            connection: dataframe_height(result, MessageType::Connection.dataframe_name()),
            order: dataframe_height(result, MessageType::Order.dataframe_name()),
            instant_actions: dataframe_height(result, MessageType::InstantActions.dataframe_name()),
        },
    }
}

/// Return parser failures ordered by count descending.
///
/// Equal-count entries retain the parser map's iteration-dependent order, as
/// no tie order is promised by the parser result.
pub fn failure_breakdown(result: &VdaAnalysisResult) -> Vec<FailureCount> {
    let mut failures: Vec<_> = result
        .parse_failures
        .iter()
        .map(|(message_type, count)| FailureCount {
            message_type: message_type.clone(),
            count: *count,
        })
        .collect();
    failures.sort_by_key(|failure| std::cmp::Reverse(failure.count));
    failures
}

/// Count the values in the canonical `index.msg_type` column.
///
/// This performs the same Polars `value_counts` operation used by the CLI.
/// Missing or incompatible columns are returned as [`PolarsResult`] errors;
/// an empty compatible column produces Polars' empty result.
pub fn message_type_distribution(index: &DataFrame) -> PolarsResult<DataFrame> {
    let msg_type_col = index.column("msg_type")?;
    msg_type_col
        .as_materialized_series()
        .value_counts(true, false, "count".into(), false)
}

fn dataframe_height(result: &VdaAnalysisResult, name: &str) -> usize {
    result
        .dataframes
        .get(name)
        .map(DataFrame::height)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::{NamedFrom, Series};
    use std::{
        collections::{HashMap, HashSet},
        time::Duration,
    };

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
                mmap_setup: Duration::ZERO,
                delimiter_scanning: Duration::ZERO,
                parsing_and_builder_appends: Duration::ZERO,
                batch_dataframe_construction: Duration::ZERO,
                final_dataframe_concatenation: Duration::ZERO,
            },
        }
    }

    fn rows_frame(rows: usize) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![Series::new("row_id".into(), 0..rows as u64).into()])
    }

    #[test]
    fn empty_results_and_missing_frames_are_zero() -> PolarsResult<()> {
        let empty = result(HashMap::new(), 0, 0, HashMap::new());

        assert_eq!(record_counts(&empty), RecordCounts::default());
        assert_eq!(
            canonical_frame_counts(&empty),
            CanonicalFrameCounts::default()
        );
        assert!(failure_breakdown(&empty).is_empty());
        Ok(())
    }

    #[test]
    fn record_counts_preserve_parser_metadata() {
        let analysis = result(HashMap::new(), 10, 7, HashMap::new());

        assert_eq!(
            record_counts(&analysis),
            RecordCounts {
                total_records: 10,
                parsed_records: 7,
                ignored_records: 3,
            }
        );
    }

    #[test]
    fn record_counts_defensively_saturate_inconsistent_metadata() {
        let analysis = result(HashMap::new(), 2, 5, HashMap::new());

        assert_eq!(record_counts(&analysis).ignored_records, 0);
    }

    #[test]
    fn canonical_frame_counts_cover_all_frames_and_missing_frames() -> PolarsResult<()> {
        let mut dataframes = HashMap::new();
        dataframes.insert("index".to_string(), rows_frame(7)?);
        dataframes.insert("state".to_string(), rows_frame(2)?);
        dataframes.insert("visualization".to_string(), rows_frame(3)?);
        dataframes.insert("connection".to_string(), rows_frame(4)?);
        dataframes.insert("order".to_string(), rows_frame(5)?);
        dataframes.insert("instant_actions".to_string(), rows_frame(6)?);

        assert_eq!(
            canonical_frame_counts(&result(dataframes, 0, 0, HashMap::new())),
            CanonicalFrameCounts {
                index: 7,
                messages: MessageCounts {
                    state: 2,
                    visualization: 3,
                    connection: 4,
                    order: 5,
                    instant_actions: 6,
                },
            }
        );
        Ok(())
    }

    #[test]
    fn failure_breakdown_is_descending_without_promising_tie_order() {
        let failures = HashMap::from([
            ("state".to_string(), 2),
            ("visualization".to_string(), 5),
            ("connection".to_string(), 5),
            ("order".to_string(), 1),
        ]);
        let breakdown = failure_breakdown(&result(HashMap::new(), 0, 0, failures));

        assert_eq!(
            breakdown
                .iter()
                .map(|failure| failure.count)
                .collect::<Vec<_>>(),
            vec![5, 5, 2, 1]
        );
        let equal_count_types: HashSet<_> = breakdown
            .iter()
            .filter(|failure| failure.count == 5)
            .map(|failure| failure.message_type.as_str())
            .collect();
        assert_eq!(
            equal_count_types,
            HashSet::from(["connection", "visualization"])
        );
    }
}
