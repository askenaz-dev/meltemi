// SPDX-License-Identifier: Apache-2.0

//! Session configuration options: what the agent announced, translated into the
//! contract, and a chosen value checked back against that announcement.
//!
//! Meltemi contributes nothing to this list. The options are the AGENT's, they
//! arrive with the response to `session/new`, and this module only carries them
//! across the contract boundary and validates a choice against them. An
//! embedded catalog of models here would be the core assuming a provider (§5)
//! and would rot silently with every model released
//! (modelo-y-esfuerzo-por-sesion design D1).

use agent_client_protocol::schema::v1::{
    SessionConfigKind as AcpKind, SessionConfigOption as AcpOption,
    SessionConfigOptionValue as AcpValue, SessionConfigSelectOption, SessionConfigSelectOptions,
};
use meltemi_proto::{SessionConfigKind, SessionConfigOption, SessionConfigValue};

/// Translates what the agent announced into the contract's shape.
///
/// Groups are flattened: ACP lets an agent group a selector's values under
/// headers, and the contract carries a flat list. The grouping is presentation,
/// and dropping it loses no value — every option keeps its id, which is the only
/// thing that has to survive to make a choice.
#[must_use]
pub fn from_acp(options: &[AcpOption]) -> Vec<SessionConfigOption> {
    options.iter().map(one_from_acp).collect()
}

fn one_from_acp(option: &AcpOption) -> SessionConfigOption {
    SessionConfigOption {
        id: option.id.0.to_string(),
        name: option.name.clone(),
        description: option.description.clone(),
        // Serialized rather than matched: ACP declares the category enum
        // non-exhaustive and requires clients to handle categories they do not
        // know, so the wire form is carried through instead of being narrowed
        // into a set that the next spec revision would break.
        category: option.category.as_ref().and_then(|category| {
            serde_json::to_value(category)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
        }),
        kind: match &option.kind {
            AcpKind::Select(select) => SessionConfigKind::Select {
                current_value: select.current_value.0.to_string(),
                values: flatten(&select.options),
            },
            AcpKind::Boolean(toggle) => SessionConfigKind::Boolean {
                current_value: toggle.current_value,
            },
            // ACP marks this enum non-exhaustive. A kind this build does not
            // know is announced as a selector with nothing to select, which is
            // what a surface must render as "not offered" — never as a control
            // that would send a value the agent never described.
            _ => SessionConfigKind::Select {
                current_value: String::new(),
                values: Vec::new(),
            },
        },
    }
}

fn flatten(options: &SessionConfigSelectOptions) -> Vec<SessionConfigValue> {
    match options {
        SessionConfigSelectOptions::Ungrouped(values) => {
            values.iter().map(value_from_acp).collect()
        }
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| group.options.iter().map(value_from_acp))
            .collect(),
        _ => Vec::new(),
    }
}

fn value_from_acp(value: &SessionConfigSelectOption) -> SessionConfigValue {
    SessionConfigValue {
        id: value.value.0.to_string(),
        name: value.name.clone(),
        description: value.description.clone(),
    }
}

/// Reads a caller's chosen value against the kind the AGENT announced for that
/// option, and refuses anything the announcement does not cover.
///
/// One string arrives on the wire and the announcement is what disambiguates
/// it: a select accepts only a value it listed, and a boolean accepts only the
/// two words. A select that happens to list a value called `true` is therefore
/// still resolved as a select. The alternative — trusting the caller to say
/// which kind it meant — would let a surface send a value this agent never
/// described, which is the whole thing this path exists to prevent.
///
/// # Errors
///
/// Returns the diagnostic to show when the value does not match the
/// announcement: the offending value and what the option actually accepts.
pub fn chosen_value(option: &SessionConfigOption, value: &str) -> Result<AcpValue, String> {
    match &option.kind {
        SessionConfigKind::Select {
            values,
            current_value: _,
        } => {
            if values.iter().any(|candidate| candidate.id == value) {
                Ok(AcpValue::value_id(value.to_string()))
            } else if values.is_empty() {
                Err(format!(
                    "the agent announced `{}` without any selectable value",
                    option.id
                ))
            } else {
                let announced: Vec<&str> = values
                    .iter()
                    .map(|candidate| candidate.id.as_str())
                    .collect();
                Err(format!(
                    "`{value}` is not one of the values the agent announced for `{}` ({})",
                    option.id,
                    announced.join(", ")
                ))
            }
        }
        SessionConfigKind::Boolean { .. } => match value {
            "true" => Ok(AcpValue::boolean(true)),
            "false" => Ok(AcpValue::boolean(false)),
            other => Err(format!(
                "the agent announced `{}` as a toggle, which accepts `true` or `false`, not `{other}`",
                option.id
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        SessionConfigBoolean, SessionConfigOptionCategory, SessionConfigSelect,
        SessionConfigSelectGroup,
    };

    fn select_option() -> SessionConfigOption {
        SessionConfigOption {
            id: "model".into(),
            name: "Model".into(),
            description: None,
            category: Some("model".into()),
            kind: SessionConfigKind::Select {
                current_value: "fast".into(),
                values: vec![
                    SessionConfigValue {
                        id: "fast".into(),
                        name: "Fast".into(),
                        description: None,
                    },
                    SessionConfigValue {
                        id: "true".into(),
                        name: "A value that reads like a boolean".into(),
                        description: None,
                    },
                ],
            },
        }
    }

    #[test]
    fn an_announced_selector_crosses_the_contract_with_its_values_and_its_category() {
        let announced = AcpOption::new(
            "model",
            "Model",
            AcpKind::Select(SessionConfigSelect::new(
                "fast",
                vec![
                    SessionConfigSelectOption::new("fast", "Fast"),
                    SessionConfigSelectOption::new("slow", "Slow").description("The careful one"),
                ],
            )),
        )
        .category(SessionConfigOptionCategory::Model);

        let carried = from_acp(&[announced]);
        assert_eq!(carried.len(), 1);
        assert_eq!(carried[0].id, "model");
        // The category travels as the wire string, not as a narrowed enum.
        assert_eq!(carried[0].category.as_deref(), Some("model"));
        let SessionConfigKind::Select {
            current_value,
            values,
        } = &carried[0].kind
        else {
            panic!("a select stays a select");
        };
        assert_eq!(current_value, "fast");
        assert_eq!(values.len(), 2);
        assert_eq!(values[1].description.as_deref(), Some("The careful one"));
    }

    #[test]
    fn a_category_this_build_does_not_know_is_carried_and_not_dropped() {
        let announced = AcpOption::new(
            "thinking",
            "Thinking",
            AcpKind::Boolean(SessionConfigBoolean::new(true)),
        )
        .category(SessionConfigOptionCategory::Other("_vendor_thing".into()));

        let carried = from_acp(&[announced]);
        assert_eq!(carried[0].category.as_deref(), Some("_vendor_thing"));
        assert!(matches!(
            carried[0].kind,
            SessionConfigKind::Boolean {
                current_value: true
            }
        ));
    }

    #[test]
    fn grouped_values_flatten_because_only_the_id_has_to_survive() {
        let announced = AcpOption::new(
            "model",
            "Model",
            AcpKind::Select(SessionConfigSelect::new(
                "a",
                vec![
                    SessionConfigSelectGroup::new(
                        "cheap",
                        "Cheap",
                        vec![SessionConfigSelectOption::new("a", "A")],
                    ),
                    SessionConfigSelectGroup::new(
                        "dear",
                        "Dear",
                        vec![SessionConfigSelectOption::new("b", "B")],
                    ),
                ],
            )),
        );

        let carried = from_acp(&[announced]);
        let SessionConfigKind::Select { values, .. } = &carried[0].kind else {
            panic!("a select stays a select");
        };
        let ids: Vec<&str> = values.iter().map(|value| value.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn the_announced_kind_decides_how_a_value_is_read_not_the_caller() {
        let option = select_option();
        // `true` is a legitimate value id here, and the announcement — not its
        // spelling — is what settles it.
        assert!(matches!(
            chosen_value(&option, "true"),
            Ok(AcpValue::ValueId { .. })
        ));

        let toggle = SessionConfigOption {
            id: "web".into(),
            name: "Web".into(),
            description: None,
            category: None,
            kind: SessionConfigKind::Boolean {
                current_value: false,
            },
        };
        assert!(matches!(
            chosen_value(&toggle, "true"),
            Ok(AcpValue::Boolean { value: true })
        ));
        let refusal = chosen_value(&toggle, "fast").expect_err("a toggle takes no value id");
        assert!(refusal.contains("true"), "{refusal}");
    }

    #[test]
    fn a_value_the_agent_never_announced_is_refused_with_what_it_did_announce() {
        let refusal =
            chosen_value(&select_option(), "opus").expect_err("an unannounced value is refused");
        assert!(refusal.contains("opus"), "{refusal}");
        assert!(
            refusal.contains("fast"),
            "the refusal says what IS accepted: {refusal}"
        );
    }
}
