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
    /// The timestamp of the last state change in ISO 8601 format.
    pub last_state_change: String,
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

