pub mod builders;
pub mod io;
pub mod models;
pub mod process;

use crate::{
    builders::AllBuilders,
    io::{VdaIterator, parse_version},
    models::ParsedMessage,
    process::parse_record,
};
use anyhow::Result;
use polars::prelude::*;
use rayon::prelude::*;
use std::{collections::HashMap, fs::File, path::Path, sync::Mutex};

pub struct VdaAnalysisResult {
    pub dataframes: HashMap<String, DataFrame>,
    pub total_chunks: usize,
    pub num_parsed: usize,
    pub parse_failures: HashMap<String, usize>,
    pub parse_examples: HashMap<String, String>,
}

/// Processes a VDA5050 log file and returns dataframes for each message type.
pub fn process_log_file(
    file_path: &Path,
    batch_size: usize,
    verbose: bool,
) -> Result<VdaAnalysisResult> {
    // 1. Memory-map the file for zero-copy reads.
    let file = File::open(file_path)?;
    // SAFETY: The file is read-only, and we are the only process accessing it.
    // The file will not be modified while the mmap is active.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };

    // 2. Use the custom iterator to find all VDA 5050 message chunks.
    let iterator = VdaIterator::new(&mmap);
    let chunks: Vec<_> = iterator.collect();
    let total_chunks = chunks.len();

    // 3. Process chunks in parallel batches using Rayon with per-thread builders
    // Track parse failures by message type
    let parse_failures: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    let parse_examples: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());

    // Split work into batches for parallel processing
    let num_batches = (total_chunks + batch_size - 1) / batch_size;

    // Collect DataFrames from each batch
    let batch_results: Vec<_> = (0..num_batches)
        .into_par_iter()
        .map(|batch_idx| {
            let start_idx = batch_idx * batch_size;
            let end_idx = (start_idx + batch_size).min(total_chunks);
            let batch_chunks = &chunks[start_idx..end_idx];

            // Per-thread builders - column-wise append
            let mut builders = AllBuilders::with_capacity(batch_size);
            let mut row_id_offset = start_idx as u64;

            for chunk in batch_chunks {
                match parse_record(chunk) {
                    Ok(message) => {
                        let topic = message.topic();
                        let msg_type = message.msg_type();

                        // Append to builders based on message type (column-wise, no row objects)
                        match &message {
                            ParsedMessage::State { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.state.append(
                                    row_id_offset,
                                    format!("{:?}", data.operating_mode).to_uppercase(),
                                    data.battery_state.battery_charge,
                                    !data.errors.is_empty(),
                                );
                            }
                            ParsedMessage::Visualization { data, .. } => {
                                let manufacturer = data
                                    .manufacturer
                                    .clone()
                                    .unwrap_or_else(|| topic.manufacturer.clone());
                                let serial_number = data
                                    .serial_number
                                    .clone()
                                    .unwrap_or_else(|| topic.serial_number.clone());

                                builders.index.append(
                                    row_id_offset,
                                    manufacturer,
                                    serial_number,
                                    msg_type.to_string(),
                                    data.header_id.unwrap(),
                                    data.timestamp.unwrap() * 1000, // Convert µs to ns
                                    parse_version(data.version.as_ref().unwrap()),
                                );

                                // Extract position data if available
                                let (x, y, theta, map_id) = if let Some(pos) = &data.agv_position {
                                    (
                                        Some(pos.x),
                                        Some(pos.y),
                                        Some(pos.theta),
                                        Some(pos.map_id.clone()),
                                    )
                                } else {
                                    (None, None, None, None)
                                };

                                builders
                                    .visualization
                                    .append(row_id_offset, x, y, theta, map_id);
                            }
                            ParsedMessage::Connection { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.connection.append(
                                    row_id_offset,
                                    format!("{:?}", data.connection_state).to_uppercase(),
                                );
                            }
                            ParsedMessage::Order { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

                                builders.order.append(row_id_offset, data.order_id.clone());
                            }
                            ParsedMessage::InstantActions { data, .. } => {
                                builders.index.append(
                                    row_id_offset,
                                    topic.manufacturer.clone(),
                                    topic.serial_number.clone(),
                                    msg_type.to_string(),
                                    data.header.header_id,
                                    data.header.timestamp * 1000, // Convert µs to ns
                                    parse_version(&data.header.version),
                                );

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
                        if let Some(start) = chunk_str.find("uagv/v1/") {
                            let topic_part = &chunk_str[start + 8..];
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

            // Finish builders for this batch and return DataFrames
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

    // 4. Concatenate batches into final DataFrames
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

    let num_parsed = index_df.height();

    let mut dataframes = HashMap::new();
    dataframes.insert("index".to_string(), index_df);
    dataframes.insert("state".to_string(), state_df);
    dataframes.insert("visualization".to_string(), viz_df);
    dataframes.insert("connection".to_string(), conn_df);
    dataframes.insert("order".to_string(), order_df);
    dataframes.insert("instant_actions".to_string(), ia_df);

    Ok(VdaAnalysisResult {
        dataframes,
        total_chunks,
        num_parsed,
        parse_failures: parse_failures.into_inner().unwrap(),
        parse_examples: parse_examples.into_inner().unwrap(),
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
