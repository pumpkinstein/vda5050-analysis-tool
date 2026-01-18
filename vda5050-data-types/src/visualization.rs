use crate::common::{AgvPosition, Header, Point, Velocity};
use serde::{Deserialize, Serialize};

/// A message for visualization purposes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Visualization {
    #[serde(flatten)]
    pub header: Header,
    /// The position of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_position: Option<AgvPosition>,
    /// The velocity of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_velocity: Option<Velocity>,
    /// The outline of the AGV.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agv_outline: Option<Vec<Point>>,
    /// A list of visualization objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualizations: Option<Vec<VisualizationObject>>,
}

/// A visualization object.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationObject {
    /// The type of the visualization object.
    #[serde(rename = "type")]
    pub visualization_type: String,
    /// The ID of the visualization_object.
    pub id: String,
    /// The data of the visualization object.
    pub data: serde_json::Value,
}

