use crate::common::{Action, Header};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstantActions {
    #[serde(flatten)]
    pub header: Header,
    pub instant_actions: Vec<Action>,
}
