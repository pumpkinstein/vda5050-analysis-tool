use dioxus::prelude::*;

#[component]
pub(crate) fn SettingsView(font_size: Signal<i32>, icon_size: Signal<u32>) -> Element {
    rsx! {
        div { class: "view-container",
            h1 { "Settings" }

            div { class: "form-group",
                label { "Font Size: {font_size}px" }
                input {
                    class: "slider",
                    r#type: "range",
                    min: 10,
                    max: 24,
                    value: "{font_size}",
                    oninput: move |evt| {
                        if let Ok(size) = evt.value().parse::<i32>() {
                            font_size.set(size);
                        }
                    },
                }
                div { class: "slider-labels",
                    span { "10px" }
                    span { "24px" }
                }
            }

            div { class: "form-group",
                label { "Icon Size: {icon_size}px" }
                input {
                    class: "slider",
                    r#type: "range",
                    min: 16,
                    max: 48,
                    value: "{icon_size}",
                    oninput: move |evt| {
                        if let Ok(size) = evt.value().parse::<u32>() {
                            icon_size.set(size);
                        }
                    },
                }
                div { class: "slider-labels",
                    span { "16px" }
                    span { "48px" }
                }
            }

            div { class: "preview-text",
                "Preview: The quick brown fox jumps over the lazy dog"
            }
        }
    }
}
