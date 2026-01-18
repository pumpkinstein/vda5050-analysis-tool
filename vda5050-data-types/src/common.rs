use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub header_id: u32,
    pub timestamp: String,
    pub version: String,
    pub manufacturer: String,
    pub serial_number: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub action_id: String,
    pub action_type: String,
    pub blocking_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_parameters: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
    pub map_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_xy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_deviation_theta: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Trajectory {
    pub header_id: u32,
    pub degree: u8,
    pub knot_vector: Vec<f64>,
    pub control_points: Vec<ControlPoint>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControlPoint {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgvPosition {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
    pub map_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_initialized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localization_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviation_range: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Velocity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vx: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omega: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}
