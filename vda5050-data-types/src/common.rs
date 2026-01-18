use serde::{Deserialize, Serialize};

/// The header is part of each VDA 5050 message.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// Unique continuous number, can be used to identify lost messages.
    pub header_id: u32,
    /// Timestamp in ISO 8601 format. YYYY-MM-DDTHH:mm:ss.ssZ
    pub timestamp: String,
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
    pub action_id: String,
    /// Type of the action.
    pub action_type: String,
    /// Defines if the action is blocking.
    pub blocking_type: String,
    /// Optional parameters for the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_parameters: Option<serde_json::Value>,
}

/// The position of a node in a map.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodePosition {
    /// X coordinate of the position.
    pub x: f64,
    /// Y coordinate of the position.
    pub y: f64,
    /// Identifier of the map.
    pub map_id: String,
    /// Orientation of the AGV at the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    /// Allowed deviation in XY plane.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_xy: Option<f64>,
    /// Allowed deviation in orientation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_theta: Option<f64>,
}

/// A trajectory for the AGV to follow.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Trajectory {
    /// Header ID of the trajectory.
    pub header_id: u32,
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
    /// X coordinate of the control point.
    pub x: f64,
    /// Y coordinate of the control point.
    pub y: f64,
    /// Weight of the control point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

/// The position of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgvPosition {
    /// X coordinate of the AGV.
    pub x: f64,
    /// Y coordinate of the AGV.
    pub y: f64,
    /// Orientation of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    /// Identifier of the map.
    pub map_id: String,
    /// Indicates if the position is initialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_initialized: Option<bool>,
    /// Score of the localization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localization_score: Option<f64>,
    /// Deviation range of the position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviation_range: Option<f64>,
}

/// The velocity of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Velocity {
    /// Velocity in X direction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vx: Option<f64>,
    /// Velocity in Y direction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vy: Option<f64>,
    /// Angular velocity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omega: Option<f64>,
}

/// A point in 3D space.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    /// X coordinate of the point.
    pub x: f64,
    /// Y coordinate of the point.
    pub y: f64,
    /// Z coordinate of the point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}
