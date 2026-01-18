use crate::common::{Action, Header};
use serde::{Deserialize, Serialize};

/// A list of instant actions to be executed by the AGV.
///
/// Instant actions are actions that are executed immediately by the AGV,
/// without being part of an order.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstantActions {
    #[serde(flatten)]
    pub header: Header,
    /// The list of instant actions.
    pub instant_actions: Vec<Action>,
}

