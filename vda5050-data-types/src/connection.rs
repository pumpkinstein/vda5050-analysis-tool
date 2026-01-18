use crate::common::Header;
use serde::{Deserialize, Serialize};

/// A message that contains connection information.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    #[serde(flatten)]
    pub header: Header,
    /// The state of the connection.
    pub connection_state: ConnectionState,
    /// The timestamp of the last state change.
    pub last_state_change: String,
}

/// The state of the connection.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionState {
    /// The AGV is online.
    #[serde(rename = "ONLINE")]
    Online,
    /// The AGV is offline.
    #[serde(rename = "OFFLINE")]
    Offline,
    /// The connection to the AGV is broken.
    #[serde(rename = "CONNECTIONBROKEN")]
    ConnectionBroken,
}

