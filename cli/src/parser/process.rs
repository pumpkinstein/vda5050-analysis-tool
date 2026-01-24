//! Contains the core parsing logic for VDA 5050 log entries.

use crate::parser::models::ParsedRecord;
use anyhow::Result;
use chrono::DateTime;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_while},
    combinator::map_res,
};
use std::str;
use vda5050_data_types::{
    connection::Connection, instant_actions::InstantActions, order::Order, state::State,
    visualization::Visualization,
};

/// A temporary struct to hold the fields parsed from the MQTT topic.
struct Topic<'a> {
    manufacturer: &'a str,
    serial_number: &'a str,
    msg_type: &'a str,
}

/// Uses `nom` to parse the VDA 5050 topic prefix from a raw log entry slice.
fn parse_topic<'a>(input: &'a [u8]) -> IResult<&'a [u8], Topic<'a>> {
    let not_separator = |c: u8| c != b'/' && c != b' ';

    let (rest, (manufacturer, _, serial_number, _, msg_type, _)) = (
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b"/"[..]),
        map_res(take_while(not_separator), str::from_utf8),
        tag(&b" "[..]), // The space separating the topic from the JSON payload.
    )
        .parse(input)?;

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
    let mut stream =
        serde_json::Deserializer::from_slice(json_payload).into_iter::<serde_json::Value>();
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
            record.battery_charge = Some(state.battery_state.battery_charge);
            record.has_errors = Some(!state.errors.is_empty());
        }
        "visualization" => {
            let viz: Visualization = serde_json::from_value(json_value)?;

            // Per VDA5050 schema, all fields in visualization are optional.
            // However, for data analysis purposes, we need at least header_id and timestamp.
            // Return an error to skip visualization messages without minimum required metadata.
            let header_id = viz.header_id.ok_or_else(|| {
                anyhow::anyhow!("Visualization missing headerId - skipping incomplete record")
            })?;
            let timestamp = viz.timestamp.ok_or_else(|| {
                anyhow::anyhow!("Visualization missing timestamp - skipping incomplete record")
            })?;
            let version = viz.version.ok_or_else(|| {
                anyhow::anyhow!("Visualization missing version - skipping incomplete record")
            })?;

            record.header_id = header_id;
            record.timestamp_us = parse_timestamp_us(&timestamp)?;
            record.version_packed = parse_version(&version);

            // Override manufacturer and serial_number from the message payload if available
            if let Some(mfr) = viz.manufacturer {
                record.manufacturer = mfr;
            }
            if let Some(sn) = viz.serial_number {
                record.serial_number = sn;
            }

            if let Some(pos) = viz.agv_position {
                record.x = Some(pos.x);
                record.y = Some(pos.y);
                record.theta = Some(pos.theta);
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
        "order" => {
            let order: Order = serde_json::from_value(json_value)?;
            record.header_id = order.header.header_id;
            record.timestamp_us = parse_timestamp_us(&order.header.timestamp)?;
            record.version_packed = parse_version(&order.header.version);

            // Store order-specific information in operating_mode field
            record.operating_mode = Some(format!("ORDER:{}", order.order_id));
        }
        "instantActions" => {
            let instant_actions: InstantActions = serde_json::from_value(json_value)?;
            record.header_id = instant_actions.header.header_id;
            record.timestamp_us = parse_timestamp_us(&instant_actions.header.timestamp)?;
            record.version_packed = parse_version(&instant_actions.header.version);

            // Store number of instant actions in operating_mode field
            record.operating_mode =
                Some(format!("INSTANT_ACTIONS:{}", instant_actions.actions.len()));
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported message type: {}",
                topic.msg_type
            ));
        }
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
        let log_entry = br#"uagv/v1/test-mfr/test-sn/state {"headerId":1,"timestamp":"2024-05-20T14:35:12.123Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"test-sn","orderId":"","orderUpdateId":0,"lastNodeId":"","lastNodeSequenceId":0,"driving":false,"operatingMode":"AUTOMATIC","nodeStates":[],"edgeStates":[],"actionStates":[],"batteryState":{"batteryCharge":88.5,"charging":false},"errors":[],"safetyState":{"eStop":"NONE","fieldViolation":false}}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.serial_number, "test-sn");
        assert_eq!(record.msg_type, "state");
        assert_eq!(record.header_id, 1);
        assert_eq!(record.version_packed, 2 << 24);
        assert_eq!(record.operating_mode, Some("AUTOMATIC".to_string()));
        assert_eq!(record.has_errors, Some(false));
        assert_eq!(record.battery_charge, Some(88.5));
        assert!(record.x.is_none());
    }

    #[test]
    fn test_parse_record_visualization() {
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"headerId":2,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","manufacturer":"test-mfr","serialNumber":"test-sn","agvPosition":{"x":1.0,"y":2.5,"theta":1.57,"mapId":"map1","positionInitialized":true}}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.header_id, 2);
        assert_eq!(record.version_packed, (2 << 24) | (1 << 16));
        assert_eq!(record.x, Some(1.0));
        assert_eq!(record.y, Some(2.5));
        assert_eq!(record.theta, Some(1.57));
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
        assert_eq!(
            format!("{:?}", ConnectionState::Online).to_uppercase(),
            "ONLINE"
        );
        assert_eq!(
            format!("{:?}", ConnectionState::Offline).to_uppercase(),
            "OFFLINE"
        );
        assert_eq!(
            format!("{:?}", ConnectionState::ConnectionBroken).to_uppercase(),
            "CONNECTIONBROKEN"
        );

        assert_eq!(
            format!("{:?}", OperatingMode::Automatic).to_uppercase(),
            "AUTOMATIC"
        );
        assert_eq!(
            format!("{:?}", OperatingMode::Manual).to_uppercase(),
            "MANUAL"
        );
        assert_eq!(
            format!("{:?}", OperatingMode::Service).to_uppercase(),
            "SERVICE"
        );
    }

    #[test]
    fn test_parse_record_visualization_with_payload_metadata() {
        // Test that manufacturer and serial_number are extracted from the visualization payload
        let log_entry = br#"uagv/v1/topic-mfr/topic-sn/visualization {"headerId":3,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","manufacturer":"payload-mfr","serialNumber":"payload-sn","agvPosition":{"x":5.0,"y":10.0,"theta":0.0,"mapId":"map2","positionInitialized":true}}"#;
        let record = parse_record(log_entry).unwrap();

        // Should use manufacturer and serial_number from the message payload, not the topic
        assert_eq!(record.manufacturer, "payload-mfr");
        assert_eq!(record.serial_number, "payload-sn");
        assert_eq!(record.msg_type, "visualization");
        assert_eq!(record.header_id, 3);
        assert_eq!(record.x, Some(5.0));
        assert_eq!(record.y, Some(10.0));
        assert_eq!(record.theta, Some(0.0));
        assert_eq!(record.map_id, Some("map2".to_string()));
    }

    #[test]
    fn test_parse_record_visualization_without_payload_metadata() {
        // Test that topic manufacturer and serial_number are used as fallback
        let log_entry = br#"uagv/v1/topic-mfr/topic-sn/visualization {"headerId":4,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","agvPosition":{"x":7.5,"y":12.5,"theta":3.14,"mapId":"map3","positionInitialized":false}}"#;
        let record = parse_record(log_entry).unwrap();

        // Should use manufacturer and serial_number from the topic when not in payload
        assert_eq!(record.manufacturer, "topic-mfr");
        assert_eq!(record.serial_number, "topic-sn");
        assert_eq!(record.msg_type, "visualization");
        assert_eq!(record.header_id, 4);
    }

    #[test]
    fn test_parse_record_visualization_missing_header_id() {
        // Test that visualization without headerId is rejected (even though spec allows it)
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","agvPosition":{"x":1.0,"y":2.0,"theta":0.0,"mapId":"map1","positionInitialized":true}}"#;
        let result = parse_record(log_entry);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("headerId"));
    }

    #[test]
    fn test_parse_record_visualization_missing_timestamp() {
        // Test that visualization without timestamp is rejected
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"headerId":5,"version":"2.1.0","agvPosition":{"x":1.0,"y":2.0,"theta":0.0,"mapId":"map1","positionInitialized":true}}"#;
        let result = parse_record(log_entry);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timestamp"));
    }

    #[test]
    fn test_parse_record_visualization_missing_version() {
        // Test that visualization without version is rejected
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"headerId":6,"timestamp":"2024-05-20T15:00:00Z","agvPosition":{"x":1.0,"y":2.0,"theta":0.0,"mapId":"map1","positionInitialized":true}}"#;
        let result = parse_record(log_entry);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_parse_record_visualization_only_position() {
        // Test that visualization with only position data (no header fields) is rejected
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"agvPosition":{"x":1.0,"y":2.0,"theta":0.0,"mapId":"map1","positionInitialized":true}}"#;
        let result = parse_record(log_entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_record_order() {
        let log_entry = br#"uagv/v1/test-mfr/agv-001/order {"headerId":10,"timestamp":"2024-05-20T10:00:00Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"agv-001","orderId":"order-123","orderUpdateId":0,"nodes":[{"nodeId":"node1","sequenceId":0,"released":true,"actions":[]}],"edges":[]}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.serial_number, "agv-001");
        assert_eq!(record.msg_type, "order");
        assert_eq!(record.header_id, 10);
        assert_eq!(record.version_packed, 2 << 24);
        assert_eq!(record.operating_mode, Some("ORDER:order-123".to_string()));
    }

    #[test]
    fn test_parse_record_order_with_edges() {
        let log_entry = br#"uagv/v1/robot-corp/robot-5/order {"headerId":20,"timestamp":"2024-05-20T11:00:00Z","version":"2.1.0","manufacturer":"robot-corp","serialNumber":"robot-5","orderId":"order-456","orderUpdateId":2,"nodes":[{"nodeId":"n1","sequenceId":0,"released":true,"actions":[]},{"nodeId":"n2","sequenceId":2,"released":true,"actions":[]}],"edges":[{"edgeId":"e1","sequenceId":1,"released":true,"startNodeId":"n1","endNodeId":"n2","actions":[]}]}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "robot-corp");
        assert_eq!(record.serial_number, "robot-5");
        assert_eq!(record.msg_type, "order");
        assert_eq!(record.header_id, 20);
        assert_eq!(record.operating_mode, Some("ORDER:order-456".to_string()));
    }

    #[test]
    fn test_parse_record_instant_actions_empty() {
        let log_entry = br#"uagv/v1/test-mfr/agv-001/instantActions {"headerId":30,"timestamp":"2024-05-20T12:00:00Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"agv-001","actions":[]}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "test-mfr");
        assert_eq!(record.serial_number, "agv-001");
        assert_eq!(record.msg_type, "instantActions");
        assert_eq!(record.header_id, 30);
        assert_eq!(record.version_packed, 2 << 24);
        assert_eq!(record.operating_mode, Some("INSTANT_ACTIONS:0".to_string()));
    }

    #[test]
    fn test_parse_record_instant_actions_with_actions() {
        let log_entry = br#"uagv/v1/robot-corp/robot-7/instantActions {"headerId":40,"timestamp":"2024-05-20T13:00:00Z","version":"2.1.0","manufacturer":"robot-corp","serialNumber":"robot-7","actions":[{"actionId":"pause-1","actionType":"startPause","blockingType":"HARD"},{"actionId":"pause-2","actionType":"stopPause","blockingType":"HARD"}]}"#;
        let record = parse_record(log_entry).unwrap();
        assert_eq!(record.manufacturer, "robot-corp");
        assert_eq!(record.serial_number, "robot-7");
        assert_eq!(record.msg_type, "instantActions");
        assert_eq!(record.header_id, 40);
        assert_eq!(record.operating_mode, Some("INSTANT_ACTIONS:2".to_string()));
    }
}
