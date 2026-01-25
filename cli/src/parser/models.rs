//! Contains the data models for parsed log entries.

use vda5050_data_types::{
    connection::Connection, instant_actions::InstantActions, order::Order, state::State,
    visualization::Visualization,
};

/// Minimal metadata extracted from the MQTT topic
#[derive(Debug, Clone)]
pub struct TopicMetadata {
    pub manufacturer: String,
    pub serial_number: String,
}

/// A parsed VDA 5050 message with topic metadata
#[derive(Debug)]
pub enum ParsedMessage {
    State {
        topic: TopicMetadata,
        data: State,
    },
    Visualization {
        topic: TopicMetadata,
        data: Visualization,
    },
    Connection {
        topic: TopicMetadata,
        data: Connection,
    },
    Order {
        topic: TopicMetadata,
        data: Order,
    },
    InstantActions {
        topic: TopicMetadata,
        data: InstantActions,
    },
}

impl ParsedMessage {
    /// Get the message type as a string
    pub fn msg_type(&self) -> &str {
        match self {
            ParsedMessage::State { .. } => "state",
            ParsedMessage::Visualization { .. } => "visualization",
            ParsedMessage::Connection { .. } => "connection",
            ParsedMessage::Order { .. } => "order",
            ParsedMessage::InstantActions { .. } => "instantActions",
        }
    }

    /// Get the topic metadata from any message variant
    pub fn topic(&self) -> &TopicMetadata {
        match self {
            ParsedMessage::State { topic, .. } => topic,
            ParsedMessage::Visualization { topic, .. } => topic,
            ParsedMessage::Connection { topic, .. } => topic,
            ParsedMessage::Order { topic, .. } => topic,
            ParsedMessage::InstantActions { topic, .. } => topic,
        }
    }
}
