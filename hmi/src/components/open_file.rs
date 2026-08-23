use crate::{AppState, ParseStatus, recent_files};
use dioxus::prelude::*;
use log_file_parser::process_log_file;
use std::path::{Path, PathBuf};
use vda5050_analysis::analyze;

#[component]
pub(crate) fn OpenFileView(
    root_topic: Signal<String>,
    log_file_path: Signal<String>,
    mut file_size: Signal<Option<u64>>,
    recent_file_paths: Signal<Vec<String>>,
) -> Element {
    // Context provider allows for accessing global shared state without explicitly declaring it as signal
    let mut state = use_context::<AppState>();

    // Don't use built-in resource mechanism, this will lead to gnarly memory leaks
    // as it is mixing different ways to deal with lifetimes
    let mut load_task = use_signal(|| None::<dioxus::core::Task>);
    let mut show_recent = use_signal(|| false);

    rsx! {
        div { class: "view-container",
            h1 { "Open Log File" }

            div { class: "form-group",
                label { "Root Topic Name" }
                input {
                    class: "text-input",
                    r#type: "text",
                    placeholder: "e.g., uagv/v1",
                    value: "{root_topic}",
                    oninput: move |evt| root_topic.set(evt.value()),
                }
            }

            div { class: "form-group",
                label { "Log File Path" }
                div { class: "file-input-group",
                    input {
                        class: "text-input",
                        r#type: "text",
                        placeholder: "/path/to/logfile.log",
                        value: "{log_file_path}",
                        oninput: move |evt| {
                            update_file_path(log_file_path, file_size, evt.value());
                        },
                    }
                    button {
                        class: "browse-btn",
                        onclick: move |_| {
                            let initial_directory = Path::new(&log_file_path())
                                .parent()
                                .filter(|directory| {
                                    !directory.as_os_str().is_empty() && directory.is_dir()
                                })
                                .map(PathBuf::from);

                            // File picker using rfd (native file dialog)
                            spawn(async move {
                                let mut dialog = rfd::AsyncFileDialog::new()
                                    .add_filter("Log files", &["log", "txt"]);
                                if let Some(directory) = initial_directory {
                                    dialog = dialog.set_directory(directory);
                                }

                                if let Some(file) = dialog.pick_file().await {
                                    let path = file.path().display().to_string();
                                    update_file_path(log_file_path, file_size, path);
                                }
                            });
                        },
                        "Browse..."
                    }
                    if !recent_file_paths().is_empty() {
                        button {
                            class: "browse-btn",
                            onclick: move |_| show_recent.set(true),
                            "Open Recent"
                        }
                    }
                }
            }

            div {
                style: "display: flex; gap: 8px; align-items: center; margin-top: 24px;",
                button {
                    class: "primary-btn",
                    disabled: root_topic().is_empty() || log_file_path().is_empty(),
                    onclick: move |_| {
                        // cancel previous load if still running
                        if let Some(task) = load_task() {
                            task.cancel();
                        }
                        let path = log_file_path();
                        let root_topic = root_topic();
                        let task = spawn(async move {
                            state.reset();
                            state.parse_status.set(ParseStatus::Loading);

                            let path = Path::new(&path);
                            let batch_size = 4_000;
                            let verbose = false;

                            match process_log_file(&path, &root_topic, batch_size, verbose) {
                                Ok(result) => {
                                    let analysis = analyze(&result);
                                    state.data.set(Some(result));
                                    state.analysis.set(Some(analysis));
                                    state.parse_status.set(ParseStatus::Loaded);

                                    let mut paths = recent_file_paths();
                                    recent_files::remember(&mut paths, path);
                                    recent_file_paths.set(paths);
                                }
                                Err(e) => {
                                    state.data.set(None);
                                    state.parse_status.set(ParseStatus::Error(e.to_string()));
                                }
                            }
                        });
                        load_task.set(Some(task));
                    },
                    "Load File"
                }

                if !log_file_path().is_empty() {
                    button {
                        class: "browse-btn",
                        onclick: move |_| {
                            if let Some(task) = load_task() {
                                task.cancel();
                            }
                            load_task.set(None);
                            state.reset();
                            log_file_path.set("".to_string());
                            file_size.set(None);
                            show_recent.set(false);
                        },
                        "Clear"
                    }
                }
            }

            if show_recent() {
                div { class: "recent-overlay",
                    div { class: "recent-dialog",
                        div { class: "recent-dialog-header",
                            h2 { "Open Recent" }
                            button {
                                class: "dialog-close-btn",
                                onclick: move |_| show_recent.set(false),
                                "Close"
                            }
                        }

                        div { class: "recent-file-list",
                            for path in recent_file_paths() {
                                RecentFileEntry {
                                    path,
                                    log_file_path,
                                    file_size,
                                    show_recent,
                                }
                            }
                        }
                    }
                }
            }

            if let Some(err) = state.error_msg() {
                div {
                    class: "preview-text", // Re-use existing style for a bordered box
                    style: "margin-top: 24px; color: #f87171;", // Red text for error
                    h3 { "Parsing Failed" }
                    p { "{err}" }
                }
            }

            if let Some(res) = &*state.data.read() {
                div {
                    class: "preview-text",
                    style: "margin-top: 24px;",
                    h3 { "Parsing Complete" }
                    p { "Parsed {res.num_parsed} of {res.total_chunks} records." }
                    if !res.parse_failures.is_empty() {
                        p { "Encountered {res.parse_failures.values().sum::<usize>()} failures." }
                    }
                }
            }
        }
    }
}

#[component]
fn RecentFileEntry(
    path: String,
    log_file_path: Signal<String>,
    file_size: Signal<Option<u64>>,
    mut show_recent: Signal<bool>,
) -> Element {
    let display_name = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.clone());
    let display_path = path.clone();

    rsx! {
        button {
            class: "recent-file-item",
            onclick: move |_| {
                update_file_path(log_file_path, file_size, path.clone());
                show_recent.set(false);
            },
            span { class: "recent-file-name", "{display_name}" }
            span { class: "recent-file-path", "{display_path}" }
        }
    }
}

fn update_file_path(
    mut log_file_path: Signal<String>,
    mut file_size: Signal<Option<u64>>,
    path: String,
) {
    file_size.set(std::fs::metadata(&path).ok().map(|metadata| metadata.len()));
    log_file_path.set(path);
}
