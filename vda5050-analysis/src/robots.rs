use log_file_parser::VdaAnalysisResult;
use polars::prelude::{AnyValue, DataFrame, DataType, UniqueKeepStrategy};
use std::borrow::Cow;

const MANUFACTURER_COLUMN: &str = "manufacturer";
const SERIAL_NUMBER_COLUMN: &str = "serial_number";
const IDENTITY_COLUMNS: [&str; 2] = [MANUFACTURER_COLUMN, SERIAL_NUMBER_COLUMN];

/// A normalized robot identity found in the canonical `index` DataFrame.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RobotIdentity {
    /// The robot manufacturer value from the `index` DataFrame.
    pub manufacturer: String,
    /// The robot serial number value from the `index` DataFrame.
    pub serial_number: String,
}

/// Return the distinct robot identities represented by a parsed analysis result.
///
/// Identities are read from the canonical `index` DataFrame, deduplicated as a
/// `(manufacturer, serial_number)` pair, and returned in alphabetical order by
/// manufacturer and then serial number. Missing frames or columns, nulls,
/// empty values, and unsupported column values are ignored. The query is
/// intentionally infallible so incomplete parser results remain displayable
/// by callers.
pub fn unique_robot_identities(result: &VdaAnalysisResult) -> Vec<RobotIdentity> {
    result
        .dataframes
        .get("index")
        .map(unique_robot_identities_from_index)
        .unwrap_or_default()
}

pub(crate) fn unique_robot_identities_from_index(index: &DataFrame) -> Vec<RobotIdentity> {
    let mut identities = Vec::new();
    for_each_unique_robot_identity(index, |manufacturer, serial_number| {
        identities.push(RobotIdentity {
            manufacturer: manufacturer.to_owned(),
            serial_number: serial_number.to_owned(),
        });
    });
    identities.sort_unstable_by(|left, right| {
        left.manufacturer
            .cmp(&right.manufacturer)
            .then_with(|| left.serial_number.cmp(&right.serial_number))
    });
    identities
}

pub(crate) fn count_unique_robot_identities_from_index(index: &DataFrame) -> usize {
    for_each_unique_robot_identity(index, |_, _| {})
}

fn for_each_unique_robot_identity<F>(index: &DataFrame, mut visit: F) -> usize
where
    F: FnMut(&str, &str),
{
    let Some(index) = unique_identity_rows(index) else {
        return 0;
    };

    let Ok(manufacturer) = index.column(MANUFACTURER_COLUMN) else {
        return 0;
    };
    let Ok(serial_number) = index.column(SERIAL_NUMBER_COLUMN) else {
        return 0;
    };

    let mut count = 0;

    for row in 0..index.height() {
        let Some(manufacturer) = string_identity_value(manufacturer, row) else {
            continue;
        };
        let Some(serial_number) = string_identity_value(serial_number, row) else {
            continue;
        };

        visit(manufacturer.as_ref(), serial_number.as_ref());
        count += 1;
    }

    count
}

fn unique_identity_rows(index: &DataFrame) -> Option<DataFrame> {
    let manufacturer = index.column(MANUFACTURER_COLUMN).ok()?;
    let serial_number = index.column(SERIAL_NUMBER_COLUMN).ok()?;
    if !is_supported_identity_dtype(manufacturer.dtype())
        || !is_supported_identity_dtype(serial_number.dtype())
    {
        return None;
    }

    let identity_columns = index.select(IDENTITY_COLUMNS).ok()?;
    let subset = IDENTITY_COLUMNS.map(String::from);
    identity_columns
        .unique::<(), ()>(Some(&subset), UniqueKeepStrategy::Any, None)
        .ok()
}

fn is_supported_identity_dtype(dtype: &DataType) -> bool {
    matches!(
        dtype,
        DataType::String | DataType::Categorical(_, _) | DataType::Enum(_, _)
    )
}

fn string_identity_value(column: &polars::prelude::Column, row: usize) -> Option<Cow<'_, str>> {
    let value = column.get(row).ok()?;
    if matches!(value, AnyValue::Null) {
        return None;
    }

    let value = value.str_value();
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::{chunked_array::builder::CategoricalChunkedBuilder, prelude::*};
    use std::{collections::HashMap, time::Duration};

    fn result(dataframes: HashMap<String, DataFrame>) -> VdaAnalysisResult {
        VdaAnalysisResult {
            dataframes,
            total_chunks: 0,
            num_parsed: 0,
            parse_failures: HashMap::new(),
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

    fn result_with_index(index: DataFrame) -> VdaAnalysisResult {
        result(HashMap::from([(String::from("index"), index)]))
    }

    fn index_frame(manufacturers: &[&str], serial_numbers: &[&str]) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            Series::new(MANUFACTURER_COLUMN.into(), manufacturers).into(),
            Series::new(SERIAL_NUMBER_COLUMN.into(), serial_numbers).into(),
        ])
    }

    fn categorical_index_frame(
        manufacturers: &[&str],
        serial_numbers: &[&str],
    ) -> PolarsResult<DataFrame> {
        let manufacturer_categories =
            Categories::random("test-manufacturer".into(), CategoricalPhysical::U32);
        let manufacturer_mapping = manufacturer_categories.mapping();
        let mut manufacturer_builder = CategoricalChunkedBuilder::<Categorical32Type>::new(
            MANUFACTURER_COLUMN.into(),
            DataType::Categorical(manufacturer_categories, manufacturer_mapping),
        );
        for manufacturer in manufacturers {
            manufacturer_builder.append_str(manufacturer)?;
        }

        let serial_categories =
            Categories::random("test-serial-number".into(), CategoricalPhysical::U32);
        let serial_mapping = serial_categories.mapping();
        let mut serial_builder = CategoricalChunkedBuilder::<Categorical32Type>::new(
            SERIAL_NUMBER_COLUMN.into(),
            DataType::Categorical(serial_categories, serial_mapping),
        );
        for serial_number in serial_numbers {
            serial_builder.append_str(serial_number)?;
        }

        DataFrame::new_infer_height(vec![
            manufacturer_builder.finish().into_series().into(),
            serial_builder.finish().into_series().into(),
        ])
    }

    #[test]
    fn empty_parser_result_returns_no_identities() {
        assert!(unique_robot_identities(&result(HashMap::new())).is_empty());
    }

    #[test]
    fn missing_index_or_identity_columns_returns_no_identities() -> PolarsResult<()> {
        let missing_columns = DataFrame::new_infer_height(vec![
            Series::new(MANUFACTURER_COLUMN.into(), ["m1"]).into(),
        ])?;

        assert!(unique_robot_identities(&result(HashMap::new())).is_empty());
        assert!(unique_robot_identities(&result_with_index(missing_columns)).is_empty());
        Ok(())
    }

    #[test]
    fn repeated_pairs_are_deduplicated_in_alphabetical_order() -> PolarsResult<()> {
        let index = index_frame(
            &["m2", "m1", "m2", "m1", "m1", "m2"],
            &["r2", "r3", "r1", "r3", "r1", "r1"],
        )?;

        assert_eq!(
            unique_robot_identities(&result_with_index(index)),
            vec![
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r1".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r3".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m2".to_string(),
                    serial_number: "r1".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m2".to_string(),
                    serial_number: "r2".to_string(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn pairs_sharing_one_field_remain_distinct() -> PolarsResult<()> {
        let index = index_frame(&["m2", "m1", "m1"], &["r1", "r2", "r1"])?;

        assert_eq!(
            unique_robot_identities(&result_with_index(index)),
            vec![
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r1".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r2".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m2".to_string(),
                    serial_number: "r1".to_string(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn categorical_identity_columns_are_supported() -> PolarsResult<()> {
        let index = categorical_index_frame(&["m2", "m1", "m2", "m1"], &["r2", "r1", "r2", "r2"])?;

        assert_eq!(
            unique_robot_identities(&result_with_index(index)),
            vec![
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r1".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r2".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m2".to_string(),
                    serial_number: "r2".to_string(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn null_empty_and_unsupported_identity_values_are_filtered() -> PolarsResult<()> {
        let index = DataFrame::new_infer_height(vec![
            Series::new(
                MANUFACTURER_COLUMN.into(),
                &[
                    Some("m4"),
                    Some("m3"),
                    Some(" "),
                    None,
                    Some("m2"),
                    Some(""),
                    Some("m1"),
                    Some("m0"),
                ],
            )
            .into(),
            Series::new(
                SERIAL_NUMBER_COLUMN.into(),
                &[
                    Some("r1"),
                    Some("r2"),
                    Some("r3"),
                    Some("r4"),
                    Some(""),
                    Some("r5"),
                    Some("r0"),
                    None,
                ],
            )
            .into(),
        ])?;

        assert_eq!(
            unique_robot_identities(&result_with_index(index)),
            vec![
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r0".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m3".to_string(),
                    serial_number: "r2".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m4".to_string(),
                    serial_number: "r1".to_string(),
                },
            ]
        );

        let numeric_columns = DataFrame::new_infer_height(vec![
            Series::new(MANUFACTURER_COLUMN.into(), [1, 2]).into(),
            Series::new(SERIAL_NUMBER_COLUMN.into(), ["r1", "r2"]).into(),
        ])?;
        assert!(unique_robot_identities(&result_with_index(numeric_columns)).is_empty());
        Ok(())
    }

    #[test]
    fn nonblank_identity_spelling_is_preserved() -> PolarsResult<()> {
        let index = index_frame(&[" m2 ", "m1"], &[" r2 ", "r1"])?;

        assert_eq!(
            unique_robot_identities(&result_with_index(index)),
            vec![
                RobotIdentity {
                    manufacturer: " m2 ".to_string(),
                    serial_number: " r2 ".to_string(),
                },
                RobotIdentity {
                    manufacturer: "m1".to_string(),
                    serial_number: "r1".to_string(),
                },
            ]
        );
        Ok(())
    }
}
