use crate::common::{Action, AgvPosition, Header, NodePosition, Point, Trajectory, Velocity};
use serde::{Deserialize, Serialize};

/// The state of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(flatten)]
    pub header: Header,
    /// The ID of the order that is currently being processed.
    /// If no order is being processed, this field is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// The update ID of the order that is currently being processed.
    /// If no order is being processed, this field is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_update_id: Option<u32>,
    /// The ID of the last node that was reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_node_id: Option<String>,
    /// The sequence ID of the last node that was reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_node_sequence_id: Option<u32>,
    /// The state of the nodes in the current order.
    pub node_states: Vec<NodeState>,
    /// The state of the edges in the current order.
    pub edge_states: Vec<EdgeState>,
    /// The position of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    /// The velocity of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
    /// The loads that are currently on the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loads: Option<Vec<Load>>,
    /// Indicates if the AGV is currently driving.
    pub driving: bool,
    /// Indicates if the AGV is currently paused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    /// Indicates if the AGV is requesting a new base.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_base_request: Option<bool>,
    /// The distance since the last node was reached in [m].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_since_last_node: Option<f64>,
    /// The operating mode of the AGV.
    pub operating_mode: OperatingMode,
    /// A list of errors that occurred on the AGV.
    pub errors: Vec<Error>,
    /// A list of information messages from the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information: Option<Vec<Info>>,
    /// The state of the AGV's battery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_state: Option<BatteryState>,
}

/// The operating mode of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OperatingMode {
    /// The AGV is operating in automatic mode.
    #[serde(rename = "AUTOMATIC")]
    Automatic,
    /// The AGV is operating in manual mode.
    #[serde(rename = "MANUAL")]
    Manual,
    /// The AGV is in service mode.
    #[serde(rename = "SERVICE")]
    Service,
}

/// The state of a node.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    /// The ID of the node.
    pub node_id: String,
    /// The sequence ID of the node.
    pub sequence_id: u32,
    /// A description of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_description: Option<String>,
    /// Indicates if the node has been released.
    pub released: bool,
    /// The position of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    /// A list of actions that are being executed at the node.
    pub actions: Vec<Action>,
}

/// The state of an edge.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EdgeState {
    /// The ID of the edge.
    pub edge_id: String,
    /// The sequence ID of the edge.
    pub sequence_id: u32,
    /// A description of the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_description: Option<String>,
    /// Indicates if the edge has been released.
    pub released: bool,
    /// The trajectory of the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    /// A list of actions that are being executed on the edge.
    pub actions: Vec<Action>,
}

/// The state of a load on the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Load {
    /// The ID of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_id: Option<String>,
    /// The type of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_type: Option<String>,
    /// The position of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_position: Option<String>,
    /// The weight of the load in [kg].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// The reference to the bounding box of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box_reference: Option<BoundingBoxReference>,
    /// The bounding box of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<Vec<Point>>,
}

/// A reference to a bounding box.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBoxReference {
    /// The x coordinate of the bounding box reference.
    pub x: f64,
    /// The y coordinate of the bounding box reference.
    pub y: f64,
    /// The z coordinate of the bounding box reference.
    pub z: f64,
    /// The orientation of the bounding box reference in [rad].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
}

/// An error that occurred on the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// The type of the error.
    pub error_type: String,
    /// A description of the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    /// The level of the error.
    pub error_level: ErrorLevel,
    /// A list of references to the error.
    pub error_references: Vec<ErrorReference>,
}

/// The level of the error.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ErrorLevel {
    /// The AGV can continue its operation.
    #[serde(rename = "WARNING")]
    Warning,
    /// The AGV cannot continue its operation.
    #[serde(rename = "FATAL")]
    Fatal,
}

/// A reference to an error.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ErrorReference {
    /// The key of the reference.
    pub reference_key: String,
    /// The value of the reference.
    pub reference_value: String,
}

/// An information message from the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// The type of the information message.
    pub info_type: String,
    /// A description of the information message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_description: Option<String>,
    /// The level of the information message.
    pub info_level: String,
}

/// The state of the AGV's battery.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    /// The charge of the battery in percent. Range: [0.0 ... 100.0].
    pub battery_charge: f64,
    /// The voltage of the battery in [V].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>,
    /// The current of the battery in [A].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current: Option<f64>,
    /// Indicates if the battery is charging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging: Option<bool>,
    /// The remaining range of the AGV with the current battery charge in [m].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<f64>,
}
