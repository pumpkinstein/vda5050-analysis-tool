use crate::common::Header;
use serde::{Deserialize, Serialize};

/// A message that contains connection information.
///
/// This message is sent by the AGV to the master control to indicate its connection state.
/// It is also sent by the broker as a last will to indicate that the connection to the AGV was lost.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    #[serde(flatten)]
    pub header: Header,
    /// The state of the connection.
    pub connection_state: ConnectionState,
}

/// The state of the connection.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionState {
    /// The AGV is online and connected to the master control.
    #[serde(rename = "ONLINE")]
    Online,
    /// The AGV is offline and not connected to the master control.
    #[serde(rename = "OFFLINE")]
    Offline,
    /// The connection to the AGV was lost.
    #[serde(rename = "CONNECTIONBROKEN")]
    ConnectionBroken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_connection_online() {
        let json = r#"{
            "headerId": 5,
            "timestamp": "2025-04-12T06:19:11.012598Z",
            "version": "1.1.0",
            "manufacturer": "Jungheinrich",
            "serialNumber": "2",
            "connectionState": "ONLINE"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 5);
        assert_eq!(connection.header.manufacturer, "Jungheinrich");
        assert_eq!(connection.header.serial_number, "2");
        assert!(matches!(connection.connection_state, ConnectionState::Online));
    }

    #[test]
    fn test_deserialize_connection_broken() {
        let json = r#"{
            "headerId": 4,
            "timestamp": "2025-04-12T06:19:07.242319Z",
            "version": "1.1.0",
            "manufacturer": "Jungheinrich",
            "serialNumber": "2",
            "connectionState": "CONNECTIONBROKEN"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 4);
        assert!(matches!(connection.connection_state, ConnectionState::ConnectionBroken));
    }

    #[test]
    fn test_deserialize_connection_offline() {
        let json = r#"{
            "headerId": 1,
            "timestamp": "2025-04-12T06:19:07.242319Z",
            "version": "1.1.0",
            "manufacturer": "Test",
            "serialNumber": "1",
            "connectionState": "OFFLINE"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 1);
        assert!(matches!(connection.connection_state, ConnectionState::Offline));
    }
}
