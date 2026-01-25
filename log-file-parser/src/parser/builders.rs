//! Arrow column builders for high-performance VDA 5050 log ingestion.
//!
//! This module implements zero-copy, cache-efficient builders following the spec:
//! - One Arrow builder per column
//! - Append column-wise (never row-wise)
//! - Batch-oriented (flush in large batches)
//! - Always append to every column (null if missing)
//! TODO: Comments probably not true, still seems like it is using Vec underneath

use polars::prelude::*;

/// Index builders - common fields for all message types
pub struct IndexBuilders {
    pub row_id: Vec<u64>,
    pub manufacturer: Vec<String>,
    pub serial_number: Vec<String>,
    pub msg_type: Vec<String>,
    pub header_id: Vec<u32>,
    pub timestamp_ns: Vec<i64>,
    pub version_packed: Vec<u32>,
}

impl IndexBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            manufacturer: Vec::with_capacity(capacity),
            serial_number: Vec::with_capacity(capacity),
            msg_type: Vec::with_capacity(capacity),
            header_id: Vec::with_capacity(capacity),
            timestamp_ns: Vec::with_capacity(capacity),
            version_packed: Vec::with_capacity(capacity),
        }
    }

    pub fn append(
        &mut self,
        row_id: u64,
        manufacturer: String,
        serial_number: String,
        msg_type: String,
        header_id: u32,
        timestamp_ns: i64,
        version_packed: u32,
    ) {
        self.row_id.push(row_id);
        self.manufacturer.push(manufacturer);
        self.serial_number.push(serial_number);
        self.msg_type.push(msg_type);
        self.header_id.push(header_id);
        self.timestamp_ns.push(timestamp_ns);
        self.version_packed.push(version_packed);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "manufacturer" => self.manufacturer,
            "serial_number" => self.serial_number,
            "msg_type" => self.msg_type,
            "header_id" => self.header_id,
            "timestamp" => Series::new("timestamp".into(), self.timestamp_ns)
                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?,
            "version_packed" => self.version_packed,
        )
    }
}

/// State message builders
pub struct StateBuilders {
    pub row_id: Vec<u64>,
    pub operating_mode: Vec<String>,
    pub battery_charge: Vec<f64>,
    pub has_errors: Vec<bool>,
}

impl StateBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            operating_mode: Vec::with_capacity(capacity),
            battery_charge: Vec::with_capacity(capacity),
            has_errors: Vec::with_capacity(capacity),
        }
    }

    pub fn append(
        &mut self,
        row_id: u64,
        operating_mode: String,
        battery_charge: f64,
        has_errors: bool,
    ) {
        self.row_id.push(row_id);
        self.operating_mode.push(operating_mode);
        self.battery_charge.push(battery_charge);
        self.has_errors.push(has_errors);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "operating_mode" => self.operating_mode,
            "battery_charge" => self.battery_charge,
            "has_errors" => self.has_errors,
        )
    }
}

/// Visualization message builders
pub struct VisualizationBuilders {
    pub row_id: Vec<u64>,
    pub x: Vec<Option<f64>>,
    pub y: Vec<Option<f64>>,
    pub theta: Vec<Option<f64>>,
    pub map_id: Vec<Option<String>>,
}

impl VisualizationBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            x: Vec::with_capacity(capacity),
            y: Vec::with_capacity(capacity),
            theta: Vec::with_capacity(capacity),
            map_id: Vec::with_capacity(capacity),
        }
    }

    pub fn append(
        &mut self,
        row_id: u64,
        x: Option<f64>,
        y: Option<f64>,
        theta: Option<f64>,
        map_id: Option<String>,
    ) {
        self.row_id.push(row_id);
        self.x.push(x);
        self.y.push(y);
        self.theta.push(theta);
        self.map_id.push(map_id);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "x" => self.x,
            "y" => self.y,
            "theta" => self.theta,
            "map_id" => self.map_id,
        )
    }
}

/// Connection message builders
pub struct ConnectionBuilders {
    pub row_id: Vec<u64>,
    pub connection_state: Vec<String>,
}

impl ConnectionBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            connection_state: Vec::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, row_id: u64, connection_state: String) {
        self.row_id.push(row_id);
        self.connection_state.push(connection_state);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "connection_state" => self.connection_state,
        )
    }
}

/// Order message builders
pub struct OrderBuilders {
    pub row_id: Vec<u64>,
    pub order_id: Vec<String>,
}

impl OrderBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            order_id: Vec::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, row_id: u64, order_id: String) {
        self.row_id.push(row_id);
        self.order_id.push(order_id);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "order_id" => self.order_id,
        )
    }
}

/// InstantActions message builders
pub struct InstantActionsBuilders {
    pub row_id: Vec<u64>,
    pub action_count: Vec<u32>,
}

impl InstantActionsBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: Vec::with_capacity(capacity),
            action_count: Vec::with_capacity(capacity),
        }
    }

    pub fn append(&mut self, row_id: u64, action_count: u32) {
        self.row_id.push(row_id);
        self.action_count.push(action_count);
    }

    pub fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "row_id" => self.row_id,
            "action_count" => self.action_count,
        )
    }
}

/// Container for all builder types
pub struct AllBuilders {
    pub index: IndexBuilders,
    pub state: StateBuilders,
    pub visualization: VisualizationBuilders,
    pub connection: ConnectionBuilders,
    pub order: OrderBuilders,
    pub instant_actions: InstantActionsBuilders,
}

impl AllBuilders {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            index: IndexBuilders::with_capacity(capacity),
            state: StateBuilders::with_capacity(capacity),
            visualization: VisualizationBuilders::with_capacity(capacity),
            connection: ConnectionBuilders::with_capacity(capacity),
            order: OrderBuilders::with_capacity(capacity),
            instant_actions: InstantActionsBuilders::with_capacity(capacity),
        }
    }
}
