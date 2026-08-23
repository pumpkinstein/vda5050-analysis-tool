pub mod builders;
pub mod io;
pub mod models;
pub mod process;
pub mod validation;

pub use models::MessageType;
pub use validation::{CANONICAL_FRAME_NAMES, ValidationErrors, validate_result};

use crate::{
    builders::{AllBuilders, DictionaryMappings},
    io::{VdaIterator, parse_version, root_topic_prefix},
    models::ParsedMessage,
    process::{ParseBuffers, parse_record_with_buffers},
};
use anyhow::Result;
use polars::prelude::*;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Default VDA 5050 MQTT root topic used by the CLI and HMI.
pub const DEFAULT_ROOT_TOPIC: &str = "uagv/v1";

#[derive(Debug, Clone, Copy)]
pub struct ProcessingTimings {
    pub mmap_setup: Duration,
    pub delimiter_scanning: Duration,
    pub parsing_and_builder_appends: Duration,
    pub batch_dataframe_construction: Duration,
    pub final_dataframe_concatenation: Duration,
}

#[derive(Debug)]
pub struct VdaAnalysisResult {
    pub dataframes: HashMap<String, DataFrame>,
    pub total_chunks: usize,
    pub num_parsed: usize,
    pub parse_failures: HashMap<String, usize>,
    pub parse_examples: HashMap<String, String>,
    pub timings: ProcessingTimings,
}

/// Processes a VDA5050 log file and returns dataframes for each message type.
pub fn process_log_file(
    file_path: &Path,
    root_topic: &str,
    batch_size: usize,
    verbose: bool,
) -> Result<VdaAnalysisResult> {
    let root_topic_prefix = root_topic_prefix(root_topic)?;

    // 1. Memory-map the file for zero-copy reads.
    let mmap_start = Instant::now();
    let file = File::open(file_path)?;
    // SAFETY: The file is read-only, and we are the only process accessing it.
    // The file will not be modified while the mmap is active.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let mmap_setup = mmap_start.elapsed();

    // 2. Use the custom iterator to find all VDA 5050 message chunks.
    let delimiter_scan_start = Instant::now();
    let iterator = VdaIterator::new(&mmap, &root_topic_prefix);
    let chunks: Vec<_> = iterator.collect();
    let total_chunks = chunks.len();
    let delimiter_scanning = delimiter_scan_start.elapsed();

    // 3. Process chunks in parallel batches using Rayon with per-thread builders
    // Track parse failures by message type
    let parse_failures: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    let parse_examples: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    let dictionary_mappings = DictionaryMappings::new()?;

    // Split work into batches for parallel processing
    let num_batches = total_chunks.div_ceil(batch_size);

    // Parse records and append them to per-batch builders in parallel.
    let parsing_start = Instant::now();
    let batch_builders: Vec<_> = (0..num_batches)
        .into_par_iter()
        .map_init(ParseBuffers::default, |parse_buffers, batch_idx| {
            let start_idx = batch_idx * batch_size;
            let end_idx = (start_idx + batch_size).min(total_chunks);
            let batch_chunks = &chunks[start_idx..end_idx];

            // Per-thread builders - column-wise append
            let mut builders = AllBuilders::with_capacity(batch_size, &dictionary_mappings);
            let mut row_id_offset = start_idx as u64;

            for chunk in batch_chunks {
                match parse_record_with_buffers(chunk, parse_buffers, &root_topic_prefix) {
                    Ok(message) => {
                        // Consume the parsed message so topic metadata can be moved directly
                        // into the output builders instead of being cloned for every row.
                        match message {
                            ParsedMessage::State { topic, data } => {
                                let operating_mode = data.operating_mode.code();

                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer,
                                    topic.serial_number,
                                    MessageType::State.code(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                )?;

                                builders.state.append(
                                    row_id_offset,
                                    operating_mode,
                                    data.battery_state.battery_charge,
                                    !data.errors.is_empty(),
                                )?;
                            }
                            ParsedMessage::Visualization { topic, data } => {
                                let manufacturer = data.manufacturer.unwrap_or(topic.manufacturer);
                                let serial_number =
                                    data.serial_number.unwrap_or(topic.serial_number);

                                builders.index.append(
                                    row_id_offset,
                                    manufacturer,
                                    serial_number,
                                    MessageType::Visualization.code(),
                                    data.header_id.unwrap(),
                                    data.timestamp.unwrap() * 1000, // Convert µs to ns
                                    parse_version(data.version.as_ref().unwrap()),
                                )?;

                                // Extract position data if available
                                let (x, y, theta, map_id) = if let Some(pos) = data.agv_position {
                                    (Some(pos.x), Some(pos.y), Some(pos.theta), Some(pos.map_id))
                                } else {
                                    (None, None, None, None)
                                };

                                builders
                                    .visualization
                                    .append(row_id_offset, x, y, theta, map_id);
                            }
                            ParsedMessage::Connection { topic, data } => {
                                let connection_state = data.connection_state.code();

                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer,
                                    topic.serial_number,
                                    MessageType::Connection.code(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                )?;

                                builders
                                    .connection
                                    .append(row_id_offset, connection_state)?;
                            }
                            ParsedMessage::Order { topic, data } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer,
                                    topic.serial_number,
                                    MessageType::Order.code(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                )?;

                                builders.order.append(row_id_offset, data.order_id);
                            }
                            ParsedMessage::InstantActions { topic, data } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer,
                                    topic.serial_number,
                                    MessageType::InstantActions.code(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                )?;

                                builders
                                    .instant_actions
                                    .append(row_id_offset, data.actions.len() as u32);
                            }
                        }

                        row_id_offset += 1;
                    }
                    Err(e) => {
                        // Extract message type from topic for failure statistics
                        let chunk_str = std::str::from_utf8(chunk).unwrap_or("");
                        if let Some(topic_part) = chunk.strip_prefix(root_topic_prefix.as_slice()) {
                            let topic_part = std::str::from_utf8(topic_part).unwrap_or("");
                            if let Some(slash_pos) = topic_part.find('/') {
                                let rest = &topic_part[slash_pos + 1..];
                                if let Some(slash_pos2) = rest.find('/') {
                                    let rest2 = &rest[slash_pos2 + 1..];
                                    if let Some(space_pos) = rest2.find(' ') {
                                        let msg_type = &rest2[..space_pos];
                                        let mut failures = parse_failures.lock().unwrap();
                                        *failures.entry(msg_type.to_string()).or_insert(0) += 1;

                                        // Store first example of each failure type
                                        if verbose {
                                            let mut examples = parse_examples.lock().unwrap();
                                            examples.entry(msg_type.to_string()).or_insert_with(
                                                || {
                                                    let preview = if chunk_str.len() > 200 {
                                                        format!("{}...", &chunk_str[..200])
                                                    } else {
                                                        chunk_str.to_string()
                                                    };
                                                    format!("Error: {}\nExample: {}", e, preview)
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(builders)
        })
        .collect::<Result<Vec<_>>>()?;
    let parsing_and_builder_appends = parsing_start.elapsed();

    // Construct the six DataFrames for each batch in parallel.
    let batch_dataframe_start = Instant::now();
    let batch_results: Vec<_> = batch_builders
        .into_par_iter()
        .map(|builders| {
            Ok((
                builders.index.finish()?,
                builders.state.finish()?,
                builders.visualization.finish()?,
                builders.connection.finish()?,
                builders.order.finish()?,
                builders.instant_actions.finish()?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let batch_dataframe_construction = batch_dataframe_start.elapsed();

    // 4. Concatenate batches into final DataFrames
    let concatenation_start = Instant::now();
    let mut index_batches = Vec::new();
    let mut state_batches = Vec::new();
    let mut viz_batches = Vec::new();
    let mut conn_batches = Vec::new();
    let mut order_batches = Vec::new();
    let mut ia_batches = Vec::new();

    for (index, state, viz, conn, order, ia) in batch_results {
        if index.height() > 0 {
            index_batches.push(index);
        }
        if state.height() > 0 {
            state_batches.push(state);
        }
        if viz.height() > 0 {
            viz_batches.push(viz);
        }
        if conn.height() > 0 {
            conn_batches.push(conn);
        }
        if order.height() > 0 {
            order_batches.push(order);
        }
        if ia.height() > 0 {
            ia_batches.push(ia);
        }
    }

    let index_df = concatenate_dataframes(&index_batches)?;
    let state_df = concatenate_dataframes(&state_batches)?;
    let viz_df = concatenate_dataframes(&viz_batches)?;
    let conn_df = concatenate_dataframes(&conn_batches)?;
    let order_df = concatenate_dataframes(&order_batches)?;
    let ia_df = concatenate_dataframes(&ia_batches)?;
    let final_dataframe_concatenation = concatenation_start.elapsed();

    let num_parsed = index_df.height();

    let mut dataframes = HashMap::new();
    dataframes.insert("index".to_string(), index_df);
    dataframes.insert(MessageType::State.dataframe_name().to_string(), state_df);
    dataframes.insert(
        MessageType::Visualization.dataframe_name().to_string(),
        viz_df,
    );
    dataframes.insert(
        MessageType::Connection.dataframe_name().to_string(),
        conn_df,
    );
    dataframes.insert(MessageType::Order.dataframe_name().to_string(), order_df);
    dataframes.insert(
        MessageType::InstantActions.dataframe_name().to_string(),
        ia_df,
    );

    Ok(VdaAnalysisResult {
        dataframes,
        total_chunks,
        num_parsed,
        parse_failures: parse_failures.into_inner().unwrap(),
        parse_examples: parse_examples.into_inner().unwrap(),
        timings: ProcessingTimings {
            mmap_setup,
            delimiter_scanning,
            parsing_and_builder_appends,
            batch_dataframe_construction,
            final_dataframe_concatenation,
        },
    })
}

/// Concatenates multiple DataFrames into one, or returns empty DataFrame if list is empty
fn concatenate_dataframes(dfs: &[DataFrame]) -> Result<DataFrame> {
    if dfs.is_empty() {
        return Ok(DataFrame::empty());
    }

    if dfs.len() == 1 {
        return Ok(dfs[0].clone());
    }

    // Use Polars vstack to vertically stack DataFrames
    let mut result = dfs[0].clone();
    for df in &dfs[1..] {
        result.vstack_mut(df)?;
    }
    Ok(result)
}
