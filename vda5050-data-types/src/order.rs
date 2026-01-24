use crate::common::{Action, Header, NodePosition, Trajectory};
use serde::{Deserialize, Serialize};

/// An order to be processed by the AGV.
///
/// The order is a list of nodes and edges that the AGV has to traverse.
/// The AGV is expected to traverse the nodes and edges in the order of their sequenceId.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(flatten)]
    pub header: Header,
    /// Unique identifier for the order.
    /// This is to be used to identify multiple order messages that belong to the same order.
    pub order_id: String,
    /// Order update identification. Is unique per orderId.
    /// If an order update is rejected, this field is to be passed in the rejection message.
    pub order_update_id: u32,
    /// Unique identifier of the zone set that the AGV has to use for navigation
    /// or that was used by MC for planning.
    /// Optional: Some MC systems do not use zones. Some AGVs do not understand zones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_set_id: Option<String>,
    /// Array of nodes objects to be traversed for fulfilling the order.
    /// One node is enough for a valid order. Leave edge list empty for that case.
    pub nodes: Vec<Node>,
    /// Array of edge objects to be traversed for fulfilling the order.
    /// Directional connection between two nodes.
    pub edges: Vec<Edge>,
}

/// A node in the order graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Unique identifier for the node.
    pub node_id: String,
    /// Sequence number of the node.
    /// Number to track the sequence of nodes and edges in an order and to simplify order updates.
    /// The main purpose is to distinguish between a node which is passed more than once within one orderId.
    pub sequence_id: u32,
    /// Additional information on the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_description: Option<String>,
    /// Indicates if the node is released.
    /// True indicates that the node is part of the base.
    /// False indicates that the node is part of the horizon.
    pub released: bool,
    /// The position of the node.
    /// Optional for vehicle-types that do not require the node position (e.g., line-guided vehicles).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    /// Array of actions to be executed on a node.
    /// Empty array, if no actions required.
    pub actions: Vec<Action>,
}

/// An edge that connects two nodes in the order graph.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Unique identifier for the edge.
    pub edge_id: String,
    /// Sequence number of the edge.
    /// Number to track the sequence of nodes and edges in an order and to simplify order updates.
    pub sequence_id: u32,
    /// Additional information on the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_description: Option<String>,
    /// Indicates if the edge is released.
    /// True indicates that the edge is part of the base.
    /// False indicates that the edge is part of the horizon.
    pub released: bool,
    /// The nodeId of the start node.
    pub start_node_id: String,
    /// The nodeId of the end node.
    pub end_node_id: String,
    /// Permitted maximum speed on the edge in m/s.
    /// Speed is defined by the fastest measurement of the vehicle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_speed: Option<f64>,
    /// Permitted maximum height of the vehicle, including the load, on edge in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<f64>,
    /// Permitted minimal height of the load handling device on the edge in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_height: Option<f64>,
    /// Orientation of the AGV on the edge in [rad].
    /// The value orientationType defines if it has to be interpreted relative to the
    /// global project specific map coordinate system or tangential to the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
    /// Defines how orientation is interpreted.
    /// GLOBAL: relative to the global project specific map coordinate system.
    /// TANGENTIAL: tangential to the edge.
    /// If not defined, the default value is TANGENTIAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_type: Option<OrientationType>,
    /// Sets direction at junctions for line-guided or wire-guided vehicles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Indicates if rotation is allowed on the edge.
    /// True: rotation is allowed on the edge.
    /// False: rotation is not allowed on the edge.
    /// Optional: No limit, if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_allowed: Option<bool>,
    /// Maximum rotation speed in rad/s.
    /// Optional: No limit, if not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rotation_speed: Option<f64>,
    /// Distance of the path from startNode to endNode in meters.
    /// Optional: This value is used by line-guided AGVs to decrease their speed before reaching a stop position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    /// Trajectory JSON-object for this edge as a NURBS.
    /// Defines the curve, on which the AGV should move between startNode and endNode.
    /// Optional: Can be omitted, if AGV cannot process trajectories or if AGV plans its own trajectory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    /// Definition of boundaries in which a vehicle can deviate from its trajectory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corridor: Option<Corridor>,
    /// Array of action objects with detailed information.
    pub actions: Vec<Action>,
}

/// Orientation type for edge orientation interpretation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OrientationType {
    /// Relative to the global project specific map coordinate system.
    #[serde(rename = "GLOBAL")]
    Global,
    /// Tangential to the edge.
    #[serde(rename = "TANGENTIAL")]
    Tangential,
}

/// Definition of boundaries in which a vehicle can deviate from its trajectory.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Corridor {
    /// Defines the width of the corridor in meters to the left related to the trajectory of the vehicle.
    pub left_width: f64,
    /// Defines the width of the corridor in meters to the right related to the trajectory of the vehicle.
    pub right_width: f64,
    /// Defines whether the boundaries are valid for the kinematic center or the contour of the vehicle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corridor_ref_point: Option<CorridorRefPoint>,
}

/// Reference point for corridor boundaries.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CorridorRefPoint {
    /// Boundaries are valid for the kinematic center of the vehicle.
    #[serde(rename = "KINEMATICCENTER")]
    KinematicCenter,
    /// Boundaries are valid for the contour of the vehicle.
    #[serde(rename = "CONTOUR")]
    Contour,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_order_minimal() {
        let json = r#"{
            "headerId": 1,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-123",
            "orderUpdateId": 0,
            "nodes": [
                {
                    "nodeId": "node1",
                    "sequenceId": 0,
                    "released": true,
                    "actions": []
                }
            ],
            "edges": []
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.order_id, "order-123");
        assert_eq!(order.order_update_id, 0);
        assert_eq!(order.nodes.len(), 1);
        assert_eq!(order.edges.len(), 0);
        assert_eq!(order.nodes[0].node_id, "node1");
    }

    #[test]
    fn test_deserialize_order_with_edge() {
        let json = r#"{
            "headerId": 2,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-456",
            "orderUpdateId": 1,
            "nodes": [
                {
                    "nodeId": "node1",
                    "sequenceId": 0,
                    "released": true,
                    "actions": []
                },
                {
                    "nodeId": "node2",
                    "sequenceId": 2,
                    "released": true,
                    "actions": []
                }
            ],
            "edges": [
                {
                    "edgeId": "edge1",
                    "sequenceId": 1,
                    "released": true,
                    "startNodeId": "node1",
                    "endNodeId": "node2",
                    "maxSpeed": 2.5,
                    "actions": []
                }
            ]
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.order_id, "order-456");
        assert_eq!(order.nodes.len(), 2);
        assert_eq!(order.edges.len(), 1);
        assert_eq!(order.edges[0].edge_id, "edge1");
        assert_eq!(order.edges[0].start_node_id, "node1");
        assert_eq!(order.edges[0].end_node_id, "node2");
        assert_eq!(order.edges[0].max_speed, Some(2.5));
    }

    #[test]
    fn test_deserialize_order_with_orientation_type() {
        let json = r#"{
            "headerId": 3,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-789",
            "orderUpdateId": 0,
            "nodes": [
                {
                    "nodeId": "node1",
                    "sequenceId": 0,
                    "released": true,
                    "actions": []
                },
                {
                    "nodeId": "node2",
                    "sequenceId": 2,
                    "released": true,
                    "actions": []
                }
            ],
            "edges": [
                {
                    "edgeId": "edge1",
                    "sequenceId": 1,
                    "released": true,
                    "startNodeId": "node1",
                    "endNodeId": "node2",
                    "orientation": 1.57,
                    "orientationType": "GLOBAL",
                    "actions": []
                }
            ]
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.edges[0].orientation, Some(1.57));
        assert!(matches!(
            order.edges[0].orientation_type,
            Some(OrientationType::Global)
        ));
    }

    #[test]
    fn test_deserialize_order_with_corridor() {
        let json = r#"{
            "headerId": 4,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-999",
            "orderUpdateId": 0,
            "nodes": [
                {
                    "nodeId": "node1",
                    "sequenceId": 0,
                    "released": true,
                    "actions": []
                }
            ],
            "edges": [
                {
                    "edgeId": "edge1",
                    "sequenceId": 1,
                    "released": true,
                    "startNodeId": "node1",
                    "endNodeId": "node2",
                    "corridor": {
                        "leftWidth": 0.5,
                        "rightWidth": 0.5,
                        "corridorRefPoint": "KINEMATICCENTER"
                    },
                    "actions": []
                }
            ]
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        let corridor = order.edges[0].corridor.as_ref().unwrap();
        assert_eq!(corridor.left_width, 0.5);
        assert_eq!(corridor.right_width, 0.5);
        assert!(matches!(
            corridor.corridor_ref_point,
            Some(CorridorRefPoint::KinematicCenter)
        ));
    }

    #[test]
    fn test_deserialize_order_fields_at_end() {
        // Test that Order can parse JSON with header fields at the end (common in real logs)
        let json = r#"{
            "nodes": [
                {
                    "nodeId": "10854",
                    "sequenceId": 0,
                    "released": true,
                    "nodePosition": {
                        "x": 152.595,
                        "y": 163.65,
                        "theta": 0,
                        "allowedDeviationXY": null,
                        "mapId": "1",
                        "mapDescription": null
                    },
                    "actions": []
                }
            ],
            "edges": [],
            "orderId": "4e277916-a1a3-411a-9d6b-685b3e973bd7",
            "orderUpdateId": 0,
            "headerId": 0,
            "version": "1.1.0",
            "timestamp": "2025-04-12T06:52:03.028Z",
            "manufacturer": "agv-dummy-manufacturer",
            "serialNumber": "14"
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.header.header_id, 0);
        assert_eq!(order.header.manufacturer, "agv-dummy-manufacturer");
        assert_eq!(order.header.serial_number, "14");
        assert_eq!(order.order_id, "4e277916-a1a3-411a-9d6b-685b3e973bd7");
        assert_eq!(order.nodes.len(), 1);
        assert_eq!(order.nodes[0].node_id, "10854");
    }
}
