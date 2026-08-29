use anyhow::Result;
use log_file_parser::{DEFAULT_ROOT_TOPIC, MessageType, process_log_file, validate_result};
use polars::prelude::{DataType, TimeUnit};
use std::{collections::HashSet, path::Path};

fn sample_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/sample.log")
}

#[test]
fn sample_log_has_expected_parser_metadata_and_representative_cells() -> Result<()> {
    let result = process_log_file(&sample_path(), DEFAULT_ROOT_TOPIC, 4_000, true)?;
    validate_result(&result)?;

    assert_eq!(result.total_chunks, 6);
    assert_eq!(result.num_parsed, 4);
    assert_eq!(result.parse_failures.get("state").copied(), Some(1));
    assert_eq!(result.parse_failures.get("visualization").copied(), Some(1));

    let frame_names: HashSet<_> = result.dataframes.keys().map(String::as_str).collect();
    assert_eq!(
        frame_names,
        HashSet::from([
            "index",
            "state",
            "visualization",
            "connection",
            "order",
            "instant_actions",
        ])
    );

    let index = result
        .dataframes
        .get("index")
        .ok_or_else(|| anyhow::anyhow!("missing index frame"))?;
    assert_eq!(index.height(), 4);
    assert_eq!(
        index
            .get_column_names()
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "row_id",
            "manufacturer",
            "serial_number",
            "msg_type",
            "header_id",
            "timestamp",
            "version_packed",
        ]
    );
    assert_eq!(index.column("row_id")?.dtype(), &DataType::UInt64);
    assert_eq!(
        index.column("timestamp")?.dtype(),
        &DataType::Datetime(TimeUnit::Nanoseconds, None)
    );
    assert_eq!(
        index.column("manufacturer")?.get(0)?.str_value(),
        "robot-inc"
    );
    assert_eq!(index.column("msg_type")?.get(0)?.str_value(), "state");
    assert_eq!(
        index
            .column("timestamp")?
            .cast(&DataType::Int64)?
            .i64()?
            .get(0),
        Some(1_716_285_600_123_000_000_i64)
    );

    let state = result
        .dataframes
        .get(MessageType::State.dataframe_name())
        .ok_or_else(|| anyhow::anyhow!("missing state frame"))?;
    assert_eq!(state.height(), 2);
    assert_eq!(
        state.column("operating_mode")?.get(1)?.str_value(),
        "MANUAL"
    );
    assert_eq!(state.column("battery_charge")?.f64()?.get(0), Some(88.5));
    assert_eq!(state.column("has_errors")?.bool()?.get(1), Some(true));

    let visualization = result
        .dataframes
        .get(MessageType::Visualization.dataframe_name())
        .ok_or_else(|| anyhow::anyhow!("missing visualization frame"))?;
    assert_eq!(visualization.height(), 1);
    assert_eq!(visualization.column("x")?.f64()?.get(0), Some(10.9));
    assert_eq!(
        visualization.column("map_id")?.get(0)?.str_value(),
        "level-1"
    );

    let connection = result
        .dataframes
        .get(MessageType::Connection.dataframe_name())
        .ok_or_else(|| anyhow::anyhow!("missing connection frame"))?;
    assert_eq!(connection.height(), 1);
    assert_eq!(
        connection.column("connection_state")?.get(0)?.str_value(),
        "ONLINE"
    );

    Ok(())
}
