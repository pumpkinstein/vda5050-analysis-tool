use clap::Parser;
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::bs_icons::{BsFileEarmarkText, BsGear, BsRobot, BsSpeedometer2};
use log_file_parser::{DEFAULT_ROOT_TOPIC, VdaAnalysisResult};
use std::path::PathBuf;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use vda5050_analysis::AnalysisSnapshot;

mod components;
mod recent_files;
use components::{
    dashboard::DashboardView, open_file::OpenFileView, robots::RobotsView, settings::SettingsView,
    status_panel::StatusPanel,
};

/// Shared state between many components. Uses Dioxus context() mechanism
#[derive(Debug, Clone, Default, Copy)]
pub(crate) struct AppState {
    /// Data frames from parsing log files plus some metadata
    pub(crate) data: Signal<Option<VdaAnalysisResult>>,
    pub(crate) parse_status: Signal<ParseStatus>,
    /// Shared dashboard and robots analysis derived when a file is loaded
    pub(crate) analysis: Signal<Option<AnalysisSnapshot>>,
}

/// Implements some helper functions mostly to deal with epic &* syntax
/// Which dereferences guard returned by read() so you can match on the actual ParseStatus enum.
/// Otherwise this will be cluttered all over components
impl AppState {
    /// Returns the error message if the status is Error
    pub fn error_msg(&self) -> Option<String> {
        if let ParseStatus::Error(err) = &*self.parse_status.read() {
            Some(err.clone())
        } else {
            None
        }
    }

    /// Bunch of internal state that has be reset when loading a new file
    pub fn reset(&mut self) {
        self.data.set(None);
        self.parse_status.set(ParseStatus::Idle);
        self.analysis.set(None);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum View {
    OpenFile,
    Dashboard,
    Robots,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum ParseStatus {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

impl ParseStatus {
    fn as_str(&self) -> &str {
        match self {
            ParseStatus::Idle => "Ready",
            ParseStatus::Loading => "Parsing...",
            ParseStatus::Loaded => "Loaded",
            ParseStatus::Error(_) => "Error",
        }
    }
}

const STYLE_CSS: Asset = asset!("/assets/style.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const PUMPKINSTEIN: Asset = asset!("/assets/pumpkinstein.png");
#[cfg(feature = "desktop")]
const WINDOW_ICON: &[u8] = include_bytes!("../assets/pumpkinstein.png");
#[cfg(all(feature = "desktop", target_os = "linux"))]
const LINUX_APPLICATION_ID: &str = "com.pumpkinstein.vda5050analysis";

#[derive(Debug, Parser)]
#[command(author, version, about = "VDA 5050 log analysis HMI")]
struct Args {
    /// Path to the VDA 5050 log file to prefill
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Root MQTT topic used by the VDA 5050 messages
    #[arg(long, default_value = DEFAULT_ROOT_TOPIC)]
    root_topic: String,
}

#[derive(Clone, Debug)]
struct StartupArgs {
    file: Option<PathBuf>,
    root_topic: String,
}

fn main() {
    #[cfg(all(feature = "desktop", target_os = "linux"))]
    gtk::glib::set_prgname(Some(LINUX_APPLICATION_ID));

    let args = Args::parse();

    let launcher = dioxus::LaunchBuilder::new().with_context(StartupArgs {
        file: args.file,
        root_topic: args.root_topic,
    });

    #[cfg(feature = "desktop")]
    let launcher = launcher.with_cfg(desktop_config());

    launcher.launch(App);
}

#[cfg(feature = "desktop")]
fn desktop_config() -> dioxus::desktop::Config {
    let icon = dioxus::desktop::icon_from_memory::<dioxus::desktop::tao::window::Icon>(WINDOW_ICON)
        .expect("failed to load window icon");

    let config = dioxus::desktop::Config::new()
        .with_window(dioxus::desktop::WindowBuilder::new().with_title("VDA5050 Analysis Tool"))
        .with_icon(icon);

    #[cfg(target_os = "linux")]
    let config = config.with_on_window(|window, _| set_linux_application_id(&window));

    #[cfg(target_os = "linux")]
    {
        use dioxus::desktop::tao::platform::unix::EventLoopBuilderExtUnix;

        let mut event_loop = dioxus::desktop::tao::event_loop::EventLoopBuilder::with_user_event();
        event_loop.with_app_id(LINUX_APPLICATION_ID);

        return config.with_event_loop(event_loop.build());
    }

    #[cfg(not(target_os = "linux"))]
    {
        config
    }
}

#[cfg(all(feature = "desktop", target_os = "linux"))]
fn set_linux_application_id(window: &std::sync::Arc<dioxus::desktop::tao::window::Window>) {
    use dioxus::desktop::tao::platform::unix::WindowExtUnix;
    use gtk::gdk::prelude::DisplayExtManual;
    use gtk::glib::translate::ToGlibPtr;
    use gtk::prelude::WidgetExt;
    use std::ffi::CString;

    let gtk_window = window.gtk_window();
    let Some(gdk_window) = gtk_window.window() else {
        return;
    };

    if !gdk_window.display().backend().is_wayland() {
        return;
    }

    let application_id = CString::new(LINUX_APPLICATION_ID)
        .expect("Linux application ID must not contain a NUL byte");

    // Tao/GTK can initially derive the Wayland surface ID from the executable
    // name. Set the intended ID on the surface that the compositor sees as a
    // fallback for GTK versions where the initial ID is still incorrect.
    let gdk_window_ptr: *mut gtk::gdk::ffi::GdkWindow = gdk_window.to_glib_none().0;
    unsafe {
        gdk_wayland_sys::gdk_wayland_window_set_application_id(
            gdk_window_ptr.cast(),
            application_id.as_ptr(),
        );
    }
}

#[component]
fn App() -> Element {
    let startup_args = use_context::<StartupArgs>();
    let recent_file_paths = use_signal(recent_files::load);
    let initial_file_path = startup_args
        .file
        .as_ref()
        .map(|path| path.display().to_string())
        .or_else(|| recent_file_paths().first().cloned())
        .unwrap_or_default();
    let initial_file_size = if initial_file_path.is_empty() {
        None
    } else {
        std::fs::metadata(&initial_file_path)
            .ok()
            .map(|metadata| metadata.len())
    };
    let initial_root_topic = startup_args.root_topic.clone();

    let mut current_view = use_signal(|| View::OpenFile);
    let font_size = use_signal(|| 14);
    let icon_size = use_signal(|| 24);
    let strict_robot_ordering = use_signal(|| false);
    let root_topic = use_signal(move || initial_root_topic);
    let log_file_path = use_signal(move || initial_file_path);
    let file_size = use_signal(move || initial_file_size);
    let mut process_memory = use_signal(|| 0u64);
    let mut cpu_usage = use_signal(|| 0.0f32);

    // Initialize global state at root
    let app_state = use_context_provider(|| AppState::default());

    // Share the load-time analysis between the dashboard and robots views.
    let analysis = app_state.analysis;

    // Check if data is loaded
    let data_loaded =
        use_memo(move || matches!(&*app_state.parse_status.read(), ParseStatus::Loaded));

    // Memory and CPU monitoring effect - updates every second
    use_effect(move || {
        spawn(async move {
            let mut sys = System::new();
            let pid = sysinfo::get_current_pid().ok();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // Only refresh the current process instead of all processes
                if let Some(pid) = pid {
                    sys.refresh_processes_specifics(
                        ProcessesToUpdate::Some(&[pid]), // Only refresh specific PIDs
                        true,
                        ProcessRefreshKind::everything(),
                    );
                    if let Some(process) = sys.process(pid) {
                        process_memory.set(process.memory());
                        cpu_usage.set(process.cpu_usage());
                    }
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: PUMPKINSTEIN }
        document::Link { rel: "stylesheet", href: STYLE_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div { class: "app-container",
            // Left sidebar
            div { class: "sidebar",
                button {
                    class: if current_view() == View::OpenFile { "sidebar-btn active" } else { "sidebar-btn" },
                    onclick: move |_| current_view.set(View::OpenFile),
                    Icon {
                        icon: BsFileEarmarkText,
                        width: icon_size(),
                        height: icon_size(),
                    }
                    span { "Open File" }
                }

                // Dashboard button - only shown when data is loaded
                if data_loaded() {
                    button {
                        class: if current_view() == View::Dashboard { "sidebar-btn active" } else { "sidebar-btn" },
                        onclick: move |_| current_view.set(View::Dashboard),
                        Icon {
                            icon: BsSpeedometer2,
                            width: icon_size(),
                            height: icon_size(),
                        }
                        span { "Dashboard" }
                    }

                    button {
                        class: if current_view() == View::Robots { "sidebar-btn active" } else { "sidebar-btn" },
                        onclick: move |_| current_view.set(View::Robots),
                        Icon {
                            icon: BsRobot,
                            width: icon_size(),
                            height: icon_size(),
                        }
                        span { "Robots" }
                    }
                }

                // Spacer pushes everything below to the bottom
                div { class: "spacer" }

                button {
                    class: if current_view() == View::Settings { "sidebar-btn active" } else { "sidebar-btn" },
                    onclick: move |_| current_view.set(View::Settings),
                    Icon {
                        icon: BsGear,
                        width: icon_size(),
                        height: icon_size(),
                    }
                    span { "Settings" }
                }
            }

            // Main content area with working area and status panel
            div { class: "main-content",
                // Working area
                div {
                    class: "working-area",
                    style: "font-size: {font_size}px;",

                    match current_view() {
                        View::OpenFile => rsx! {
                            OpenFileView {
                                root_topic,
                                log_file_path,
                                file_size,
                                recent_file_paths,
                            }
                        },
                        View::Dashboard => rsx! {
                            DashboardView { analysis }
                        },
                        View::Robots => rsx! {
                            RobotsView {
                                analysis,
                                strict_robot_ordering,
                            }
                        },
                        View::Settings => rsx! {
                            SettingsView {
                                font_size,
                                icon_size,
                                strict_robot_ordering,
                            }
                        },
                    }
                }

                // Status panel
                StatusPanel {
                    log_file_path,
                    file_size,
                    process_memory,
                    cpu_usage,
                }
            }
        }
    }
}
