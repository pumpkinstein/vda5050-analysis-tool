use crate::common::{Action, AgvPosition, Header, NodePosition, Point, Trajectory, Velocity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(flatten)]
    pub header: Header,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_update_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_node_sequence_id: Option<u32>,
    pub node_states: Vec<NodeState>,
    pub edge_states: Vec<EdgeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loads: Option<Vec<Load>>,
    pub driving: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_base_request: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_since_last_node: Option<f64>,
    pub operating_mode: String,
    pub errors: Vec<Error>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information: Option<Vec<Info>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_state: Option<BatteryState>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    pub node_id: String,
    pub sequence_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_description: Option<String>,
    pub released: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    pub actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EdgeState {
    pub edge_id: String,
    pub sequence_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_description: Option<String>,
    pub released: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    pub actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Load {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box_reference: Option<BoundingBoxReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<Vec<Point>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBoxReference {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub error_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    pub error_level: String,
    pub error_references: Vec<ErrorReference>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ErrorReference {
    pub reference_key: String,
    pub reference_value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub info_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_description: Option<String>,
    pub info_level: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    pub battery_charge: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<f64>,
}
