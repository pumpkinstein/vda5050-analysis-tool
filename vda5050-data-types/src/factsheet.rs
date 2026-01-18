use crate::common::Header;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Factsheet {
    #[serde(flatten)]
    pub header: Header,
    #[serde(rename = "type")]
    pub type_field: String,
    pub type_version: String,
    pub agv_kinematic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_load: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_dimensions: Option<Dimensions>,
    pub agv_dimensions: Dimensions,
    pub actions: Vec<ActionDefinition>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Dimensions {
    pub length: f64,
    pub width: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionDefinition {
    pub action_type: String,
    pub action_description: String,
    pub action_scopes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_description: Option<serde_json::Value>,
}
