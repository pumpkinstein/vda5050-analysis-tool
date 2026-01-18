use crate::common::{AgvPosition, Header, Point, Velocity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Visualization {
    #[serde(flatten)]
    pub header: Header,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_velocity: Option<Velocity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_outline: Option<Vec<Point>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualizations: Option<Vec<VisualizationObject>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationObject {
    #[serde(rename = "type")]
    pub visualization_type: String,
    pub id: String,
    pub data: serde_json::Value,
}
