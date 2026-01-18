use crate::common::Header;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    #[serde(flatten)]
    pub header: Header,
    pub connection_state: ConnectionState,
    pub last_state_change: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionState {
    #[serde(rename = "ONLINE")]
    Online,
    #[serde(rename = "OFFLINE")]
    Offline,
    #[serde(rename = "CONNECTIONBROKEN")]
    ConnectionBroken,
}
