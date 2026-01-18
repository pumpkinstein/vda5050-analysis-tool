//! Contains the data models for parsed log entries.

use vda5050_data_types::{
    connection::Connection, instant_actions::InstantActions, order::Order, state::State,
    visualization::Visualization,
};

// Fixed message types used by the index Enum column.
vda5050_data_types::fixed_string_enum! {
    pub(crate) enum MessageType {
        State => "state",
        Visualization => "visualization",
        Connection => "connection",
        Order => "order",
        InstantActions => "instantActions",
    }
}

/// Minimal metadata extracted from the MQTT topic
#[derive(Debug, Clone)]
pub(crate) struct TopicMetadata {
    pub manufacturer: String,
    pub serial_number: String,
}

/// A parsed VDA 5050 message with topic metadata
#[derive(Debug)]
pub(crate) enum ParsedMessage {
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
