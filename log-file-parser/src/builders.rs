//! Arrow column builders for high-performance VDA 5050 log ingestion.

use crate::models::MessageType;
use polars::{chunked_array::builder::CategoricalChunkedBuilder, prelude::*};
use std::sync::Arc;
use vda5050_data_types::{connection::ConnectionState, state::OperatingMode};

type U32Builder = PrimitiveChunkedBuilder<UInt32Type>;
type U64Builder = PrimitiveChunkedBuilder<UInt64Type>;
type I64Builder = PrimitiveChunkedBuilder<Int64Type>;
type F64Builder = PrimitiveChunkedBuilder<Float64Type>;
type BoolBuilder = BooleanChunkedBuilder;
type StringBuilder = StringChunkedBuilder;
type Categorical32Builder = CategoricalChunkedBuilder<Categorical32Type>;
type Enum8Builder = CategoricalChunkedBuilder<Categorical8Type>;

/// Shared dictionary state for one logical string column.
///
/// Every batch must use the same mapping for a column so that Polars can
/// concatenate the categorical chunks without remapping their category IDs.
#[derive(Clone)]
pub(crate) struct DictionarySpec {
    categories: Arc<Categories>,
    mapping: Arc<CategoricalMapping>,
}

impl DictionarySpec {
    pub(crate) fn new(namespace: &str) -> Self {
        let categories = Categories::random(namespace.into(), CategoricalPhysical::U32);
        let mapping = categories.mapping();

        Self {
            categories,
            mapping,
        }
    }

    fn builder(&self, name: &str, capacity: usize) -> DictionaryBuilder {
        let mut inner = Categorical32Builder::new(
            name.into(),
            DataType::Categorical(self.categories.clone(), self.mapping.clone()),
        );
        inner.reserve(capacity);

        DictionaryBuilder { inner }
    }
}

/// Categorical string builder backed by integer category IDs.
pub(crate) struct DictionaryBuilder {
    inner: Categorical32Builder,
}

/// Shared fixed dictionary state for an enum-like string column.
#[derive(Clone)]
pub(crate) struct FixedDictionarySpec {
    categories: Arc<FrozenCategories>,
    mapping: Arc<CategoricalMapping>,
}

impl FixedDictionarySpec {
    pub(crate) fn new(values: &'static [&'static str]) -> PolarsResult<Self> {
        let categories = FrozenCategories::new(values.iter().copied())?;
        let mapping = categories.mapping().clone();

        Ok(Self {
            categories,
            mapping,
        })
    }

    fn builder(&self, name: &str, capacity: usize) -> FixedDictionaryBuilder {
        let mut inner = Enum8Builder::new(
            name.into(),
            DataType::Enum(self.categories.clone(), self.mapping.clone()),
        );
        inner.reserve(capacity);

        FixedDictionaryBuilder {
            inner,
            mapping: self.mapping.clone(),
        }
    }
}

/// Fixed categorical builder that appends precomputed category IDs directly.
pub(crate) struct FixedDictionaryBuilder {
    inner: Enum8Builder,
    mapping: Arc<CategoricalMapping>,
}

impl FixedDictionaryBuilder {
    fn append_code(&mut self, code: u8) -> PolarsResult<()> {
        self.inner.append_cat(code as CatSize, &self.mapping)
    }

    fn finish(self) -> Series {
        self.inner.finish().into_series()
    }
}

impl DictionaryBuilder {
    fn append_value(&mut self, value: impl AsRef<str>) -> PolarsResult<()> {
        self.inner.append_str(value.as_ref())
    }

    fn finish(self) -> Series {
        self.inner.finish().into_series()
    }
}

/// Mappings shared by all parallel batches for the current log file.
pub(crate) struct DictionaryMappings {
    pub manufacturer: DictionarySpec,
    pub serial_number: DictionarySpec,
    pub msg_type: FixedDictionarySpec,
    pub operating_mode: FixedDictionarySpec,
    pub connection_state: FixedDictionarySpec,
}

impl DictionaryMappings {
    pub(crate) fn new() -> PolarsResult<Self> {
        Ok(Self {
            manufacturer: DictionarySpec::new("manufacturer"),
            serial_number: DictionarySpec::new("serial_number"),
            msg_type: FixedDictionarySpec::new(MessageType::VALUES)?,
            operating_mode: FixedDictionarySpec::new(OperatingMode::VALUES)?,
            connection_state: FixedDictionarySpec::new(ConnectionState::VALUES)?,
        })
    }
}

/// Index builders - common fields for all message types
pub(crate) struct IndexBuilders {
    pub row_id: U64Builder,
    pub manufacturer: DictionaryBuilder,
    pub serial_number: DictionaryBuilder,
    pub msg_type: FixedDictionaryBuilder,
    pub header_id: U32Builder,
    pub timestamp_ns: I64Builder,
    pub version_packed: U32Builder,
}

impl IndexBuilders {
    pub(crate) fn with_capacity(capacity: usize, mappings: &DictionaryMappings) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            manufacturer: mappings.manufacturer.builder("manufacturer", capacity),
            serial_number: mappings.serial_number.builder("serial_number", capacity),
            msg_type: mappings.msg_type.builder("msg_type", capacity),
            header_id: U32Builder::new("header_id".into(), capacity),
            timestamp_ns: I64Builder::new("timestamp".into(), capacity),
            version_packed: U32Builder::new("version_packed".into(), capacity),
        }
    }

    pub(crate) fn append(
        &mut self,
        row_id: u64,
        manufacturer: impl AsRef<str>,
        serial_number: impl AsRef<str>,
        msg_type: u8,
        header_id: u32,
        timestamp_ns: i64,
        version_packed: u32,
    ) -> PolarsResult<()> {
        self.row_id.append_value(row_id);
        self.manufacturer.append_value(manufacturer)?;
        self.serial_number.append_value(serial_number)?;
        self.msg_type.append_code(msg_type)?;
        self.header_id.append_value(header_id);
        self.timestamp_ns.append_value(timestamp_ns);
        self.version_packed.append_value(version_packed);
        Ok(())
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        let timestamp = self
            .timestamp_ns
            .finish()
            .into_series()
            .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))?;

        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.manufacturer.finish().into(),
            self.serial_number.finish().into(),
            self.msg_type.finish().into(),
            self.header_id.finish().into_series().into(),
            timestamp.into(),
            self.version_packed.finish().into_series().into(),
        ])
    }
}

/// State message builders
pub(crate) struct StateBuilders {
    pub row_id: U64Builder,
    pub operating_mode: FixedDictionaryBuilder,
    pub battery_charge: F64Builder,
    pub has_errors: BoolBuilder,
}

impl StateBuilders {
    pub(crate) fn with_capacity(capacity: usize, operating_mode: &FixedDictionarySpec) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            operating_mode: operating_mode.builder("operating_mode", capacity),
            battery_charge: F64Builder::new("battery_charge".into(), capacity),
            has_errors: BoolBuilder::new("has_errors".into(), capacity),
        }
    }

    pub(crate) fn append(
        &mut self,
        row_id: u64,
        operating_mode: u8,
        battery_charge: f64,
        has_errors: bool,
    ) -> PolarsResult<()> {
        self.row_id.append_value(row_id);
        self.operating_mode.append_code(operating_mode)?;
        self.battery_charge.append_value(battery_charge);
        self.has_errors.append_value(has_errors);
        Ok(())
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.operating_mode.finish().into(),
            self.battery_charge.finish().into_series().into(),
            self.has_errors.finish().into_series().into(),
        ])
    }
}

/// Visualization message builders
pub(crate) struct VisualizationBuilders {
    pub row_id: U64Builder,
    pub x: F64Builder,
    pub y: F64Builder,
    pub theta: F64Builder,
    pub map_id: StringBuilder,
}

impl VisualizationBuilders {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            x: F64Builder::new("x".into(), capacity),
            y: F64Builder::new("y".into(), capacity),
            theta: F64Builder::new("theta".into(), capacity),
            map_id: StringBuilder::new("map_id".into(), capacity),
        }
    }

    pub(crate) fn append(
        &mut self,
        row_id: u64,
        x: Option<f64>,
        y: Option<f64>,
        theta: Option<f64>,
        map_id: Option<String>,
    ) {
        self.row_id.append_value(row_id);
        self.x.append_option(x);
        self.y.append_option(y);
        self.theta.append_option(theta);
        self.map_id.append_option(map_id);
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.x.finish().into_series().into(),
            self.y.finish().into_series().into(),
            self.theta.finish().into_series().into(),
            self.map_id.finish().into_series().into(),
        ])
    }
}

/// Connection message builders
pub(crate) struct ConnectionBuilders {
    pub row_id: U64Builder,
    pub connection_state: FixedDictionaryBuilder,
}

impl ConnectionBuilders {
    pub(crate) fn with_capacity(capacity: usize, connection_state: &FixedDictionarySpec) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            connection_state: connection_state.builder("connection_state", capacity),
        }
    }

    pub(crate) fn append(&mut self, row_id: u64, connection_state: u8) -> PolarsResult<()> {
        self.row_id.append_value(row_id);
        self.connection_state.append_code(connection_state)?;
        Ok(())
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.connection_state.finish().into(),
        ])
    }
}

/// Order message builders
pub(crate) struct OrderBuilders {
    pub row_id: U64Builder,
    pub order_id: StringBuilder,
}

impl OrderBuilders {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            order_id: StringBuilder::new("order_id".into(), capacity),
        }
    }

    pub(crate) fn append(&mut self, row_id: u64, order_id: String) {
        self.row_id.append_value(row_id);
        self.order_id.append_value(order_id);
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.order_id.finish().into_series().into(),
        ])
    }
}

/// InstantActions message builders
pub struct InstantActionsBuilders {
    pub row_id: U64Builder,
    pub action_count: U32Builder,
}

impl InstantActionsBuilders {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            row_id: U64Builder::new("row_id".into(), capacity),
            action_count: U32Builder::new("action_count".into(), capacity),
        }
    }

    pub(crate) fn append(&mut self, row_id: u64, action_count: u32) {
        self.row_id.append_value(row_id);
        self.action_count.append_value(action_count);
    }

    pub(crate) fn finish(self) -> PolarsResult<DataFrame> {
        DataFrame::new_infer_height(vec![
            self.row_id.finish().into_series().into(),
            self.action_count.finish().into_series().into(),
        ])
    }
}

/// Container for all builder types
pub(crate) struct AllBuilders {
    pub index: IndexBuilders,
    pub state: StateBuilders,
    pub visualization: VisualizationBuilders,
    pub connection: ConnectionBuilders,
    pub order: OrderBuilders,
    pub instant_actions: InstantActionsBuilders,
}

impl AllBuilders {
    pub(crate) fn with_capacity(capacity: usize, mappings: &DictionaryMappings) -> Self {
        Self {
            index: IndexBuilders::with_capacity(capacity, mappings),
            state: StateBuilders::with_capacity(capacity, &mappings.operating_mode),
            visualization: VisualizationBuilders::with_capacity(capacity),
            connection: ConnectionBuilders::with_capacity(capacity, &mappings.connection_state),
            order: OrderBuilders::with_capacity(capacity),
            instant_actions: InstantActionsBuilders::with_capacity(capacity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_builder_preserves_values_and_timestamp_dtype() {
        let mappings = DictionaryMappings::new().unwrap();
        let mut builders = IndexBuilders::with_capacity(1, &mappings);
        builders
            .append(
                7,
                "manufacturer".to_string(),
                "serial".to_string(),
                0,
                42,
                1_000,
                0x0201_0000,
            )
            .unwrap();

        let dataframe = builders.finish().unwrap();

        assert_eq!(dataframe.height(), 1);
        assert_eq!(
            dataframe
                .column("manufacturer")
                .unwrap()
                .as_materialized_series()
                .cat32()
                .unwrap()
                .iter_str()
                .next(),
            Some(Some("manufacturer"))
        );
        assert!(
            dataframe
                .column("manufacturer")
                .unwrap()
                .dtype()
                .is_categorical()
        );
        assert_eq!(
            dataframe
                .column("msg_type")
                .unwrap()
                .as_materialized_series()
                .cat8()
                .unwrap()
                .iter_str()
                .next(),
            Some(Some("state"))
        );
        assert!(dataframe.column("msg_type").unwrap().dtype().is_enum());
        assert_eq!(
            dataframe.column("timestamp").unwrap().dtype(),
            &DataType::Datetime(TimeUnit::Nanoseconds, None)
        );
    }

    #[test]
    fn visualization_builder_preserves_nullable_values() {
        let mut builders = VisualizationBuilders::with_capacity(1);
        builders.append(3, None, Some(2.0), None, None);

        let dataframe = builders.finish().unwrap();

        assert_eq!(dataframe.height(), 1);
        assert_eq!(dataframe.column("x").unwrap().null_count(), 1);
        assert_eq!(dataframe.column("y").unwrap().null_count(), 0);
        assert_eq!(dataframe.column("theta").unwrap().null_count(), 1);
        assert_eq!(dataframe.column("map_id").unwrap().null_count(), 1);
    }
}
