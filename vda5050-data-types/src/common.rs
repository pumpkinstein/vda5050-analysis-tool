use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Custom deserializer for timestamps: parses ISO8601 string to microseconds since epoch.
///
/// This uses a visitor instead of deserializing into `String` so deserializers
/// such as simd-json can provide an ordinary timestamp as a borrowed `&str`.
/// The timestamp is parsed immediately, avoiding one owned string allocation
/// and copy per message while retaining owned and transient-string fallbacks.
pub fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(TimestampVisitor)
}

/// Custom deserializer for optional timestamps.
///
/// It preserves the borrowed-string path used by `deserialize_timestamp` and
/// handles `null` directly instead of allocating an intermediate `Option<String>`.
pub fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_option(OptionalTimestampVisitor)
}

struct TimestampVisitor;

impl<'de> Visitor<'de> for TimestampVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an RFC3339 timestamp string")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse_timestamp(value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse_timestamp(value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse_timestamp(&value).map_err(E::custom)
    }
}

struct OptionalTimestampVisitor;

impl<'de> Visitor<'de> for OptionalTimestampVisitor {
    type Value = Option<i64>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an optional RFC3339 timestamp string")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_timestamp(deserializer).map(Some)
    }
}

/// Parses the common UTC timestamp representation without allocating.
///
/// RFC3339 timestamps that do not match this shape are handled by Chrono so
/// offsets, unusual fractional precision, and all validation semantics remain
/// supported.
fn parse_timestamp(value: &str) -> Result<i64, String> {
    if let Some(timestamp) = parse_common_utc_timestamp(value) {
        return Ok(timestamp);
    }

    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_micros())
        .map_err(|error| error.to_string())
}

/// Fast path for `YYYY-MM-DDTHH:MM:SS[.ffffff]Z`.
///
/// Fractions with up to six digits are sufficient for the microsecond output
/// type. Longer fractions deliberately use Chrono so they retain the parser's
/// established truncation and validation behavior.
#[inline]
fn parse_common_utc_timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let fraction_digits = match bytes.len() {
        20 if bytes[19] == b'Z' => 0,
        length @ 22..=27 if bytes[19] == b'.' && bytes[length - 1] == b'Z' => length - 21,
        _ => return None,
    };

    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }

    let year = parse_two_digits(bytes[0], bytes[1])? * 100 + parse_two_digits(bytes[2], bytes[3])?;
    let month = parse_two_digits(bytes[5], bytes[6])?;
    let day = parse_two_digits(bytes[8], bytes[9])?;
    let hour = parse_two_digits(bytes[11], bytes[12])?;
    let minute = parse_two_digits(bytes[14], bytes[15])?;
    let second = parse_two_digits(bytes[17], bytes[18])?;

    let mut microsecond = 0;
    for index in 0..fraction_digits {
        let digit = bytes[20 + index];
        if !digit.is_ascii_digit() {
            return None;
        }
        microsecond = microsecond * 10 + (digit - b'0') as u32;
    }
    for _ in fraction_digits..6 {
        microsecond *= 10;
    }

    chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)?
        .and_hms_micro_opt(hour as u32, minute as u32, second as u32, microsecond)
        .map(|datetime| datetime.and_utc().timestamp_micros())
}

#[inline]
fn parse_two_digits(first: u8, second: u8) -> Option<u32> {
    if first.is_ascii_digit() && second.is_ascii_digit() {
        Some(((first - b'0') as u32) * 10 + (second - b'0') as u32)
    } else {
        None
    }
}

/// Custom serializer for timestamps: formats microseconds since epoch as ISO8601 string
pub fn serialize_timestamp<S>(timestamp: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use chrono::{DateTime, Utc};
    let dt = DateTime::<Utc>::from_timestamp_micros(*timestamp)
        .ok_or_else(|| serde::ser::Error::custom("Invalid timestamp"))?;
    serializer.serialize_str(&dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

/// The header is part of each VDA 5050 message.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Unique continuous number, can be used to identify lost messages.
    /// Recommended to be generated by the AGV.
    /// On receiving a message, the receiver can check for gaps in the sequence.
    pub header_id: u32,
    /// Timestamp in microseconds since Unix epoch (parsed from ISO 8601 format).
    /// The timestamp should be in UTC.
    /// Stored as i64 for efficiency and parsed during deserialization.
    #[serde(
        deserialize_with = "deserialize_timestamp",
        serialize_with = "serialize_timestamp"
    )]
    pub timestamp: i64,
    /// VDA 5050 version, e.g. "2.0.0".
    pub version: String,
    /// Name of the AGV manufacturer.
    pub manufacturer: String,
    /// Serial number of the AGV.
    pub serial_number: String,
}

/// An action that is to be executed by the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    /// Unique identifier for the action.
    /// The actionId is to be used in the state message to report the progress of an action.
    /// Suggestion: Use UUIDs.
    pub action_id: String,
    /// Type of the action.
    /// Name of action as described in the first column of "Actions and Parameters".
    /// Identifies the function of the action.
    pub action_type: String,
    /// Additional information on the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_description: Option<String>,
    /// Regulates if the action is allowed to be executed during movement and/or parallel to other actions.
    pub blocking_type: BlockingType,
    /// Array of actionParameter-objects for the indicated action e.g. deviceId, loadId, external Triggers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_parameters: Option<Vec<ActionParameter>>,
}

/// A parameter for an action.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionParameter {
    /// The key of the action parameter.
    pub key: String,
    /// The value of the action parameter.
    /// Can be a string, number, boolean, array, or object.
    pub value: serde_json::Value,
}

/// Defines if the action is blocking.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BlockingType {
    /// The AGV can execute the next action in parallel.
    #[serde(rename = "NONE")]
    None,
    /// The AGV can execute the next action in parallel, but it has to wait for the action to finish before it can start the next one.
    #[serde(rename = "SOFT")]
    Soft,
    /// The AGV has to wait for the action to finish before it can start the next one.
    #[serde(rename = "HARD")]
    Hard,
}

/// The position of a node in a map.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodePosition {
    /// X-coordinate described in the world coordinate system. In [m].
    pub x: f64,
    /// Y-coordinate described in the world coordinate system. In [m].
    pub y: f64,
    /// Identifier of the map.
    pub map_id: String,
    /// Orientation of the AGV at the node in [rad].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    /// Allowed deviation in XY plane in [m].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_xy: Option<f64>,
    /// Allowed deviation in orientation in [rad].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_theta: Option<f64>,
}

/// A trajectory for the AGV to follow, based on a NURBS curve.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Trajectory {
    /// Degree of the spline.
    pub degree: u8,
    /// Knot vector of the spline.
    pub knot_vector: Vec<f64>,
    /// Control points of the spline.
    pub control_points: Vec<ControlPoint>,
}

/// A control point of a spline.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControlPoint {
    /// X-coordinate described in the world coordinate system.
    pub x: f64,
    /// Y-coordinate described in the world coordinate system.
    pub y: f64,
    /// Range: [0.0 ... float64.max]
    /// The weight of the control point on the curve.
    /// When not defined, the default will be 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

/// The position of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgvPosition {
    /// X-coordinate of the AGV in the world coordinate system. In [m].
    pub x: f64,
    /// Y-coordinate of the AGV in the world coordinate system. In [m].
    pub y: f64,
    /// Orientation of the AGV in [rad].
    pub theta: f64,
    /// Identifier of the map.
    pub map_id: String,
    /// Additional information on the map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_description: Option<String>,
    /// Indicates if the position is initialized.
    /// True: position is initialized. False: position is not initialized.
    pub position_initialized: bool,
    /// Score of the localization. Range: [0.0 ... 1.0].
    /// Describes the quality of the localization (e.g., for SLAM-AGV).
    /// 0.0: position unknown, 1.0: position known.
    /// Optional for vehicles that cannot estimate their localization score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localization_score: Option<f64>,
    /// Deviation range of the position in meters.
    /// Optional for vehicles that cannot estimate their deviation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviation_range: Option<f64>,
}

/// The velocity of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Velocity {
    /// Velocity in X direction. In [m/s].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vx: Option<f64>,
    /// Velocity in Y direction. In [m/s].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vy: Option<f64>,
    /// Angular velocity. In [rad/s].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omega: Option<f64>,
}

/// A point in 3D space.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    /// X coordinate of the point. In [m].
    pub x: f64,
    /// Y coordinate of the point. In [m].
    pub y: f64,
    /// Z coordinate of the point. In [m].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_utc_forms_match_chrono() {
        for value in [
            "2025-04-12T06:19:23Z",
            "2025-04-12T06:19:23.1Z",
            "2025-04-12T06:19:23.144Z",
            "2025-04-12T06:19:23.144189Z",
        ] {
            let expected = chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .timestamp_micros();

            assert_eq!(parse_common_utc_timestamp(value), Some(expected));
            assert_eq!(parse_timestamp(value).unwrap(), expected);
        }
    }

    #[test]
    fn non_common_rfc3339_forms_use_fallback() {
        for value in [
            "2025-04-12T08:19:23.144189+02:00",
            "2025-04-12T06:19:23.144189123Z",
        ] {
            let expected = chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .timestamp_micros();

            assert_eq!(parse_common_utc_timestamp(value), None);
            assert_eq!(parse_timestamp(value).unwrap(), expected);
        }
    }

    #[test]
    fn invalid_common_forms_are_rejected_without_panicking() {
        for value in [
            "2025-02-29T06:19:23Z",
            "2025-04-12T25:19:23Z",
            "2025-04-12T06:19:23.abcdZ",
            "2025-04-12T06:19:23",
        ] {
            assert!(parse_timestamp(value).is_err(), "invalid value: {value}");
        }
    }
}
