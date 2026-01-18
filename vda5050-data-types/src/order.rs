use crate::common::{Action, Header, NodePosition, Trajectory};
use serde::{Deserialize, Serialize};

/// An order to be processed by the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(flatten)]
    pub header: Header,
    /// Unique identifier for the order.
    pub order_id: String,
    /// Unique identifier for the order update.
    pub order_update_id: u32,
    /// List of nodes that make up the order.
    pub nodes: Vec<Node>,
    /// List of edges that connect the nodes.
    pub edges: Vec<Edge>,
}

/// A node in the order.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Unique identifier for the node.
    pub node_id: String,
    /// Sequence number of the node.
    pub sequence_id: u32,
    /// Indicates if the node is released.
    pub released: bool,
    /// The position of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    /// List of actions to be performed at the node.
    pub actions: Vec<Action>,
}

/// An edge that connects two nodes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Unique identifier for the edge.
    pub edge_id: String,
    /// Sequence number of the edge.
    pub sequence_id: u32,
    /// Indicates if the edge is released.
    pub released: bool,
    /// The start node of the edge.
    pub start_node_id: String,
    /// The end node of the edge.
    pub end_node_id: String,
    /// List of actions to be performed on the edge.
    pub actions: Vec<Action>,
    /// The trajectory to be followed on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    /// The length of the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    /// The maximum speed on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_speed: Option<f64>,
    /// The maximum height on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<f64>,
    /// The minimum height on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height: Option<f64>,
    /// The orientation of the AGV on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
    /// The direction of the AGV on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

