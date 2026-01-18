use crate::AppState;
use dioxus::prelude::*;
use polars::prelude::*;

/// Format numbers with thousands separators for readability
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

#[component]
pub(crate) fn DashboardView() -> Element {
    let mut state = use_context::<AppState>();

    // Get the data or return early if none
    let _ = match &*state.data.read() {
        Some(d) => d,
        None => {
            return rsx! {
                div { class: "view-container",
                    h1 { "Dashboard" }
                    p { "No data loaded. Please open a file first." }
                }
            };
        }
    };
    // Recalculate dashboard stats only once
    // TODO: Some redundancy here, refactor API
    let needs_recalc = state.dashboard_stats.read().is_none();

    if needs_recalc {
        let stats = state
            .data
            .read()
            .as_ref()
            .map(|d| calculate_stats(d))
            .unwrap_or_default();

        state.dashboard_stats.set(Some(stats));
    }
    let s = state.dashboard_stats.read();
    let s = s.as_ref().expect("dashboard_stats not initialized");

    rsx! {
        div { class: "view-container",
            h1 { "Dashboard" }

            // Stats grid
            div { class: "dashboard-grid",
                // Parse Statistics
                StatCard {
                    title: "Total Records".to_string(),
                    value: format_number(s.total_records),
                    subtitle: format!("{} parsed successfully", format_number(s.parsed_records)),
                }

                StatCard {
                    title: "Parse Success Rate".to_string(),
                    value: format!("{:.1}%", s.parse_success_rate),
                    subtitle: format!("{} failures", format_number(s.parse_failures)),
                }

                StatCard {
                    title: "Unique Robots".to_string(),
                    value: format_number(s.unique_robots),
                    subtitle: "manufacturer + serial combinations".to_string(),
                }

                // Message Type Counts
                StatCard {
                    title: "State Messages".to_string(),
                    value: format_number(s.state_count),
                    subtitle: "operating mode & battery data".to_string(),
                }

                StatCard {
                    title: "Visualization Messages".to_string(),
                    value: format_number(s.visualization_count),
                    subtitle: "position & map data".to_string(),
                }

                StatCard {
                    title: "Connection Messages".to_string(),
                    value: format_number(s.connection_count),
                    subtitle: "connection state updates".to_string(),
                }

                StatCard {
                    title: "Order Messages".to_string(),
                    value: format_number(s.order_count),
                    subtitle: "order assignments".to_string(),
                }

                StatCard {
                    title: "Instant Actions".to_string(),
                    value: format_number(s.instant_actions_count),
                    subtitle: "immediate action requests".to_string(),
                }

                // Time Range
                if let Some(time_range) = &s.time_range {
                    StatCard {
                        title: "Time Range".to_string(),
                        value: time_range.clone(),
                        subtitle: format!("duration: {}", s.duration.as_ref().unwrap_or(&"N/A".to_string())),
                    }
                }
            }

            // Failure breakdown if any
            if !s.failure_breakdown.is_empty() {
                div { class: "dashboard-section",
                    h2 { "Parse Failures by Type" }
                    div { class: "failure-grid",
                        for (msg_type, count) in s.failure_breakdown.iter() {
                            div { class: "failure-item",
                                span { class: "failure-type", "{msg_type}" }
                                span { class: "failure-count", "{format_number(*count)} failures" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(title: String, value: String, subtitle: String) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-title", "{title}" }
            div { class: "stat-value", "{value}" }
            div { class: "stat-subtitle", "{subtitle}" }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DashboardStats {
    total_records: usize,
    parsed_records: usize,
    parse_failures: usize,
    parse_success_rate: f64,
    unique_robots: usize,
    state_count: usize,
    visualization_count: usize,
    connection_count: usize,
    order_count: usize,
    instant_actions_count: usize,
    time_range: Option<String>,
    duration: Option<String>,
    failure_breakdown: Vec<(String, usize)>,
}

fn calculate_stats(data: &crate::VdaAnalysisResult) -> DashboardStats {
    let total_records = data.total_chunks;
    let parsed_records = data.num_parsed;
    let parse_failures = data.parse_failures.values().sum::<usize>();
    let parse_success_rate = if total_records > 0 {
        (parsed_records as f64 / total_records as f64) * 100.0
    } else {
        0.0
    };

    // Get unique robots from index dataframe
    let unique_robots = if let Some(index_df) = data.dataframes.get("index") {
        count_unique_robots(index_df)
    } else {
        0
    };

    // Count messages by type
    let state_count = data
        .dataframes
        .get("state")
        .map(|df| df.height())
        .unwrap_or(0);
    let visualization_count = data
        .dataframes
        .get("visualization")
        .map(|df| df.height())
        .unwrap_or(0);
    let connection_count = data
        .dataframes
        .get("connection")
        .map(|df| df.height())
        .unwrap_or(0);
    let order_count = data
        .dataframes
        .get("order")
        .map(|df| df.height())
        .unwrap_or(0);
    let instant_actions_count = data
        .dataframes
        .get("instant_actions")
        .map(|df| df.height())
        .unwrap_or(0);

    // Calculate time range from index dataframe
    let (time_range, duration) = if let Some(index_df) = data.dataframes.get("index") {
        calculate_time_range(index_df)
    } else {
        (None, None)
    };

    // Sort failure breakdown by count descending
    let mut failure_breakdown: Vec<(String, usize)> = data
        .parse_failures
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    failure_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

    DashboardStats {
        total_records,
        parsed_records,
        parse_failures,
        parse_success_rate,
        unique_robots,
        state_count,
        visualization_count,
        connection_count,
        order_count,
        instant_actions_count,
        time_range,
        duration,
        failure_breakdown,
    }
}

fn count_unique_robots(df: &DataFrame) -> usize {
    // Count unique combinations of manufacturer and serial_number
    if let (Ok(_manufacturer_col), Ok(_serial_col)) =
        (df.column("manufacturer"), df.column("serial_number"))
    {
        // Create a new dataframe with just these two columns and get unique rows
        let subset = df
            .select(["manufacturer", "serial_number"])
            .ok()
            .and_then(|df| {
                df.unique_stable(None, UniqueKeepStrategy::First, None::<(i64, usize)>)
                    .ok()
            })
            .map(|df| df.height())
            .unwrap_or(0);

        subset
    } else {
        0
    }
}

fn calculate_time_range(df: &DataFrame) -> (Option<String>, Option<String>) {
    // The index dataframe has a column called "timestamp" with Datetime type
    if let Ok(timestamp_col) = df.column("timestamp") {
        // Cast datetime to its physical representation (i64 nanoseconds)
        if let Ok(i64_series) = timestamp_col.cast(&DataType::Int64) {
            if let Ok(timestamp_series) = i64_series.i64() {
                if let (Some(min_ts), Some(max_ts)) =
                    (timestamp_series.min(), timestamp_series.max())
                {
                    // Timestamps are in nanoseconds
                    let min_dt = chrono::DateTime::from_timestamp_nanos(min_ts);
                    let max_dt = chrono::DateTime::from_timestamp_nanos(max_ts);

                    // Check if same day
                    let same_day = min_dt.date_naive() == max_dt.date_naive();

                    let time_range = if same_day {
                        // Same day: show date once, then time range
                        format!(
                            "{}\n{} → {}",
                            min_dt.format("%Y-%m-%d"),
                            min_dt.format("%H:%M:%S"),
                            max_dt.format("%H:%M:%S")
                        )
                    } else {
                        // Different days: show compact date + time
                        format!(
                            "{}\n→ {}",
                            min_dt.format("%Y-%m-%d %H:%M"),
                            max_dt.format("%Y-%m-%d %H:%M")
                        )
                    };

                    // Calculate duration
                    let duration_secs = (max_ts - min_ts) / 1_000_000_000;
                    let duration = if duration_secs < 60 {
                        Some(format!("{}s", duration_secs))
                    } else if duration_secs < 3600 {
                        Some(format!("{}m {}s", duration_secs / 60, duration_secs % 60))
                    } else if duration_secs < 86400 {
                        Some(format!(
                            "{}h {}m",
                            duration_secs / 3600,
                            (duration_secs % 3600) / 60
                        ))
                    } else {
                        let days = duration_secs / 86400;
                        let hours = (duration_secs % 86400) / 3600;
                        Some(format!("{}d {}h", days, hours))
                    };

                    return (Some(time_range), duration);
                }
            }
        }
    }
    (None, None)
}
