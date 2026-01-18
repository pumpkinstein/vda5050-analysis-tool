use crate::common::{Action, Header};
use serde::{Deserialize, Serialize};

/// A list of instant actions to be executed by the AGV.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstantActions {
    #[serde(flatten)]
    pub header: Header,
    /// The list of instant actions.
    pub instant_actions: Vec<Action>,
}

