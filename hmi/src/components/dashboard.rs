use dioxus::prelude::*;
use vda5050_analysis::{AnalysisSnapshot, TimeRange};

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
pub(crate) fn DashboardView(analysis: Signal<Option<AnalysisSnapshot>>) -> Element {
    let snapshot = analysis.read();

    let Some(snapshot) = snapshot.as_ref() else {
        return rsx! {
            div { class: "view-container",
                h1 { "Dashboard" }
                p { "No data loaded. Please open a file first." }
            }
        };
    };

    let s = &snapshot.summary;

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
                    value: format_number(s.message_counts.state),
                    subtitle: "operating mode & battery data".to_string(),
                }

                StatCard {
                    title: "Visualization Messages".to_string(),
                    value: format_number(s.message_counts.visualization),
                    subtitle: "position & map data".to_string(),
                }

                StatCard {
                    title: "Connection Messages".to_string(),
                    value: format_number(s.message_counts.connection),
                    subtitle: "connection state updates".to_string(),
                }

                StatCard {
                    title: "Order Messages".to_string(),
                    value: format_number(s.message_counts.order),
                    subtitle: "order assignments".to_string(),
                }

                StatCard {
                    title: "Instant Actions".to_string(),
                    value: format_number(s.message_counts.instant_actions),
                    subtitle: "immediate action requests".to_string(),
                }

                // Time Range
                if let Some(time_range) = &s.time_range {
                    StatCard {
                        title: "Time Range".to_string(),
                        value: format_time_range(time_range),
                        subtitle: format!("duration: {}", format_duration(time_range)),
                    }
                }
            }

            // Failure breakdown if any
            if !s.failure_breakdown.is_empty() {
                div { class: "dashboard-section",
                    h2 { "Parse Failures by Type" }
                    div { class: "failure-grid",
                        for failure in s.failure_breakdown.iter() {
                            div { class: "failure-item",
                                span { class: "failure-type", "{failure.message_type}" }
                                span { class: "failure-count", "{format_number(failure.count)} failures" }
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

fn format_time_range(time_range: &TimeRange) -> String {
    // Keep the existing dashboard display format in the HMI while the shared
    // analysis crate retains raw UTC values.
    let same_day = time_range.start.date_naive() == time_range.end.date_naive();

    if same_day {
        format!(
            "{}\n{} → {}",
            time_range.start.format("%Y-%m-%d"),
            time_range.start.format("%H:%M:%S"),
            time_range.end.format("%H:%M:%S")
        )
    } else {
        format!(
            "{}\n→ {}",
            time_range.start.format("%Y-%m-%d %H:%M"),
            time_range.end.format("%Y-%m-%d %H:%M")
        )
    }
}

fn format_duration(time_range: &TimeRange) -> String {
    let duration_secs = time_range.duration.num_seconds();

    if duration_secs < 60 {
        format!("{}s", duration_secs)
    } else if duration_secs < 3600 {
        format!("{}m {}s", duration_secs / 60, duration_secs % 60)
    } else if duration_secs < 86400 {
        format!("{}h {}m", duration_secs / 3600, (duration_secs % 3600) / 60)
    } else {
        let days = duration_secs / 86400;
        let hours = (duration_secs % 86400) / 3600;
        format!("{}d {}h", days, hours)
    }
}
