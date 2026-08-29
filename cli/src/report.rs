use crate::VISUALIZATION_CONTEXT_SAMPLE_LIMIT;
use anyhow::{Context, Result};
use log_file_parser::{MessageType, VdaAnalysisResult};
use std::{io::Write, time::Duration};
use vda5050_analysis::{
    canonical_frame_counts, failure_breakdown, message_type_distribution, record_counts,
    visualization_context_sample,
};

pub fn write_report<W: Write>(
    writer: &mut W,
    result: &VdaAnalysisResult,
    verbose: bool,
    processing_duration: Duration,
) -> Result<()> {
    let record_counts = record_counts(result);
    let frame_counts = canonical_frame_counts(result);

    writeln!(
        writer,
        "Processed {} records in {:.2?}. (Ignored {} entries)",
        record_counts.parsed_records, processing_duration, record_counts.ignored_records
    )?;

    writeln!(writer, "Ingestion timing:")?;
    writeln!(writer, "  mmap setup: {:.2?}", result.timings.mmap_setup)?;
    writeln!(
        writer,
        "  delimiter scanning: {:.2?}",
        result.timings.delimiter_scanning
    )?;
    writeln!(
        writer,
        "  JSON parsing and builder appends: {:.2?}",
        result.timings.parsing_and_builder_appends
    )?;
    writeln!(
        writer,
        "  per-batch DataFrame construction: {:.2?}",
        result.timings.batch_dataframe_construction
    )?;
    writeln!(
        writer,
        "  final DataFrame concatenation: {:.2?}",
        result.timings.final_dataframe_concatenation
    )?;

    if record_counts.ignored_records > 0 {
        let failures = failure_breakdown(result);
        if !failures.is_empty() {
            writeln!(writer, "\nParse failure summary:")?;
            for failure in failures {
                writeln!(
                    writer,
                    "  {}: {} messages failed (missing required fields)",
                    failure.message_type, failure.count
                )?;
            }

            if verbose {
                writeln!(writer, "\nExample parse failures:")?;
                for (msg_type, example) in &result.parse_examples {
                    writeln!(writer, "\n  Message type: {}", msg_type)?;
                    writeln!(writer, "  {}", example)?;
                }
            }
        }
    }

    let index_df = result
        .dataframes
        .get("index")
        .with_context(|| "parser result is missing the index DataFrame")?;
    let state_df = result
        .dataframes
        .get(MessageType::State.dataframe_name())
        .with_context(|| "parser result is missing the state DataFrame")?;
    let viz_df = result
        .dataframes
        .get(MessageType::Visualization.dataframe_name())
        .with_context(|| "parser result is missing the visualization DataFrame")?;
    let conn_df = result
        .dataframes
        .get(MessageType::Connection.dataframe_name())
        .with_context(|| "parser result is missing the connection DataFrame")?;
    let order_df = result
        .dataframes
        .get(MessageType::Order.dataframe_name())
        .with_context(|| "parser result is missing the order DataFrame")?;
    let ia_df = result
        .dataframes
        .get(MessageType::InstantActions.dataframe_name())
        .with_context(|| "parser result is missing the instant_actions DataFrame")?;

    writeln!(writer, "\n=== Index DataFrame ===")?;
    writeln!(writer, "Total records: {}", frame_counts.index)?;
    writeln!(writer, "Schema: {:?}", index_df.schema())?;
    writeln!(writer, "Sample:")?;
    writeln!(writer, "{}", index_df.head(Some(5)))?;

    writeln!(writer, "\n=== State DataFrame ===")?;
    writeln!(writer, "Records: {}", frame_counts.messages.state)?;
    writeln!(writer, "Schema: {:?}", state_df.schema())?;
    if frame_counts.messages.state > 0 {
        writeln!(writer, "Sample:")?;
        writeln!(writer, "{}", state_df.head(Some(5)))?;
    }

    writeln!(writer, "\n=== Visualization DataFrame ===")?;
    writeln!(writer, "Records: {}", frame_counts.messages.visualization)?;
    writeln!(writer, "Schema: {:?}", viz_df.schema())?;
    if frame_counts.messages.visualization > 0 {
        writeln!(writer, "Sample:")?;
        writeln!(writer, "{}", viz_df.head(Some(5)))?;
    }

    writeln!(writer, "\n=== Connection DataFrame ===")?;
    writeln!(writer, "Records: {}", frame_counts.messages.connection)?;
    writeln!(writer, "Schema: {:?}", conn_df.schema())?;
    if frame_counts.messages.connection > 0 {
        writeln!(writer, "Sample:")?;
        writeln!(writer, "{}", conn_df.head(Some(5)))?;
    }

    writeln!(writer, "\n=== Order DataFrame ===")?;
    writeln!(writer, "Records: {}", frame_counts.messages.order)?;
    writeln!(writer, "Schema: {:?}", order_df.schema())?;
    if frame_counts.messages.order > 0 {
        writeln!(writer, "Sample:")?;
        writeln!(writer, "{}", order_df.head(Some(5)))?;
    }

    writeln!(writer, "\n=== InstantActions DataFrame ===")?;
    writeln!(writer, "Records: {}", frame_counts.messages.instant_actions)?;
    writeln!(writer, "Schema: {:?}", ia_df.schema())?;
    if frame_counts.messages.instant_actions > 0 {
        writeln!(writer, "Sample:")?;
        writeln!(writer, "{}", ia_df.head(Some(5)))?;
    }

    writeln!(writer, "\n=== Example: Cross-Message Query ===")?;
    writeln!(
        writer,
        "Join visualization data with index to get manufacturer + position:"
    )?;

    if frame_counts.messages.visualization > 0 {
        let sample =
            visualization_context_sample(index_df, viz_df, VISUALIZATION_CONTEXT_SAMPLE_LIMIT)?;
        writeln!(
            writer,
            "Sample of joined data (manufacturer + timestamp + position):"
        )?;
        writeln!(writer, "{}", sample)?;
    } else {
        writeln!(writer, "No visualization data to demonstrate join.")?;
    }

    writeln!(writer, "\n=== Summary Statistics ===")?;
    writeln!(writer, "Message type distribution:")?;
    writeln!(writer, "{}", message_type_distribution(index_df)?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_BATCH_SIZE;
    use log_file_parser::{DEFAULT_ROOT_TOPIC, process_log_file};
    use std::{path::Path, time::Duration};

    fn sample_result() -> Result<VdaAnalysisResult> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../log-file-parser/test-data/sample.log");
        process_log_file(&path, DEFAULT_ROOT_TOPIC, DEFAULT_BATCH_SIZE, true)
    }

    #[test]
    fn sample_report_preserves_sections_counts_and_queries() -> Result<()> {
        let result = sample_result()?;
        let mut output = Vec::new();
        write_report(&mut output, &result, true, Duration::from_millis(12))?;
        let output = String::from_utf8(output)?;

        let headings = [
            "=== Index DataFrame ===",
            "=== State DataFrame ===",
            "=== Visualization DataFrame ===",
            "=== Connection DataFrame ===",
            "=== Order DataFrame ===",
            "=== InstantActions DataFrame ===",
            "=== Example: Cross-Message Query ===",
            "=== Summary Statistics ===",
        ];
        let positions: Vec<_> = headings
            .iter()
            .map(|heading| output.find(heading))
            .collect();
        assert!(positions.windows(2).all(|window| match window {
            [Some(previous), Some(current)] => previous < current,
            _ => false,
        }));

        assert!(output.contains("Processed 4 records in 12.00ms. (Ignored 2 entries)"));
        assert!(output.contains("Total records: 4"));
        assert!(output.contains("Records: 2"));
        assert!(output.contains("Records: 1"));
        assert!(output.contains("state: 1 messages failed"));
        assert!(output.contains("visualization: 1 messages failed"));
        assert!(output.contains("Message type: state"));
        assert!(output.contains("Message type: visualization"));
        assert!(output.contains("manufacturer"));
        assert!(output.contains("robot-inc"));
        assert!(output.contains("msg_type"));
        assert!(output.contains("count"));
        assert!(output.contains("state         ┆ 2"));
        assert!(output.contains("visualization ┆ 1"));

        let order_start = output
            .find("=== Order DataFrame ===")
            .context("missing Order section")?;
        let instant_start = output
            .find("=== InstantActions DataFrame ===")
            .context("missing InstantActions section")?;
        let query_start = output
            .find("=== Example: Cross-Message Query ===")
            .context("missing query section")?;
        assert!(!output[order_start..instant_start].contains("Sample:"));
        assert!(!output[instant_start..query_start].contains("Sample:"));
        Ok(())
    }
}
