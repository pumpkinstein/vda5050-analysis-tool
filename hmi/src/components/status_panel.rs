use crate::{AppState, ParseStatus};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub(crate) fn StatusPanel(
    log_file_path: Signal<String>,
    file_size: Signal<Option<u64>>,
    process_memory: Signal<u64>,
    cpu_usage: Signal<f32>,
) -> Element {
    // context provider allows for accessing global shared state without explicitly declaring it as signal
    let state = use_context::<AppState>();
    let parse_status = (state.parse_status)();
    let filename = if log_file_path().is_empty() {
        "No file loaded".to_string()
    } else {
        PathBuf::from(log_file_path())
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    };

    let file_size_str = match file_size() {
        Some(size) => format_bytes(size),
        None => "-".to_string(),
    };

    let process_mem_mb = process_memory() as f64 / 1024.0 / 1024.0;
    let cpu_percent = cpu_usage();

    let status_class = match parse_status {
        ParseStatus::Idle => "status-idle",
        ParseStatus::Loading => "status-loading",
        ParseStatus::Loaded => "status-loaded",
        ParseStatus::Error(_) => "status-error",
    };

    rsx! {
        div { class: "status-panel",
            div { class: "status-section",
                div { class: "status-section",
                    span { class: "status-label", "Status:" }
                    span { class: "status-value {status_class}", "{parse_status.as_str()}" }
                }

                div { class: "status-separator" }

                span { class: "status-label", "File:" }
                span {
                    class: "status-value",
                    title: "{log_file_path}",
                    "{filename}"
                }
            }

            div { class: "status-separator" }

            div { class: "status-section",
                span { class: "status-label", "Size:" }
                span { class: "status-value", "{file_size_str}" }
            }

            div { class: "status-separator" }


            div { class: "status-section",
                span { class: "status-label", "Memory:" }
                span { class: "status-value", "{process_mem_mb:.1} MB" }
            }

            div { class: "status-separator" }

            div { class: "status-section",
                span { class: "status-label", "CPU:" }
                span { class: "status-value", "{cpu_percent:.1}%" }
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
