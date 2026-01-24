//! Contains the data models for parsed log entries.

/// Represents a single, flattened record that has been parsed from a raw
/// log entry and is ready to be loaded into a Polars DataFrame.
///
/// This struct combines information from the MQTT topic, the message header,
/// and the message payload (either a `State` or `Visualization` message).
/// Fields that are not applicable to a given message type will be `None`.
#[derive(Debug, Default)]
pub struct ParsedRecord {
    // --- Data from MQTT Topic ---
    /// Name of the AGV manufacturer.
    pub manufacturer: String,
    /// Serial number of the AGV.
    pub serial_number: String,
    /// The type of VDA 5050 message (e.g., "state", "visualization").
    pub msg_type: String,

    // --- Data from VDA 5050 Header ---
    /// Unique continuous number for the message.
    pub header_id: u32,
    /// Timestamp converted to microseconds since the Unix Epoch.
    pub timestamp_us: i64,
    /// VDA 5050 version packed into a u32 for efficient storage and comparison.
    pub version_packed: u32,

    // --- Data from State message payload ---
    /// The operating mode of the AGV (e.g., "AUTOMATIC", "MANUAL").
    pub operating_mode: Option<String>,
    /// The battery charge in percent.
    pub battery_charge: Option<f64>,
    /// Flag indicating if the state message contains any errors.
    pub has_errors: Option<bool>,

    // --- Data from Visualization message payload ---
    /// The X-coordinate of the AGV.
    pub x: Option<f64>,
    /// The Y-coordinate of the AGV.
    pub y: Option<f64>,
    /// The orientation of the AGV.
    pub theta: Option<f64>,
    /// The map ID the AGV is on.
    pub map_id: Option<String>,
}
