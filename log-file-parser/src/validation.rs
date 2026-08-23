//! Structural validation for the parser's canonical result.

use crate::{MessageType, VdaAnalysisResult};
use polars::prelude::{AnyValue, DataFrame, DataType, TimeUnit};
use std::{collections::HashMap, error::Error, fmt};

/// The names and order of the DataFrames produced by [`crate::process_log_file`].
pub const CANONICAL_FRAME_NAMES: [&str; 6] = [
    "index",
    "state",
    "visualization",
    "connection",
    "order",
    "instant_actions",
];

/// All structural violations found in one parser result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors {
    violations: Vec<String>,
}

impl ValidationErrors {
    /// Return violations in deterministic validation order.
    pub fn violations(&self) -> &[String] {
        &self.violations
    }

    /// Consume the error and return its violations in deterministic order.
    pub fn into_violations(self) -> Vec<String> {
        self.violations
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("parser result validation failed")?;
        for violation in &self.violations {
            write!(formatter, "\n- {violation}")?;
        }
        Ok(())
    }
}

impl Error for ValidationErrors {}

/// Validate the parser-owned canonical result without scanning payload data.
///
/// Empty canonical frames are allowed to have no columns. Non-empty frames are
/// checked against the schemas produced by the parser, and every type-specific
/// row is checked against its corresponding `index` row and message type.
pub fn validate_result(result: &VdaAnalysisResult) -> Result<(), ValidationErrors> {
    let mut violations = Vec::new();

    for frame_name in CANONICAL_FRAME_NAMES {
        if !result.dataframes.contains_key(frame_name) {
            violations.push(format!("missing canonical DataFrame: {frame_name}"));
        }
    }

    let index = result.dataframes.get("index");
    let index_height = index.map(DataFrame::height);

    if let Some(index_height) = index_height {
        if result.num_parsed != index_height {
            violations.push(format!(
                "num_parsed {} does not equal index height {index_height}",
                result.num_parsed
            ));
        }

        let message_height_sum: usize = CANONICAL_FRAME_NAMES[1..]
            .iter()
            .map(|frame_name| {
                result
                    .dataframes
                    .get(*frame_name)
                    .map(DataFrame::height)
                    .unwrap_or(0)
            })
            .sum();
        if message_height_sum != index_height {
            violations.push(format!(
                "message frame heights sum to {message_height_sum}, expected index height {index_height}"
            ));
        }
    }

    if result.num_parsed > result.total_chunks {
        violations.push(format!(
            "num_parsed {} exceeds discovered chunks {}",
            result.num_parsed, result.total_chunks
        ));
    }

    validate_nonempty_schema(index, "index", INDEX_SCHEMA, &mut violations);
    validate_nonempty_schema(
        result.dataframes.get(MessageType::State.dataframe_name()),
        MessageType::State.dataframe_name(),
        STATE_SCHEMA,
        &mut violations,
    );
    validate_nonempty_schema(
        result
            .dataframes
            .get(MessageType::Visualization.dataframe_name()),
        MessageType::Visualization.dataframe_name(),
        VISUALIZATION_SCHEMA,
        &mut violations,
    );
    validate_nonempty_schema(
        result
            .dataframes
            .get(MessageType::Connection.dataframe_name()),
        MessageType::Connection.dataframe_name(),
        CONNECTION_SCHEMA,
        &mut violations,
    );
    validate_nonempty_schema(
        result.dataframes.get(MessageType::Order.dataframe_name()),
        MessageType::Order.dataframe_name(),
        ORDER_SCHEMA,
        &mut violations,
    );
    validate_nonempty_schema(
        result
            .dataframes
            .get(MessageType::InstantActions.dataframe_name()),
        MessageType::InstantActions.dataframe_name(),
        INSTANT_ACTIONS_SCHEMA,
        &mut violations,
    );

    validate_row_links(index, result, &mut violations);

    if violations.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors { violations })
    }
}

#[derive(Clone, Copy)]
enum ExpectedDataType {
    UInt32,
    UInt64,
    Float64,
    Boolean,
    String,
    Categorical,
    Enum,
    TimestampNanosecondsUtc,
}

type Schema = &'static [(&'static str, ExpectedDataType)];

const INDEX_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("manufacturer", ExpectedDataType::Categorical),
    ("serial_number", ExpectedDataType::Categorical),
    ("msg_type", ExpectedDataType::Enum),
    ("header_id", ExpectedDataType::UInt32),
    ("timestamp", ExpectedDataType::TimestampNanosecondsUtc),
    ("version_packed", ExpectedDataType::UInt32),
];

const STATE_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("operating_mode", ExpectedDataType::Enum),
    ("battery_charge", ExpectedDataType::Float64),
    ("has_errors", ExpectedDataType::Boolean),
];

const VISUALIZATION_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("x", ExpectedDataType::Float64),
    ("y", ExpectedDataType::Float64),
    ("theta", ExpectedDataType::Float64),
    ("map_id", ExpectedDataType::String),
];

const CONNECTION_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("connection_state", ExpectedDataType::Enum),
];

const ORDER_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("order_id", ExpectedDataType::String),
];

const INSTANT_ACTIONS_SCHEMA: Schema = &[
    ("row_id", ExpectedDataType::UInt64),
    ("action_count", ExpectedDataType::UInt32),
];

fn validate_nonempty_schema(
    dataframe: Option<&DataFrame>,
    frame_name: &str,
    schema: Schema,
    violations: &mut Vec<String>,
) {
    let Some(dataframe) = dataframe else {
        return;
    };
    if dataframe.height() == 0 {
        return;
    }

    for (column_name, expected) in schema {
        let Ok(column) = dataframe.column(column_name) else {
            violations.push(format!("{frame_name}: missing column {column_name}"));
            continue;
        };

        if !matches_expected_dtype(column.dtype(), *expected) {
            violations.push(format!(
                "{frame_name}.{column_name} has dtype {}, expected {}",
                dtype_label(column.dtype()),
                expected.label()
            ));
        }
    }
}

fn matches_expected_dtype(dtype: &DataType, expected: ExpectedDataType) -> bool {
    match expected {
        ExpectedDataType::UInt32 => matches!(dtype, DataType::UInt32),
        ExpectedDataType::UInt64 => matches!(dtype, DataType::UInt64),
        ExpectedDataType::Float64 => matches!(dtype, DataType::Float64),
        ExpectedDataType::Boolean => matches!(dtype, DataType::Boolean),
        ExpectedDataType::String => matches!(dtype, DataType::String),
        ExpectedDataType::Categorical => matches!(dtype, DataType::Categorical(_, _)),
        ExpectedDataType::Enum => matches!(dtype, DataType::Enum(_, _)),
        ExpectedDataType::TimestampNanosecondsUtc => {
            matches!(dtype, DataType::Datetime(TimeUnit::Nanoseconds, None))
        }
    }
}

impl ExpectedDataType {
    fn label(self) -> &'static str {
        match self {
            Self::UInt32 => "UInt32",
            Self::UInt64 => "UInt64",
            Self::Float64 => "Float64",
            Self::Boolean => "Boolean",
            Self::String => "String",
            Self::Categorical => "Categorical",
            Self::Enum => "Enum",
            Self::TimestampNanosecondsUtc => "Datetime(Nanoseconds, None)",
        }
    }
}

fn dtype_label(dtype: &DataType) -> String {
    match dtype {
        DataType::Categorical(_, _) => "Categorical".to_string(),
        DataType::Enum(_, _) => "Enum".to_string(),
        DataType::Datetime(unit, timezone) => {
            format!("Datetime({unit:?}, {timezone:?})")
        }
        other => format!("{other:?}"),
    }
}

fn validate_row_links(
    index: Option<&DataFrame>,
    result: &VdaAnalysisResult,
    violations: &mut Vec<String>,
) {
    let Some(index) = index else {
        return;
    };
    if index.height() == 0 {
        return;
    }

    let Some(index_row_ids) = u64_column(index, "row_id") else {
        return;
    };
    let Ok(index_message_types) = index.column("msg_type") else {
        return;
    };

    let mut index_rows = HashMap::with_capacity(index.height());
    for row in 0..index.height() {
        let Some(row_id) = index_row_ids.get(row) else {
            continue;
        };
        let Some(message_type) = string_value(index_message_types, row) else {
            continue;
        };
        index_rows.insert(row_id, message_type);
    }

    validate_frame_row_links(result, &index_rows, MessageType::State, violations);
    validate_frame_row_links(result, &index_rows, MessageType::Visualization, violations);
    validate_frame_row_links(result, &index_rows, MessageType::Connection, violations);
    validate_frame_row_links(result, &index_rows, MessageType::Order, violations);
    validate_frame_row_links(result, &index_rows, MessageType::InstantActions, violations);
}

fn validate_frame_row_links(
    result: &VdaAnalysisResult,
    index_rows: &HashMap<u64, String>,
    message_type: MessageType,
    violations: &mut Vec<String>,
) {
    let frame_name = message_type.dataframe_name();
    let Some(dataframe) = result.dataframes.get(frame_name) else {
        return;
    };
    if dataframe.height() == 0 {
        return;
    }
    let Some(row_ids) = u64_column(dataframe, "row_id") else {
        return;
    };
    let expected_message_type = message_type.as_str();

    for row in 0..dataframe.height() {
        let Some(row_id) = row_ids.get(row) else {
            violations.push(format!(
                "{frame_name} row {row} has no readable UInt64 row_id"
            ));
            continue;
        };

        let Some(actual_message_type) = index_rows.get(&row_id) else {
            violations.push(format!(
                "{frame_name} row {row} row_id {row_id} has no matching index row"
            ));
            continue;
        };

        if actual_message_type != expected_message_type {
            violations.push(format!(
                "{frame_name} row {row} row_id {row_id} has index message type {actual_message_type}, expected {expected_message_type}"
            ));
        }
    }
}

fn u64_column<'a>(
    dataframe: &'a DataFrame,
    name: &str,
) -> Option<&'a polars::prelude::UInt64Chunked> {
    dataframe
        .column(name)
        .ok()?
        .as_materialized_series()
        .u64()
        .ok()
}

fn string_value(column: &polars::prelude::Column, row: usize) -> Option<String> {
    let value = column.as_materialized_series().get(row).ok()?;
    if matches!(value, AnyValue::Null) {
        return None;
    }
    Some(value.str_value().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProcessingTimings, VdaAnalysisResult};
    use polars::prelude::{DataFrame, NamedFrom, Series};
    use std::{collections::HashMap, time::Duration};

    fn empty_result() -> VdaAnalysisResult {
        let dataframes = CANONICAL_FRAME_NAMES
            .iter()
            .map(|name| ((*name).to_string(), DataFrame::empty()))
            .collect();
        VdaAnalysisResult {
            dataframes,
            total_chunks: 0,
            num_parsed: 0,
            parse_failures: HashMap::new(),
            parse_examples: HashMap::new(),
            timings: ProcessingTimings {
                mmap_setup: Duration::ZERO,
                delimiter_scanning: Duration::ZERO,
                parsing_and_builder_appends: Duration::ZERO,
                batch_dataframe_construction: Duration::ZERO,
                final_dataframe_concatenation: Duration::ZERO,
            },
        }
    }

    #[test]
    fn valid_empty_canonical_frames_are_accepted() {
        assert!(validate_result(&empty_result()).is_ok());
    }

    #[test]
    fn missing_frames_are_reported_in_canonical_order() {
        let mut result = empty_result();
        result.dataframes.clear();

        let error = validate_result(&result).err();
        assert!(error.is_some());
        let violations = error
            .map(ValidationErrors::into_violations)
            .unwrap_or_default();
        assert_eq!(
            violations,
            vec![
                "missing canonical DataFrame: index",
                "missing canonical DataFrame: state",
                "missing canonical DataFrame: visualization",
                "missing canonical DataFrame: connection",
                "missing canonical DataFrame: order",
                "missing canonical DataFrame: instant_actions",
            ]
        );
    }

    #[test]
    fn malformed_nonempty_schema_is_reported() -> polars::prelude::PolarsResult<()> {
        let mut result = empty_result();
        result.total_chunks = 1;
        result.num_parsed = 1;
        result.dataframes.insert(
            "index".to_string(),
            DataFrame::new_infer_height(vec![Series::new("row_id".into(), [0_i64]).into()])?,
        );

        let error = validate_result(&result).err();
        assert!(
            error
                .as_ref()
                .is_some_and(|error| error.violations().iter().any(|violation| {
                    violation.contains("index.row_id") && violation.contains("expected UInt64")
                }))
        );
        Ok(())
    }

    #[test]
    fn broken_row_links_are_reported() -> polars::prelude::PolarsResult<()> {
        let mut result = empty_result();
        result.total_chunks = 1;
        result.num_parsed = 1;
        result.dataframes.insert(
            "index".to_string(),
            DataFrame::new_infer_height(vec![
                Series::new("row_id".into(), [0_u64]).into(),
                Series::new("manufacturer".into(), ["m1"]).into(),
                Series::new("serial_number".into(), ["r1"]).into(),
                Series::new("msg_type".into(), ["state"]).into(),
                Series::new("header_id".into(), [1_u32]).into(),
                Series::new("timestamp".into(), [1_i64]).into(),
                Series::new("version_packed".into(), [1_u32]).into(),
            ])?,
        );
        result.dataframes.insert(
            "state".to_string(),
            DataFrame::new_infer_height(vec![Series::new("row_id".into(), [9_u64]).into()])?,
        );

        let error = validate_result(&result).err();
        assert!(error.as_ref().is_some_and(|error| {
            error
                .violations()
                .iter()
                .any(|violation| violation.contains("state row 0 row_id 9"))
        }));
        Ok(())
    }

    #[test]
    fn inconsistent_counts_are_reported() {
        let mut result = empty_result();
        result.total_chunks = 1;
        result.num_parsed = 2;

        let error = validate_result(&result).err();
        assert!(error.as_ref().is_some_and(|error| {
            error
                .violations()
                .iter()
                .any(|violation| violation.contains("num_parsed 2 exceeds discovered chunks 1"))
        }));
    }
}
