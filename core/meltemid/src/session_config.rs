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
    SessionModeState,
};
use meltemi_proto::{SessionConfigKind, SessionConfigOption, SessionConfigValue};

/// The id and category of the option synthesised from the protocol's older way
/// of announcing modes.
///
/// `mode` is what the protocol itself calls that category, so a surface that
/// already renders a mode selector renders this one without being told it came
/// from anywhere else. It cannot collide with an agent's own option: an agent
/// that announces a configuration option of this category is the case where
/// nothing is synthesised at all.
pub const MODE_OPTION_ID: &str = "mode";

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

/// What a session announces, and whether its mode option came from the older
/// form (apagado-entero-y-modos design D6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announced {
    /// Exactly what the surfaces see, in the order they render it. Empty means
    /// the agent announced nothing either way.
    pub options: Vec<SessionConfigOption>,
    /// Whether the mode option in `options` was synthesised from the modes
    /// field rather than announced as a configuration option.
    ///
    /// This is the one thing the derivation knows and the option itself cannot
    /// say: an agent is free to announce its own option with this id and this
    /// category, and that one is set through the verb it was announced with.
    /// The daemon remembers this per session; it is internal state and never
    /// travels in the contract.
    pub mode_is_inherited: bool,
}

/// Translates what the agent announced, in **either** of the two ways the
/// protocol lets it announce a mode (apagado-entero-y-modos design D6).
///
/// The protocol carries both on a session response: the modes field, which came
/// first and describes modes only, and configuration options, the general shape
/// that arrived later and absorbs modes under a category of that name. Reading
/// only the second leaves an agent that announces the first looking like an
/// agent that announces nothing — and "announces nothing" is the answer that
/// stops every surface from offering a live change.
///
/// Precedence, and no merging: when the agent announces a configuration option
/// of the mode category, that is what is used and the modes field is ignored
/// entirely. Crossing the two would produce an option with one announcement's
/// values under another's identity, which is an option nobody announced.
#[must_use]
pub fn announced(options: &[AcpOption], modes: Option<&SessionModeState>) -> Announced {
    let translated = from_acp(options);
    if translated
        .iter()
        .any(|option| option.category.as_deref() == Some(MODE_OPTION_ID))
    {
        return Announced {
            options: translated,
            mode_is_inherited: false,
        };
    }
    let Some(modes) = modes.filter(|modes| !modes.available_modes.is_empty()) else {
        return Announced {
            options: translated,
            mode_is_inherited: false,
        };
    };
    // Prepended rather than appended: the surfaces render this list in order,
    // and a mode is the coarsest choice a session has — it belongs where the
    // agent's own mode option would have been offered.
    let mut with_mode = Vec::with_capacity(translated.len() + 1);
    with_mode.push(mode_option(modes));
    with_mode.extend(translated);
    Announced {
        options: with_mode,
        mode_is_inherited: true,
    }
}

/// The option synthesised from the modes field.
fn mode_option(modes: &SessionModeState) -> SessionConfigOption {
    SessionConfigOption {
        id: MODE_OPTION_ID.to_string(),
        // The protocol gives the field no label of its own — it is a list and a
        // current value — so the name is this surface's, and it is the same
        // word the category already uses.
        name: "Mode".to_string(),
        description: None,
        category: Some(MODE_OPTION_ID.to_string()),
        kind: SessionConfigKind::Select {
            current_value: modes.current_mode_id.0.to_string(),
            values: modes
                .available_modes
                .iter()
                .map(|mode| SessionConfigValue {
                    id: mode.id.0.to_string(),
                    name: mode.name.clone(),
                    description: mode.description.clone(),
                })
                .collect(),
        },
    }
}

/// Sets the current value of the mode option in an announced list, if it has
/// one, and says whether it found it.
///
/// The mode option is found by its category, not by its id: the two forms agree
/// on the category and the newer one is free to name the option whatever it
/// likes. A list with no mode option is not an error — the agent may have
/// replaced its announcement with one that has none, and a notification about
/// something nobody announces changes nothing.
pub fn set_current_mode(options: &mut [SessionConfigOption], mode_id: &str) -> bool {
    let Some(option) = options
        .iter_mut()
        .find(|option| option.category.as_deref() == Some(MODE_OPTION_ID))
    else {
        return false;
    };
    match &mut option.kind {
        SessionConfigKind::Select { current_value, .. } => {
            *current_value = mode_id.to_string();
            true
        }
        // A mode that crossed as a toggle is not a mode this can set: the
        // protocol's mode verb names a mode, and a boolean has no name to give
        // it. Leaving it untouched is the honest outcome.
        SessionConfigKind::Boolean { .. } => false,
    }
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
        SessionConfigSelectGroup, SessionMode, SessionModeId,
    };

    /// The older announcement: two modes, the second of them current.
    fn announced_modes() -> SessionModeState {
        SessionModeState::new(
            SessionModeId::new("acceptEdits"),
            vec![
                SessionMode::new(SessionModeId::new("default"), "Ask first"),
                SessionMode::new(SessionModeId::new("acceptEdits"), "Accept edits")
                    .description("Writes without asking"),
            ],
        )
    }

    /// An agent's own mode option, in the newer form.
    fn own_mode_option() -> AcpOption {
        AcpOption::new(
            "mode",
            "Permission mode",
            AcpKind::Select(SessionConfigSelect::new(
                "plan",
                vec![
                    SessionConfigSelectOption::new("plan", "Plan"),
                    SessionConfigSelectOption::new("build", "Build"),
                ],
            )),
        )
        .category(SessionConfigOptionCategory::Mode)
    }

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

    // Scenario: Modos sin opción de configuración se ofrecen como opción
    #[test]
    fn modes_announced_without_a_configuration_option_are_offered_as_one() {
        // The agent that populates only the older field is not an agent
        // without modes, and reading only the newer one would make it look
        // like one -- which is the answer that stops every surface offering a
        // live change.
        let carried = announced(&[], Some(&announced_modes()));

        assert_eq!(
            carried.options.len(),
            1,
            "the modes become exactly one option"
        );
        assert_eq!(carried.options[0].id, MODE_OPTION_ID);
        assert_eq!(carried.options[0].category.as_deref(), Some(MODE_OPTION_ID));
        let SessionConfigKind::Select {
            current_value,
            values,
        } = &carried.options[0].kind
        else {
            panic!("modes are a choice among values, so they cross as a select");
        };
        assert_eq!(
            current_value, "acceptEdits",
            "the current mode is the current value"
        );
        let ids: Vec<&str> = values.iter().map(|value| value.id.as_str()).collect();
        assert_eq!(
            ids,
            ["default", "acceptEdits"],
            "its values are exactly the available modes, in the announced order"
        );
        assert_eq!(values[1].name, "Accept edits");
        assert_eq!(
            values[1].description.as_deref(),
            Some("Writes without asking"),
            "a mode's own description travels with it"
        );
        assert!(
            carried.mode_is_inherited,
            "the daemon has to remember this one is set by the modes verb"
        );
    }

    // Scenario: Con las dos formas gana la opción de configuración
    #[test]
    fn when_both_forms_are_announced_the_configuration_option_wins_whole() {
        // Crossing the two would produce an option carrying one
        // announcement's values under the other's identity: an option nobody
        // announced, and a set nobody could answer.
        let carried = announced(&[own_mode_option()], Some(&announced_modes()));

        assert_eq!(
            carried.options.len(),
            1,
            "nothing is synthesised alongside it"
        );
        assert_eq!(
            carried.options[0].name, "Permission mode",
            "the agent's own option, untouched"
        );
        let SessionConfigKind::Select {
            current_value,
            values,
        } = &carried.options[0].kind
        else {
            panic!("a select stays a select");
        };
        assert_eq!(current_value, "plan");
        let ids: Vec<&str> = values.iter().map(|value| value.id.as_str()).collect();
        assert_eq!(
            ids,
            ["plan", "build"],
            "no value of the modes field is added to it"
        );
        assert!(
            !carried.mode_is_inherited,
            "an option the agent announced is set by the verb it announced it with"
        );
    }

    #[test]
    fn an_agent_that_announces_neither_form_announces_nothing() {
        // The third combination, pinned so the synthesis cannot start
        // inventing an empty selector for an agent that has no modes at all.
        let neither = announced(&[], None);
        assert!(neither.options.is_empty());
        assert!(!neither.mode_is_inherited);

        let empty_modes = announced(
            &[],
            Some(&SessionModeState::new(
                SessionModeId::new("only"),
                Vec::new(),
            )),
        );
        assert!(
            empty_modes.options.is_empty(),
            "a modes field with no modes in it is not an announcement"
        );
        assert!(!empty_modes.mode_is_inherited);
    }

    #[test]
    fn options_that_are_not_modes_keep_their_place_when_a_mode_is_synthesised() {
        // The synthesised option goes first because a mode is the coarsest
        // choice a session has, and the surfaces render this list in order.
        let model = AcpOption::new(
            "model",
            "Model",
            AcpKind::Select(SessionConfigSelect::new(
                "fast",
                vec![SessionConfigSelectOption::new("fast", "Fast")],
            )),
        )
        .category(SessionConfigOptionCategory::Model);

        let carried = announced(&[model], Some(&announced_modes()));
        let ids: Vec<&str> = carried
            .options
            .iter()
            .map(|option| option.id.as_str())
            .collect();
        assert_eq!(ids, [MODE_OPTION_ID, "model"]);
        assert!(carried.mode_is_inherited);
    }

    #[test]
    fn the_current_mode_is_set_on_whichever_form_the_option_came_from() {
        // Found by category, not by id: the two forms agree on the category,
        // and the newer one is free to name its option whatever it likes.
        let mut inherited = announced(&[], Some(&announced_modes())).options;
        assert!(set_current_mode(&mut inherited, "default"));
        let SessionConfigKind::Select { current_value, .. } = &inherited[0].kind else {
            panic!("a select stays a select");
        };
        assert_eq!(current_value, "default");

        let mut own = announced(&[own_mode_option()], None).options;
        assert!(set_current_mode(&mut own, "build"));
        let SessionConfigKind::Select { current_value, .. } = &own[0].kind else {
            panic!("a select stays a select");
        };
        assert_eq!(current_value, "build");

        // And a list with no mode option at all is not an error: the agent may
        // have replaced its announcement with one that has none.
        let mut none = vec![select_option()];
        assert!(
            !set_current_mode(&mut none, "anything"),
            "nothing was announced to set, and it says so instead of inventing one"
        );
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
