use crate::common::Header;
use serde::{Deserialize, Serialize};

/// A factsheet that contains information about the AGV.
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
    pub agv_kinematic: String,
    /// The maximum load of the AGV.
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

/// The dimensions of an object.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dimensions {
    /// The length of the object.
    pub length: f64,
    /// The width of the object.
    pub width: f64,
    /// The height of the object.
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
    pub action_scopes: Vec<String>,
    /// A description of the result of the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_description: Option<serde_json::Value>,
}

