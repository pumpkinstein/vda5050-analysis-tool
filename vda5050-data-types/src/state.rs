use crate::common::{AgvPosition, Header, NodePosition, Trajectory, Velocity};
use serde::{Deserialize, Serialize};

/// The state of the AGV.
/// All-encompassing state of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(flatten)]
    pub header: Header,
    /// Array of map-objects that are currently stored on the vehicle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maps: Option<Vec<Map>>,
    /// Unique order identification of the current order or the previous finished order.
    /// The orderId is kept until a new order is received.
    /// Empty string ("") if no previous orderId is available.
    pub order_id: String,
    /// Order Update Identification to identify that an order update has been accepted by the AGV.
    /// "0" if no previous orderUpdateId is available.
    pub order_update_id: u32,
    /// Unique ID of the zone set that the AGV currently uses for path planning.
    /// Must be the same as the one used in the order, otherwise the AGV is to reject the order.
    /// Optional: If the AGV does not use zones, this field can be omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_set_id: Option<String>,
    /// nodeID of last reached node or, if AGV is currently on a node, current node (e.g., "node7").
    /// Empty string ("") if no lastNodeId is available.
    pub last_node_id: String,
    /// sequenceId of the last reached node or, if the AGV is currently on a node, sequenceId of current node.
    /// "0" if no lastNodeSequenceId is available.
    pub last_node_sequence_id: u32,
    /// True: indicates that the AGV is driving and/or rotating.
    /// Other movements of the AGV (e.g., lift movements) are not included here.
    /// False: indicates that the AGV is neither driving nor rotating.
    pub driving: bool,
    /// True: AGV is currently in a paused state, either because of the push of a physical button
    /// on the AGV or because of an instantAction. The AGV can resume the order.
    /// False: The AGV is currently not in a paused state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    /// True: AGV is almost at the end of the base and will reduce speed if no new base is transmitted.
    /// Trigger for master control to send new base.
    /// False: no base update required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_base_request: Option<bool>,
    /// Used by line guided vehicles to indicate the distance it has been driving past the "lastNodeId".
    /// Distance is in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_since_last_node: Option<f64>,
    /// Current operating mode of the AGV.
    pub operating_mode: OperatingMode,
    /// Array of nodeState-Objects, that need to be traversed for fulfilling the order.
    /// Empty list if idle.
    #[serde(default)]
    pub node_states: Vec<NodeState>,
    /// Array of edgeState-Objects, that need to be traversed for fulfilling the order.
    /// Empty list if idle.
    #[serde(default)]
    pub edge_states: Vec<EdgeState>,
    /// Defines the position on a map in world coordinates. Each floor has its own map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    /// The AGV's velocity in vehicle coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<Velocity>,
    /// Loads, that are currently handled by the AGV.
    /// Optional: If AGV cannot determine load state, leave the array out of the state.
    /// If the AGV can determine the load state, but the array is empty, the AGV is considered unloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loads: Option<Vec<Load>>,
    /// Contains a list of the current actions and the actions which are yet to be finished.
    /// This may include actions from previous nodes that are still in progress.
    /// When an action is completed, an updated state message is published with actionStatus set to finished
    /// and if applicable with the corresponding resultDescription.
    /// The actionStates are kept until a new order is received.
    #[serde(default)]
    pub action_states: Vec<ActionState>,
    /// Contains all battery-related information.
    pub battery_state: BatteryState,
    /// Array of error-objects. All active errors of the AGV should be in the list.
    /// An empty array indicates that the AGV has no active errors.
    #[serde(default)]
    pub errors: Vec<Error>,
    /// Array of info-objects. An empty array indicates, that the AGV has no information.
    /// This should only be used for visualization or debugging – it must not be used for logic in master control.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information: Option<Vec<Info>>,
    /// Contains all safety-related information.
    pub safety_state: SafetyState,
}

/// Map object describing a map stored on the vehicle.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    /// ID of the map describing a defined area of the vehicle's workspace.
    pub map_id: String,
    /// Version of the map.
    pub map_version: String,
    /// Additional information on the map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_description: Option<String>,
    /// Information on the status of the map indicating, if a map version is currently used on the vehicle.
    pub map_status: MapStatus,
}

/// Status of a map on the vehicle.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MapStatus {
    /// Indicates this map is currently active / used on the AGV.
    /// At most one map with the same mapId can have its status set to ENABLED.
    #[serde(rename = "ENABLED")]
    Enabled,
    /// Indicates this map version is currently not enabled on the AGV and thus could be enabled or deleted by request.
    #[serde(rename = "DISABLED")]
    Disabled,
}

/// The operating mode of the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OperatingMode {
    /// The AGV is operating in automatic mode.
    #[serde(rename = "AUTOMATIC")]
    Automatic,
    /// The AGV is operating in semi-automatic mode.
    #[serde(rename = "SEMIAUTOMATIC")]
    SemiAutomatic,
    /// The AGV is operating in manual mode.
    #[serde(rename = "MANUAL")]
    Manual,
    /// The AGV is in service mode.
    #[serde(rename = "SERVICE")]
    Service,
    /// The AGV is in teach-in mode.
    #[serde(rename = "TEACHIN")]
    TeachIn,
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
    /// Optional: Master control has this information. Can be sent additionally, e.g., for debugging purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_position: Option<NodePosition>,
    /// Array of action states for actions that are to be executed on this node.
    #[serde(default)]
    pub action_states: Vec<ActionState>,
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
    /// Trajectory segments reach from the point, where the AGV starts to enter the edge to the point
    /// where it reports that the next node was traversed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<Trajectory>,
    /// Array of action states for actions that are to be executed on this edge.
    #[serde(default)]
    pub action_states: Vec<ActionState>,
}

/// The state of an action.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActionState {
    /// Unique actionId.
    pub action_id: String,
    /// actionType of the action.
    /// Optional: Only for informational or visualization purposes. Order knows the type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Additional information on the current action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_description: Option<String>,
    /// Status of the action.
    pub action_status: ActionStatus,
    /// Description of the result, e.g., the result of a RFID-read.
    /// Errors will be transmitted in errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_description: Option<String>,
}

/// Status of an action.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ActionStatus {
    /// Waiting for the trigger (passing the node, entering the edge).
    #[serde(rename = "WAITING")]
    Waiting,
    /// Action is being initialized.
    #[serde(rename = "INITIALIZING")]
    Initializing,
    /// Action is running.
    #[serde(rename = "RUNNING")]
    Running,
    /// Paused by instantAction or external trigger.
    #[serde(rename = "PAUSED")]
    Paused,
    /// Action has finished successfully.
    #[serde(rename = "FINISHED")]
    Finished,
    /// Action could not be performed.
    #[serde(rename = "FAILED")]
    Failed,
}

/// The state of a load on the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Load {
    /// Unique identification number of the load (e.g., barcode or RFID).
    /// Empty field, if the AGV can identify the load, but did not identify the load yet.
    /// Optional, if the AGV cannot identify the load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_id: Option<String>,
    /// Type of load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_type: Option<String>,
    /// Indicates, which load handling/carrying unit of the AGV is used, e.g., in case the AGV
    /// has multiple spots/positions to carry loads.
    /// Optional for vehicles with only one loadPosition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_position: Option<String>,
    /// Point of reference for the location of the bounding box.
    /// The point of reference is always the center of the bounding box bottom surface (at height = 0)
    /// and is described in coordinates of the AGV coordinate system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box_reference: Option<BoundingBoxReference>,
    /// Dimensions of the load's bounding box in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_dimensions: Option<LoadDimensions>,
    /// Absolute weight of the load measured in kg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
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
    /// Orientation of the load's bounding box. Important for tugger, trains, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta: Option<f64>,
}

/// Dimensions of a load's bounding box.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoadDimensions {
    /// Absolute length of the load's bounding box in meter.
    pub length: f64,
    /// Absolute width of the load's bounding box in meter.
    pub width: f64,
    /// Absolute height of the load's bounding box in meter.
    /// Optional: Set value only if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// An error that occurred on the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// Type/name of error.
    pub error_type: String,
    /// Array of references to provide more information related to the error
    /// (e.g. nodeId, edgeId, orderId, actionId, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_references: Option<Vec<ErrorReference>>,
    /// Verbose description providing details and possible causes of the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    /// Hint on how to approach or solve the reported error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_hint: Option<String>,
    /// The level of the error.
    /// WARNING: AGV is ready to start (e.g., maintenance cycle expiration warning).
    /// FATAL: AGV is not in running condition, user intervention required (e.g., laser scanner is contaminated).
    pub error_level: ErrorLevel,
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
    /// Specifies the type of reference used (e.g. nodeId, edgeId, orderId, actionId, etc.).
    pub reference_key: String,
    /// The value that belongs to the reference key. For example, the id of the node where the error occurred.
    pub reference_value: String,
}

/// An information message from the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// Type/name of information.
    pub info_type: String,
    /// Array of references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_references: Option<Vec<InfoReference>>,
    /// Info description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_description: Option<String>,
    /// The level of the information message.
    /// DEBUG: used for debugging.
    /// INFO: used for visualization.
    pub info_level: InfoLevel,
}

/// The level of the information message.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InfoLevel {
    /// Used for debugging.
    #[serde(rename = "DEBUG")]
    Debug,
    /// Used for visualization.
    #[serde(rename = "INFO")]
    Info,
}

/// A reference to an information message.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InfoReference {
    /// References the type of reference (e.g., headerId, orderId, actionId, etc.).
    pub reference_key: String,
    /// References the value, which belongs to the reference key.
    pub reference_value: String,
}

/// The state of the AGV's battery.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    /// State of Charge in %.
    /// If AGV only provides values for good or bad battery levels, these will be indicated as 20% (bad) and 80% (good).
    pub battery_charge: f64,
    /// Battery voltage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>,
    /// State of health in percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_health: Option<f64>,
    /// True: charging in progress. False: AGV is currently not charging.
    pub charging: bool,
    /// Estimated reach with current State of Charge in meter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<f64>,
}

/// Contains all safety-related information.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SafetyState {
    /// Acknowledge-Type of eStop.
    pub e_stop: EStop,
    /// Protective field violation.
    /// True: field is violated. False: field is not violated.
    pub field_violation: bool,
}

/// Acknowledge-Type of eStop.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EStop {
    /// Auto-acknowledgeable e-stop is activated, e.g., by bumper or protective field.
    /// While alias isn't correct saw it show up in real world logs
    #[serde(rename = "AUTOACK", alias = "autoAck")]
    AutoAck,
    /// E-stop has to be acknowledged manually at the vehicle.
    #[serde(rename = "MANUAL")]
    Manual,
    /// Facility e-stop has to be acknowledged remotely.
    #[serde(rename = "REMOTE")]
    Remote,
    /// No e-stop activated.
    #[serde(rename = "NONE")]
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_state_minimal() {
        let json = r#"{
            "headerId": 1,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "",
            "orderUpdateId": 0,
            "lastNodeId": "",
            "lastNodeSequenceId": 0,
            "driving": false,
            "operatingMode": "AUTOMATIC",
            "nodeStates": [],
            "edgeStates": [],
            "actionStates": [],
            "batteryState": {
                "batteryCharge": 85.0,
                "charging": false
            },
            "errors": [],
            "safetyState": {
                "eStop": "NONE",
                "fieldViolation": false
            }
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.header.header_id, 1);
        assert_eq!(state.order_id, "");
        assert_eq!(state.order_update_id, 0);
        assert_eq!(state.driving, false);
        assert!(matches!(state.operating_mode, OperatingMode::Automatic));
        assert_eq!(state.battery_state.battery_charge, 85.0);
        assert_eq!(state.battery_state.charging, false);
        assert!(matches!(state.safety_state.e_stop, EStop::None));
    }

    #[test]
    fn test_deserialize_state_with_errors() {
        let json = r#"{
            "headerId": 2,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-123",
            "orderUpdateId": 1,
            "lastNodeId": "node1",
            "lastNodeSequenceId": 0,
            "driving": true,
            "operatingMode": "AUTOMATIC",
            "nodeStates": [],
            "edgeStates": [],
            "actionStates": [],
            "batteryState": {
                "batteryCharge": 45.0,
                "charging": false
            },
            "errors": [
                {
                    "errorType": "E-STOP",
                    "errorLevel": "FATAL",
                    "errorDescription": "Emergency stop activated"
                }
            ],
            "safetyState": {
                "eStop": "MANUAL",
                "fieldViolation": false
            }
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.errors.len(), 1);
        assert_eq!(state.errors[0].error_type, "E-STOP");
        assert!(matches!(state.errors[0].error_level, ErrorLevel::Fatal));
    }

    #[test]
    fn test_deserialize_state_with_action_states() {
        let json = r#"{
            "headerId": 3,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "orderId": "order-456",
            "orderUpdateId": 2,
            "lastNodeId": "node2",
            "lastNodeSequenceId": 2,
            "driving": false,
            "operatingMode": "AUTOMATIC",
            "nodeStates": [],
            "edgeStates": [],
            "actionStates": [
                {
                    "actionId": "action-123",
                    "actionType": "pick",
                    "actionStatus": "RUNNING"
                }
            ],
            "batteryState": {
                "batteryCharge": 75.0,
                "charging": false
            },
            "errors": [],
            "safetyState": {
                "eStop": "NONE",
                "fieldViolation": false
            }
        }"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.action_states.len(), 1);
        assert_eq!(state.action_states[0].action_id, "action-123");
        assert!(matches!(
            state.action_states[0].action_status,
            ActionStatus::Running
        ));
    }

    #[test]
    fn test_operating_mode_variants() {
        assert!(matches!(
            serde_json::from_str::<OperatingMode>("\"AUTOMATIC\"").unwrap(),
            OperatingMode::Automatic
        ));
        assert!(matches!(
            serde_json::from_str::<OperatingMode>("\"SEMIAUTOMATIC\"").unwrap(),
            OperatingMode::SemiAutomatic
        ));
        assert!(matches!(
            serde_json::from_str::<OperatingMode>("\"MANUAL\"").unwrap(),
            OperatingMode::Manual
        ));
        assert!(matches!(
            serde_json::from_str::<OperatingMode>("\"SERVICE\"").unwrap(),
            OperatingMode::Service
        ));
        assert!(matches!(
            serde_json::from_str::<OperatingMode>("\"TEACHIN\"").unwrap(),
            OperatingMode::TeachIn
        ));
    }
}
