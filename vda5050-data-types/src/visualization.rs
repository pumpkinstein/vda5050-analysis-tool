use crate::common::{AgvPosition, Velocity, deserialize_timestamp, serialize_timestamp};
use serde::{Deserialize, Serialize};

/// A message for visualization purposes.
/// This message is not safety-relevant and should not be used for control.
///
/// AGV position and/or velocity for visualization purposes. Can be published at a higher rate if wanted.
/// Since bandwidth may be expensive depending on the update rate for this topic, all fields are optional.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Visualization {
    /// Header ID of the message. The headerId is defined per topic and incremented by 1 with each sent (but not necessarily received) message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_id: Option<u32>,
    /// Timestamp in microseconds since Unix epoch (parsed from ISO 8601 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(
        deserialize_with = "deserialize_optional_timestamp",
        serialize_with = "serialize_optional_timestamp"
    )]
    pub timestamp: Option<i64>,
    /// Version of the protocol [Major].[Minor].[Patch]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Manufacturer of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// Serial number of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    /// The AGV's position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    /// The AGV's velocity in vehicle coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
}

/// Custom deserializer for optional timestamps
fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    opt.map(|s| {
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.timestamp_micros())
            .map_err(serde::de::Error::custom)
    })
    .transpose()
}

/// Custom serializer for optional timestamps
fn serialize_optional_timestamp<S>(
    timestamp: &Option<i64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match timestamp {
        Some(ts) => {
            use chrono::{DateTime, Utc};
            let dt = DateTime::<Utc>::from_timestamp_micros(*ts)
                .ok_or_else(|| serde::ser::Error::custom("Invalid timestamp"))?;
            serializer.serialize_some(&dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        }
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_visualization_minimal() {
        let json = r#"{
            "headerId": 100,
            "timestamp": "2024-05-20T15:00:00Z",
            "version": "2.1.0",
            "manufacturer": "robot-inc",
            "serialNumber": "agv-001"
        }"#;

        let viz: Visualization = serde_json::from_str(json).unwrap();
        assert_eq!(viz.header_id, Some(100));
        assert_eq!(viz.manufacturer, Some("robot-inc".to_string()));
        assert!(viz.agv_position.is_none());
        assert!(viz.velocity.is_none());
    }

    #[test]
    fn test_deserialize_visualization_with_position() {
        let json = r#"{
            "headerId": 101,
            "timestamp": "2024-05-20T15:00:01Z",
            "version": "2.1.0",
            "manufacturer": "robot-inc",
            "serialNumber": "agv-001",
            "agvPosition": {
                "x": 10.5,
                "y": -5.2,
                "theta": 1.57,
                "mapId": "level-1",
                "positionInitialized": true
            }
        }"#;

        let viz: Visualization = serde_json::from_str(json).unwrap();
        assert_eq!(viz.header_id, Some(101));
        assert!(viz.agv_position.is_some());

        let pos = viz.agv_position.unwrap();
        assert_eq!(pos.x, 10.5);
        assert_eq!(pos.y, -5.2);
        assert_eq!(pos.theta, 1.57);
        assert_eq!(pos.map_id, "level-1");
        assert_eq!(pos.position_initialized, true);
    }

    #[test]
    fn test_deserialize_visualization_with_velocity() {
        let json = r#"{
            "headerId": 102,
            "timestamp": "2024-05-20T15:00:02Z",
            "version": "2.1.0",
            "manufacturer": "robot-inc",
            "serialNumber": "agv-001",
            "velocity": {
                "vx": 1.5,
                "vy": 0.0,
                "omega": 0.1
            }
        }"#;

        let viz: Visualization = serde_json::from_str(json).unwrap();
        assert!(viz.velocity.is_some());

        let vel = viz.velocity.unwrap();
        assert_eq!(vel.vx, Some(1.5));
        assert_eq!(vel.vy, Some(0.0));
        assert_eq!(vel.omega, Some(0.1));
    }
}

#[test]
fn test_deserialize_visualization_only_position() {
    // Per VDA5050 spec, all fields are optional - even header fields
    // This tests that a visualization with only position data can be deserialized
    let json = r#"{
            "agvPosition": {
                "x": 5.0,
                "y": 10.0,
                "theta": 0.5,
                "mapId": "warehouse",
                "positionInitialized": true
            }
        }"#;

    let viz: Visualization = serde_json::from_str(json).unwrap();
    assert!(viz.header_id.is_none());
    assert!(viz.timestamp.is_none());
    assert!(viz.version.is_none());
    assert!(viz.manufacturer.is_none());
    assert!(viz.serial_number.is_none());
    assert!(viz.agv_position.is_some());
}

#[test]
fn test_deserialize_visualization_empty() {
    // Per VDA5050 spec, a completely empty visualization message is valid
    let json = r#"{}"#;

    let viz: Visualization = serde_json::from_str(json).unwrap();
    assert!(viz.header_id.is_none());
    assert!(viz.timestamp.is_none());
    assert!(viz.version.is_none());
    assert!(viz.manufacturer.is_none());
    assert!(viz.serial_number.is_none());
    assert!(viz.agv_position.is_none());
    assert!(viz.velocity.is_none());
}

#[test]
fn test_deserialize_visualization_only_velocity() {
    // Test visualization with only velocity data
    let json = r#"{
            "velocity": {
                "vx": 2.0,
                "vy": 1.0,
                "omega": 0.3
            }
        }"#;

    let viz: Visualization = serde_json::from_str(json).unwrap();
    assert!(viz.header_id.is_none());
    assert!(viz.agv_position.is_none());
    assert!(viz.velocity.is_some());
}
