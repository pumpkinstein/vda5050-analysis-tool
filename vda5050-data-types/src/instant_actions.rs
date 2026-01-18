use crate::common::{Action, Header};
use serde::{Deserialize, Serialize};

/// A list of instant actions to be executed by the AGV.
///
/// Instant actions are actions that are executed immediately by the AGV,
/// without being part of an order. They are published on the /instantActions topic
/// and the AGV is to execute them as soon as they arrive.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstantActions {
    #[serde(flatten)]
    pub header: Header,
    /// Array of action objects with detailed information.
    /// These actions are to be executed immediately by the AGV.
    pub actions: Vec<Action>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_instant_actions_empty() {
        let json = r#"{
            "headerId": 1,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "actions": []
        }"#;

        let instant_actions: InstantActions = serde_json::from_str(json).unwrap();
        assert_eq!(instant_actions.header.header_id, 1);
        assert_eq!(instant_actions.header.manufacturer, "test-mfr");
        assert_eq!(instant_actions.header.serial_number, "agv-001");
        assert_eq!(instant_actions.actions.len(), 0);
    }

    #[test]
    fn test_deserialize_instant_actions_with_actions() {
        let json = r#"{
            "headerId": 2,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "actions": [
                {
                    "actionId": "action-123",
                    "actionType": "startPause",
                    "blockingType": "HARD"
                },
                {
                    "actionId": "action-456",
                    "actionType": "stopPause",
                    "blockingType": "HARD"
                }
            ]
        }"#;

        let instant_actions: InstantActions = serde_json::from_str(json).unwrap();
        assert_eq!(instant_actions.header.header_id, 2);
        assert_eq!(instant_actions.actions.len(), 2);
        assert_eq!(instant_actions.actions[0].action_id, "action-123");
        assert_eq!(instant_actions.actions[0].action_type, "startPause");
        assert_eq!(instant_actions.actions[1].action_id, "action-456");
        assert_eq!(instant_actions.actions[1].action_type, "stopPause");
    }

    #[test]
    fn test_deserialize_instant_actions_with_parameters() {
        let json = r#"{
            "headerId": 3,
            "timestamp": "2024-05-20T10:00:00Z",
            "version": "2.0.0",
            "manufacturer": "test-mfr",
            "serialNumber": "agv-001",
            "actions": [
                {
                    "actionId": "action-789",
                    "actionType": "pick",
                    "blockingType": "SOFT",
                    "actionDescription": "Pick up load",
                    "actionParameters": [
                        {
                            "key": "stationType",
                            "value": "floor"
                        },
                        {
                            "key": "loadId",
                            "value": "pallet-42"
                        }
                    ]
                }
            ]
        }"#;

        let instant_actions: InstantActions = serde_json::from_str(json).unwrap();
        assert_eq!(instant_actions.actions.len(), 1);
        assert_eq!(instant_actions.actions[0].action_id, "action-789");
        assert_eq!(instant_actions.actions[0].action_type, "pick");
        assert_eq!(
            instant_actions.actions[0].action_description,
            Some("Pick up load".to_string())
        );

        let params = instant_actions.actions[0]
            .action_parameters
            .as_ref()
            .unwrap();
        assert_eq!(params.len(), 2);
    }
}
