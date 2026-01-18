use crate::common::{Action, Header, NodePosition, Trajectory};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(flatten)]
    pub header: Header,
    pub order_id: String,
    pub order_update_id: u32,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub node_id: String,
    pub sequence_id: u32,
    pub released: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    pub actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub edge_id: String,
    pub sequence_id: u32,
    pub released: bool,
    pub start_node_id: String,
    pub end_node_id: String,
    pub actions: Vec<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}
