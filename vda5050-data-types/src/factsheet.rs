use crate::common::Header;
use serde::{Deserialize, Serialize};

/// A factsheet that contains information about the AGV.
///
/// The factsheet is sent by the AGV to the master control to provide information about its capabilities.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Factsheet {
    #[serde(flatten)]
    pub header: Header,
    /// The type of the AGV.
    #[serde(rename = "type")]
    pub type_field: String,
    /// The version of the AGV type.
    pub type_version: String,
    /// The kinematic of the AGV.
    pub agv_kinematic: AgvKinematic,
    /// The maximum load of the AGV in [kg].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_load: Option<f64>,
    /// The dimensions of the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_dimensions: Option<Dimensions>,
    /// The dimensions of the AGV.
    pub agv_dimensions: Dimensions,
    /// A list of actions that the AGV can perform.
    pub actions: Vec<ActionDefinition>,
}

/// The kinematic of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AgvKinematic {
    /// A differential drive.
    #[serde(rename = "DIFFERENTIAL")]
    Differential,
    /// A tricycle drive.
    #[serde(rename = "TRICYCLE")]
    Tricycle,
    /// An omnidirectional drive.
    #[serde(rename = "OMNIDRIVE")]
    Omnidrive,
}

/// The dimensions of an object.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dimensions {
    /// The length of the object in [m].
    pub length: f64,
    /// The width of the object in [m].
    pub width: f64,
    /// The height of the object in [m].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// The definition of an action.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionDefinition {
    /// The type of the action.
    pub action_type: String,
    /// A description of the action.
    pub action_description: String,
    /// The scopes of the action.
    pub action_scopes: Vec<ActionScope>,
    /// A description of the result of the action as a JSON schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_description: Option<serde_json::Value>,
}

/// The scope of an action.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionScope {
    /// The action can be executed at a node.
    #[serde(rename = "NODE")]
    Node,
    /// The action can be executed on an edge.
    #[serde(rename = "EDGE")]
    Edge,
    /// The action can be executed as an instant action.
    #[serde(rename = "INSTANT")]
    Instant,
}
