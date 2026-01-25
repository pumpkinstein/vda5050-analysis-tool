//! Contains the core parsing logic for VDA 5050 log entries.

use anyhow::Result;
use log_file_parser::models::{ParsedMessage, TopicMetadata};
use log_file_parser::mqtt_log_io::parse_topic;
use vda5050_data_types::{
    connection::Connection, instant_actions::InstantActions, order::Order, state::State,
    visualization::Visualization,
};

/// Parses a complete log entry slice (`&[u8]`) into a `ParsedMessage`.
pub fn parse_record(input: &[u8]) -> Result<ParsedMessage> {
    // The VdaIterator gives us slices that start with `uagv/v1/`. We strip it before topic parsing.
    let input = input
        .strip_prefix(b"uagv/v1/")
        .ok_or_else(|| anyhow::anyhow!("Missing 'uagv/v1/' prefix"))?;

    let (json_payload, topic) =
        parse_topic(input).map_err(|e| anyhow::anyhow!("Topic parsing failed: {}", e))?;

    let topic_meta = TopicMetadata {
        manufacturer: topic.manufacturer.to_string(),
        serial_number: topic.serial_number.to_string(),
    };

    // Use simd-json for faster parsing. It requires a mutable slice, so we make a copy.
    let mut json_bytes = json_payload.to_vec();

    let message = match topic.msg_type {
        "state" => {
            let state: State = simd_json::serde::from_slice(&mut json_bytes)?;
            ParsedMessage::State {
                topic: topic_meta,
                data: state,
            }
        }
        "visualization" => {
            let viz: Visualization = simd_json::serde::from_slice(&mut json_bytes)?;

            // Per VDA5050 schema, all fields in visualization are optional.
            // However, for data analysis purposes, we need at least header_id and timestamp.
            // Return an error to skip visualization messages without minimum required metadata.
            viz.header_id.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Visualization missing headerId - skipping incomplete record")
            })?;
            viz.timestamp.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Visualization missing timestamp - skipping incomplete record")
            })?;
            viz.version.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Visualization missing version - skipping incomplete record")
            })?;

            ParsedMessage::Visualization {
                topic: topic_meta,
                data: viz,
            }
        }
        "connection" => {
            let conn: Connection = simd_json::serde::from_slice(&mut json_bytes)?;
            ParsedMessage::Connection {
                topic: topic_meta,
                data: conn,
            }
        }
        "order" => {
            let order: Order = simd_json::serde::from_slice(&mut json_bytes)?;
            ParsedMessage::Order {
                topic: topic_meta,
                data: order,
            }
        }
        "instantActions" => {
            let instant_actions: InstantActions = simd_json::serde::from_slice(&mut json_bytes)?;
            ParsedMessage::InstantActions {
                topic: topic_meta,
                data: instant_actions,
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported message type: {}",
                topic.msg_type
            ));
        }
    };

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_record_state() {
        let log_entry = br#"uagv/v1/test-mfr/test-sn/state {"headerId":1,"timestamp":"2024-05-20T14:35:12.123Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"test-sn","orderId":"","orderUpdateId":0,"lastNodeId":"","lastNodeSequenceId":0,"driving":false,"operatingMode":"AUTOMATIC","nodeStates":[],"edgeStates":[],"actionStates":[],"batteryState":{"batteryCharge":88.5,"charging":false},"errors":[],"safetyState":{"eStop":"NONE","fieldViolation":false}}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::State { topic, data } => {
                assert_eq!(topic.manufacturer, "test-mfr");
                assert_eq!(topic.serial_number, "test-sn");
                assert_eq!(data.header.header_id, 1);
                assert_eq!(data.battery_state.battery_charge, 88.5);
                assert!(data.errors.is_empty());
            }
            _ => panic!("Expected State message"),
        }
    }

    #[test]
    fn test_parse_record_visualization() {
        let log_entry = br#"uagv/v1/test-mfr/test-sn/visualization {"headerId":2,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","manufacturer":"test-mfr","serialNumber":"test-sn","agvPosition":{"x":1.0,"y":2.5,"theta":1.57,"mapId":"map1","positionInitialized":true}}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Visualization { topic, data } => {
                assert_eq!(topic.manufacturer, "test-mfr");
                assert_eq!(topic.serial_number, "test-sn");
                assert_eq!(data.header_id, Some(2));

                let pos = data.agv_position.as_ref().unwrap();
                assert_eq!(pos.x, 1.0);
                assert_eq!(pos.y, 2.5);
                assert_eq!(pos.theta, 1.57);
                assert_eq!(pos.map_id, "map1");
            }
            _ => panic!("Expected Visualization message"),
        }
    }

    #[test]
    fn test_parse_record_connection() {
        let log_entry = br#"uagv/v1/Jungheinrich/2/connection {"headerId":5,"timestamp":"2025-04-12T06:19:11.012598Z","version":"1.1.0","manufacturer":"Jungheinrich","serialNumber":"2","connectionState":"ONLINE"}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Connection { topic, data } => {
                assert_eq!(topic.manufacturer, "Jungheinrich");
                assert_eq!(topic.serial_number, "2");
                assert_eq!(data.header.header_id, 5);
            }
            _ => panic!("Expected Connection message"),
        }
    }

    #[test]
    fn test_parse_record_connection_broken() {
        let log_entry = br#"uagv/v1/Jungheinrich/2/connection {"headerId":4,"timestamp":"2025-04-12T06:19:07.242319Z","version":"1.1.0","manufacturer":"Jungheinrich","serialNumber":"2","connectionState":"CONNECTIONBROKEN"}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Connection { data, .. } => {
                assert_eq!(
                    format!("{:?}", data.connection_state).to_uppercase(),
                    "CONNECTIONBROKEN"
                );
            }
            _ => panic!("Expected Connection message"),
        }
    }

    #[test]
    fn test_parse_record_connection_offline() {
        let log_entry = br#"uagv/v1/Test/1/connection {"headerId":1,"timestamp":"2025-04-12T06:19:07.242319Z","version":"1.1.0","manufacturer":"Test","serialNumber":"1","connectionState":"OFFLINE"}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Connection { data, .. } => {
                assert_eq!(
                    format!("{:?}", data.connection_state).to_uppercase(),
                    "OFFLINE"
                );
            }
            _ => panic!("Expected Connection message"),
        }
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
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Visualization { topic, data } => {
                // Topic still contains the topic-level metadata
                assert_eq!(topic.manufacturer, "topic-mfr");
                assert_eq!(topic.serial_number, "topic-sn");

                // But payload contains the payload-level metadata
                assert_eq!(data.manufacturer, Some("payload-mfr".to_string()));
                assert_eq!(data.serial_number, Some("payload-sn".to_string()));

                let pos = data.agv_position.as_ref().unwrap();
                assert_eq!(pos.x, 5.0);
                assert_eq!(pos.y, 10.0);
            }
            _ => panic!("Expected Visualization message"),
        }
    }

    #[test]
    fn test_parse_record_visualization_without_payload_metadata() {
        // Test that topic manufacturer and serial_number are used as fallback
        let log_entry = br#"uagv/v1/topic-mfr/topic-sn/visualization {"headerId":4,"timestamp":"2024-05-20T15:00:00Z","version":"2.1.0","agvPosition":{"x":7.5,"y":12.5,"theta":3.14,"mapId":"map3","positionInitialized":false}}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Visualization { topic, data } => {
                assert_eq!(topic.manufacturer, "topic-mfr");
                assert_eq!(topic.serial_number, "topic-sn");
                assert!(data.manufacturer.is_none());
                assert!(data.serial_number.is_none());
            }
            _ => panic!("Expected Visualization message"),
        }
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
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Order { topic, data } => {
                assert_eq!(topic.manufacturer, "test-mfr");
                assert_eq!(topic.serial_number, "agv-001");
                assert_eq!(data.header.header_id, 10);
                assert_eq!(data.order_id, "order-123");
            }
            _ => panic!("Expected Order message"),
        }
    }

    #[test]
    fn test_parse_record_order_with_edges() {
        let log_entry = br#"uagv/v1/robot-corp/robot-5/order {"headerId":20,"timestamp":"2024-05-20T11:00:00Z","version":"2.1.0","manufacturer":"robot-corp","serialNumber":"robot-5","orderId":"order-456","orderUpdateId":2,"nodes":[{"nodeId":"n1","sequenceId":0,"released":true,"actions":[]},{"nodeId":"n2","sequenceId":2,"released":true,"actions":[]}],"edges":[{"edgeId":"e1","sequenceId":1,"released":true,"startNodeId":"n1","endNodeId":"n2","actions":[]}]}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::Order { topic, data } => {
                assert_eq!(topic.manufacturer, "robot-corp");
                assert_eq!(topic.serial_number, "robot-5");
                assert_eq!(data.header.header_id, 20);
                assert_eq!(data.order_id, "order-456");
                assert_eq!(data.nodes.len(), 2);
                assert_eq!(data.edges.len(), 1);
            }
            _ => panic!("Expected Order message"),
        }
    }

    #[test]
    fn test_parse_record_instant_actions_empty() {
        let log_entry = br#"uagv/v1/test-mfr/agv-001/instantActions {"headerId":30,"timestamp":"2024-05-20T12:00:00Z","version":"2.0.0","manufacturer":"test-mfr","serialNumber":"agv-001","actions":[]}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::InstantActions { topic, data } => {
                assert_eq!(topic.manufacturer, "test-mfr");
                assert_eq!(topic.serial_number, "agv-001");
                assert_eq!(data.header.header_id, 30);
                assert_eq!(data.actions.len(), 0);
            }
            _ => panic!("Expected InstantActions message"),
        }
    }

    #[test]
    fn test_parse_record_instant_actions_with_actions() {
        let log_entry = br#"uagv/v1/robot-corp/robot-7/instantActions {"headerId":40,"timestamp":"2024-05-20T13:00:00Z","version":"2.1.0","manufacturer":"robot-corp","serialNumber":"robot-7","actions":[{"actionId":"pause-1","actionType":"startPause","blockingType":"HARD"},{"actionId":"pause-2","actionType":"stopPause","blockingType":"HARD"}]}"#;
        let message = parse_record(log_entry).unwrap();

        match message {
            ParsedMessage::InstantActions { topic, data } => {
                assert_eq!(topic.manufacturer, "robot-corp");
                assert_eq!(topic.serial_number, "robot-7");
                assert_eq!(data.header.header_id, 40);
                assert_eq!(data.actions.len(), 2);
            }
            _ => panic!("Expected InstantActions message"),
        }
    }
}
