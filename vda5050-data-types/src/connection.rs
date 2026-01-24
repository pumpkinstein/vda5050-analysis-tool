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
            "headerId": 1,
            "timestamp": "2026-01-24T17:19:11.012598Z",
            "version": "2.0.0",
            "manufacturer": "Acme",
            "serialNumber": "23",
            "connectionState": "ONLINE"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 1);
        assert_eq!(connection.header.manufacturer, "Acme");
        assert_eq!(connection.header.serial_number, "23");
        assert!(matches!(
            connection.connection_state,
            ConnectionState::Online
        ));
    }

    #[test]
    fn test_deserialize_connection_broken() {
        let json = r#"{
            "headerId": 2,
            "timestamp": "2026-01-24T06:19:07.242319Z",
            "version": "2.0.0",
            "manufacturer": "Acme",
            "serialNumber": "42",
            "connectionState": "CONNECTIONBROKEN"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 2);
        assert!(matches!(
            connection.connection_state,
            ConnectionState::ConnectionBroken
        ));
    }

    #[test]
    fn test_deserialize_connection_offline() {
        let json = r#"{
            "headerId": 3,
            "timestamp": "2026-01-24T06:19:07.242319Z",
            "version": "2.0.0",
            "manufacturer": "Acme",
            "serialNumber": "69",
            "connectionState": "OFFLINE"
        }"#;

        let connection: Connection = serde_json::from_str(json).unwrap();
        assert_eq!(connection.header.header_id, 3);
        assert!(matches!(
            connection.connection_state,
            ConnectionState::Offline
        ));
    }
}
