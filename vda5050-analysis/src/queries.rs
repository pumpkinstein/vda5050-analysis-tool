use polars::prelude::{DataFrame, DataFrameJoinOps, PolarsResult};

const VISUALIZATION_CONTEXT_COLUMNS: [&str; 7] = [
    "manufacturer",
    "serial_number",
    "timestamp",
    "x",
    "y",
    "theta",
    "map_id",
];

/// Join visualization rows with their canonical index context and return a
/// display-independent sample.
///
/// The inner join is performed on `row_id`, the projection is applied in the
/// order shown by the returned columns, and `limit` is applied after the join
/// and projection. Missing or incompatible columns are returned as
/// [`PolarsResult`] errors. Neither source DataFrame is cloned by this API.
pub fn visualization_context_sample(
    index: &DataFrame,
    visualization: &DataFrame,
    limit: usize,
) -> PolarsResult<DataFrame> {
    let joined = visualization.inner_join(index, ["row_id"], ["row_id"])?;
    joined
        .select(VISUALIZATION_CONTEXT_COLUMNS)
        .map(|projected| projected.head(Some(limit)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::{chunked_array::builder::CategoricalChunkedBuilder, prelude::*};

    fn enum_series(name: &str, values: &[&str]) -> PolarsResult<Series> {
        let categories = FrozenCategories::new([
            "state",
            "visualization",
            "connection",
            "order",
            "instantActions",
        ])?;
        let mapping = categories.mapping().clone();
        let mut builder = CategoricalChunkedBuilder::<Categorical8Type>::new(
            name.into(),
            DataType::Enum(categories, mapping),
        );
        for value in values {
            builder.append_str(value)?;
        }
        Ok(builder.finish().into_series())
    }

    fn enum_index(values: &[&str]) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![enum_series("msg_type", values)?.into()])
    }

    fn index_frame(
        row_ids: &[u64],
        manufacturers: &[&str],
        serial_numbers: &[&str],
        timestamps: &[i64],
    ) -> PolarsResult<DataFrame> {
        let timestamp = Series::new("timestamp".into(), timestamps)
            .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;
        DataFrame::new_infer_height(vec![
            Series::new("row_id".into(), row_ids).into(),
            Series::new("manufacturer".into(), manufacturers).into(),
            Series::new("serial_number".into(), serial_numbers).into(),
            timestamp.into(),
        ])
    }

    fn visualization_frame(
        row_ids: &[u64],
        x: &[f64],
        y: &[f64],
        theta: &[f64],
        map_ids: &[&str],
    ) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            Series::new("row_id".into(), row_ids).into(),
            Series::new("x".into(), x).into(),
            Series::new("y".into(), y).into(),
            Series::new("theta".into(), theta).into(),
            Series::new("map_id".into(), map_ids).into(),
        ])
    }

    fn column_names(dataframe: &DataFrame) -> Vec<&str> {
        dataframe
            .get_column_names()
            .iter()
            .map(|name| name.as_str())
            .collect()
    }

    #[test]
    fn message_distribution_preserves_columns_dtypes_counts_and_sorting() -> PolarsResult<()> {
        let index = enum_index(&["state", "visualization", "state", "connection", "state"])?;
        let distribution = crate::message_type_distribution(&index)?;

        assert_eq!(column_names(&distribution), vec!["msg_type", "count"]);
        assert!(distribution.column("msg_type")?.dtype().is_enum());
        assert_eq!(distribution.column("count")?.dtype(), &DataType::UInt32);

        let counts = distribution.column("count")?.u32()?;
        let count_values: Vec<_> = counts.iter().flatten().collect();
        assert!(count_values.windows(2).all(|window| window[0] >= window[1]));
        assert_eq!(count_values.first(), Some(&3));

        let mut message_types: Vec<_> = (0..distribution.height())
            .map(|index| {
                distribution
                    .column("msg_type")?
                    .get(index)
                    .map(|value| value.str_value().into_owned())
            })
            .collect::<PolarsResult<_>>()?;
        message_types.sort();
        assert_eq!(
            message_types,
            vec![
                "connection".to_string(),
                "state".to_string(),
                "visualization".to_string(),
            ]
        );
        Ok(())
    }

    #[test]
    fn message_distribution_handles_empty_and_missing_columns() -> PolarsResult<()> {
        let empty_index = enum_index(&[])?;
        assert_eq!(crate::message_type_distribution(&empty_index)?.height(), 0);
        assert!(crate::message_type_distribution(&DataFrame::empty()).is_err());
        Ok(())
    }

    #[test]
    fn visualization_query_projects_values_and_limits_after_join() -> PolarsResult<()> {
        let index = index_frame(
            &[10, 11, 12, 13],
            &["m1", "m2", "m3", "m4"],
            &["r1", "r2", "r3", "r4"],
            &[10, 11, 12, 13],
        )?;
        let visualization = visualization_frame(
            &[10, 11, 12, 99],
            &[1.0, 2.0, 3.0, 99.0],
            &[10.0, 20.0, 30.0, 990.0],
            &[0.1, 0.2, 0.3, 9.9],
            &["map-1", "map-2", "map-3", "unmatched"],
        )?;

        let sample = visualization_context_sample(&index, &visualization, 2)?;

        assert_eq!(
            column_names(&sample),
            vec![
                "manufacturer",
                "serial_number",
                "timestamp",
                "x",
                "y",
                "theta",
                "map_id",
            ]
        );
        assert_eq!(sample.height(), 2);
        assert_eq!(sample.column("manufacturer")?.get(0)?.str_value(), "m1");
        assert_eq!(sample.column("serial_number")?.get(0)?.str_value(), "r1");
        assert_eq!(sample.column("x")?.f64()?.get(0), Some(1.0));
        assert_eq!(sample.column("y")?.f64()?.get(0), Some(10.0));
        assert_eq!(sample.column("theta")?.f64()?.get(0), Some(0.1));
        assert_eq!(sample.column("map_id")?.get(0)?.str_value(), "map-1");
        assert_eq!(
            sample.column("timestamp")?.dtype(),
            &DataType::Datetime(TimeUnit::Nanoseconds, None)
        );
        Ok(())
    }

    #[test]
    fn visualization_query_documents_duplicate_and_unmatched_join_rows() -> PolarsResult<()> {
        let index = index_frame(
            &[1, 1, 2],
            &["first", "second", "two"],
            &["r1", "r1b", "r2"],
            &[1, 2, 3],
        )?;
        let visualization = visualization_frame(
            &[99, 2, 1],
            &[99.0, 20.0, 10.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            &["unmatched", "map-2", "map-1"],
        )?;

        let all_matches = visualization_context_sample(&index, &visualization, usize::MAX)?;
        assert_eq!(all_matches.height(), 3);
        let mut all_x: Vec<_> = all_matches.column("x")?.f64()?.iter().flatten().collect();
        all_x.sort_by(f64::total_cmp);
        assert_eq!(all_x, vec![10.0, 10.0, 20.0]);

        let limited = visualization_context_sample(&index, &visualization, 2)?;
        assert_eq!(limited.height(), 2);
        Ok(())
    }

    #[test]
    fn visualization_query_returns_errors_for_missing_columns() -> PolarsResult<()> {
        let missing_index =
            DataFrame::new_infer_height(vec![Series::new("row_id".into(), [1_u64]).into()])?;
        let missing_visualization =
            DataFrame::new_infer_height(vec![Series::new("row_id".into(), [1_u64]).into()])?;

        assert!(visualization_context_sample(&missing_index, &missing_visualization, 3).is_err());
        assert!(visualization_context_sample(&DataFrame::empty(), &DataFrame::empty(), 3).is_err());
        Ok(())
    }
}
