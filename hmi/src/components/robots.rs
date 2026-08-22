use dioxus::prelude::*;
use vda5050_analysis::{AnalysisSnapshot, RobotIdentity};

#[component]
pub(crate) fn RobotsView(
    analysis: Signal<Option<AnalysisSnapshot>>,
    strict_robot_ordering: Signal<bool>,
) -> Element {
    let snapshot = analysis.read();
    let Some(snapshot) = snapshot.as_ref() else {
        return rsx! {
            div { class: "view-container",
                h1 { "Robots" }
                p { "No data loaded. Please open a file first." }
            }
        };
    };

    let mut identities: Vec<_> = snapshot.robot_identities.iter().collect();
    let strict_alphabetical = strict_robot_ordering();
    sort_robot_identities(&mut identities, strict_alphabetical);

    if identities.is_empty() {
        return rsx! {
            div { class: "view-container",
                h1 { "Robots" }
                p { "No usable robot identities found in the loaded analysis." }
            }
        };
    }

    rsx! {
        div { class: "view-container",
            h1 { "Robots" }
            div { class: "dashboard-grid",
                for identity in identities.iter() {
                    RobotStatCard {
                        manufacturer: identity.manufacturer.clone(),
                        serial_number: identity.serial_number.clone(),
                    }
                }
            }
        }
    }
}

fn sort_robot_identities(identities: &mut Vec<&RobotIdentity>, strict_alphabetical: bool) {
    if strict_alphabetical {
        identities.sort_unstable_by(|left, right| {
            left.manufacturer
                .cmp(&right.manufacturer)
                .then_with(|| left.serial_number.cmp(&right.serial_number))
        });
        return;
    }

    let serial_width = identities
        .iter()
        .map(|identity| split_numeric_suffix(&identity.serial_number).1.len())
        .max()
        .unwrap_or_default();

    identities.sort_by_cached_key(|identity| {
        (
            identity.manufacturer.clone(),
            padded_serial_number(&identity.serial_number, serial_width),
            // Padding can make values such as "2" and "02" share a key. Use the
            // un-padded serial number as a tie-breaker to avoid this issue.
            identity.serial_number.clone(),
        )
    });
}

fn padded_serial_number(value: &str, width: usize) -> String {
    let (prefix, suffix) = split_numeric_suffix(value);

    if suffix.is_empty() {
        return value.to_owned();
    }

    format!("{prefix}{suffix:0>width$}", width = width)
}

fn split_numeric_suffix(value: &str) -> (&str, &str) {
    let Some((index, character)) = value
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_digit())
    else {
        return ("", value);
    };

    let suffix_start = index + character.len_utf8();
    (&value[..suffix_start], &value[suffix_start..])
}

#[component]
fn RobotStatCard(manufacturer: String, serial_number: String) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-title", "Robot" }
            div { class: "stat-value", "{manufacturer}" }
            div { class: "stat-subtitle", "Serial number: {serial_number}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(serial_number: &str) -> RobotIdentity {
        RobotIdentity {
            manufacturer: "maker".to_string(),
            serial_number: serial_number.to_string(),
        }
    }

    #[test]
    fn padded_serial_numbers_use_the_requested_width() {
        assert_eq!(padded_serial_number("1", 2), "01");
        assert_eq!(padded_serial_number("10", 2), "10");
    }

    #[test]
    fn padded_sort_compares_numeric_serials_naturally() {
        let identities = [identity("1"), identity("10"), identity("2")];
        let mut sorted: Vec<_> = identities.iter().collect();

        sort_robot_identities(&mut sorted, false);

        let serial_numbers: Vec<_> = sorted
            .iter()
            .map(|identity| identity.serial_number.as_str())
            .collect();
        assert_eq!(serial_numbers, vec!["1", "2", "10"]);
    }

    #[test]
    fn strict_ordering_preserves_lexical_serial_order() {
        let identities = [identity("1"), identity("10"), identity("2")];
        let mut sorted: Vec<_> = identities.iter().collect();

        sort_robot_identities(&mut sorted, true);

        let serial_numbers: Vec<_> = sorted
            .iter()
            .map(|identity| identity.serial_number.as_str())
            .collect();
        assert_eq!(serial_numbers, vec!["1", "10", "2"]);
    }

    #[test]
    fn natural_ordering_supports_serial_prefixes() {
        assert_eq!(padded_serial_number("robot-2", 2), "robot-02");
        assert_eq!(padded_serial_number("robot-10", 2), "robot-10");
    }
}
