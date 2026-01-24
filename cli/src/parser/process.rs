//! Contains the core parsing logic for VDA 5050 log entries.

use crate::parser::models::ParsedRecord;
use anyhow::Result;
use chrono::DateTime;
use nom::{
    bytes::complete::{tag, take_while},
    combinator::map_res,
    sequence::tuple,
    IResult,
};
use std::str;
use vda5050_data_types::{connection::Connection, state::State, visualization::Visualization};

/// A temporary struct to hold the fields parsed from the MQTT topic.
struct Topic<'a> {
    manufacturer: &'a str,
    serial_number: &'a str,
    msg_type: &'a str,
}

/// Uses `nom` to parse the VDA 5050 topic prefix from a raw log entry slice.
fn parse_topic<'a>(input: &'a [u8]) -> IResult<&'a [u8], Topic<'a>> {
    let not_separator = |c: u8| c != b'/' && c != b' ';

    let (rest, (manufacturer, _, serial_number, _, msg_type, _)) = tuple((
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b" "[..]), // The space separating the topic from the JSON payload.
    ))(input)?;

    let topic = Topic {
        manufacturer,
        serial_number,
        msg_type,
    };

    Ok((rest, topic))
}

/// Parses a SemVer string (e.g., "2.0.1") into a packed u32 for efficient comparison and storage.
fn parse_version(version: &str) -> u32 {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    (major as u32) << 24 | (minor as u32) << 16 | (patch as u32)
}

/// Parses an ISO 8601 timestamp string into a Unix timestamp (microseconds since epoch).
fn parse_timestamp_us(timestamp: &str) -> Result<i64> {
    Ok(DateTime::parse_from_rfc3339(timestamp)?.timestamp_micros())
}

/// Parses a complete log entry slice (`&[u8]`) into a `ParsedRecord`.
pub fn parse_record(input: &[u8]) -> Result<ParsedRecord> {
    // The VdaIterator gives us slices that start with `uagv/v1/`. We strip it before topic parsing.
    let input = input
        .strip_prefix(b"uagv/v1/")
        .ok_or_else(|| anyhow::anyhow!("Missing 'uagv/v1/' prefix"))?;

    let (json_payload, topic) =
        parse_topic(input).map_err(|e| anyhow::anyhow!("Topic parsing failed: {}", e))?;

    // Use an iterator to deserialize only the first JSON object from the stream.
    // This robustly handles trailing characters (e.g., other log lines) in the buffer.
    let mut stream = serde_json::Deserializer::from_slice(json_payload).into_iter::<serde_json::Value>();
    let json_value = match stream.next() {
        Some(Ok(v)) => v,
        Some(Err(e)) => return Err(e.into()),
        None => return Err(anyhow::anyhow!("No JSON object found in payload")),
    };

    let mut record = ParsedRecord {
        manufacturer: topic.manufacturer.to_string(),
        serial_number: topic.serial_number.to_string(),
        msg_type: topic.msg_type.to_string(),
        ..Default::default()
    };

    match topic.msg_type {
        "state" => {
            let state: State = serde_json::from_value(json_value)?;
            record.header_id = state.header.header_id;
            record.timestamp_us = parse_timestamp_us(&state.header.timestamp)?;
            record.version_packed = parse_version(&state.header.version);
            record.operating_mode = Some(format!("{:?}", state.operating_mode).to_uppercase());
            record.battery_charge = state.battery_state.map(|bs| bs.battery_charge);
            record.has_errors = Some(!state.errors.is_empty());
        }
        "visualization" => {
            let viz: Visualization = serde_json::from_value(json_value)?;
            record.header_id = viz.header.header_id;
            record.timestamp_us = parse_timestamp_us(&viz.header.timestamp)?;
            record.version_packed = parse_version(&viz.header.version);
            if let Some(pos) = viz.agv_position {
                record.x = Some(pos.x);
                record.y = Some(pos.y);
                record.theta = pos.theta;
                record.map_id = Some(pos.map_id);
            }
        }
        "connection" => {
            let conn: Connection = serde_json::from_value(json_value)?;
            record.header_id = conn.header.header_id;
            record.timestamp_us = parse_timestamp_us(&conn.header.timestamp)?;
            record.version_packed = parse_version(&conn.header.version);
            record.operating_mode = Some(format!("{:?}", conn.connection_state).to_uppercase());
        }
        _ => return Err(anyhow::anyhow!("Unsupported message type: {}", topic.msg_type)),
    }

    Ok(record)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("2.1.0"), (2 << 24) | (1 << 16) | 0);
        assert_eq!(parse_version("1.0.0"), (1 << 24));
        assert_eq!(parse_version("0.0.0"), 0);
        assert_eq!(parse_version("invalid"), 0);
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = "2024-05-20T14:35:12.123Z";
        let us = parse_timestamp_us(ts).unwrap();
        // Python: int(datetime.fromisoformat('2024-05-20T14:35:12.123Z').timestamp() * 1_000_000)
        assert_eq!(us, 1716215712123000);
    }

    #[test]
    fn test_parse_record_state() {
        let log_entry = br#"uagv/v1/test-mfr/test-sn/state {"headerId":1,"timestamp":"2024-05-20T14:35:12.123Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"test-sn","operatingMode":"AUTOMATIC","driving":false,"errors":[]}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.serial_number, "test-sn");
        assert_eq!(record.msg_type, "state");
        assert_eq!(record.header_id, 1);
        assert_eq!(record.version_packed, 2 << 24);
        assert_eq!(record.operating_mode, Some("AUTOMATIC".to_string()));
        assert_eq!(record.has_errors, Some(false));
        assert!(record.x.is_none());
    }

    #[test]
    fn test_parse_record_visualization() {
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"headerId":2,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","manufacturer":"test-mfr","serialNumber":"test-sn","agvPosition":{"x":1.0,"y":2.5,"mapId":"map1"}}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.header_id, 2);
        assert_eq!(record.version_packed, (2 << 24) | (1 << 16));
        assert_eq!(record.x, Some(1.0));
        assert_eq!(record.y, Some(2.5));
        assert_eq!(record.map_id, Some("map1".to_string()));
        assert!(record.operating_mode.is_none());
    }

    #[test]
    fn test_parse_record_connection() {
        let log_entry = br#"uagv/v1/Jungheinrich/2/connection {"headerId":5,"timestamp":"2025-04-12T06:19:11.012598Z","version":"1.1.0","manufacturer":"Jungheinrich","serialNumber":"2","connectionState":"ONLINE"}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "Jungheinrich");
        assert_eq!(record.serial_number, "2");
        assert_eq!(record.msg_type, "connection");
        assert_eq!(record.header_id, 5);
        assert_eq!(record.version_packed, (1 << 24) | (1 << 16));
        assert_eq!(record.operating_mode, Some("ONLINE".to_string()));
        assert!(record.x.is_none());
        assert!(record.battery_charge.is_none());
        assert!(record.has_errors.is_none());
    }

    #[test]
    fn test_parse_record_connection_broken() {
        let log_entry = br#"uagv/v1/Jungheinrich/2/connection {"headerId":4,"timestamp":"2025-04-12T06:19:07.242319Z","version":"1.1.0","manufacturer":"Jungheinrich","serialNumber":"2","connectionState":"CONNECTIONBROKEN"}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.operating_mode, Some("CONNECTIONBROKEN".to_string()));
    }

    #[test]
    fn test_parse_record_connection_offline() {
        let log_entry = br#"uagv/v1/Test/1/connection {"headerId":1,"timestamp":"2025-04-12T06:19:07.242319Z","version":"1.1.0","manufacturer":"Test","serialNumber":"1","connectionState":"OFFLINE"}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.operating_mode, Some("OFFLINE".to_string()));
    }

    #[test]
    fn test_debug_format_enum() {
        use vda5050_data_types::connection::ConnectionState;
        use vda5050_data_types::state::OperatingMode;

        // Verify Debug format produces expected strings
        assert_eq!(format!("{:?}", ConnectionState::Online).to_uppercase(), "ONLINE");
        assert_eq!(format!("{:?}", ConnectionState::Offline).to_uppercase(), "OFFLINE");
        assert_eq!(format!("{:?}", ConnectionState::ConnectionBroken).to_uppercase(), "CONNECTIONBROKEN");

        assert_eq!(format!("{:?}", OperatingMode::Automatic).to_uppercase(), "AUTOMATIC");
        assert_eq!(format!("{:?}", OperatingMode::Manual).to_uppercase(), "MANUAL");
        assert_eq!(format!("{:?}", OperatingMode::Service).to_uppercase(), "SERVICE");
    }
}
