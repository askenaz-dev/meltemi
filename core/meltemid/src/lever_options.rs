// SPDX-License-Identifier: Apache-2.0

//! The levers a session sets through the options its agent announced
//! (modelo-por-opcion-anunciada design D1, D3, D4): which option carries each
//! one, the value it takes, whether the agent's answer confirmed it, and the
//! refusal that says why not.
//!
//! Pure on purpose. What travels to the agent, and when, lives in `acp`; what
//! is decided here is decided against announcements alone, so every rule the
//! design states can be pinned without an agent.

use std::time::Duration;

use agent_client_protocol::schema::v1::SessionConfigOptionValue as AcpValue;
use meltemi_proto::{
    AnnouncedOption, ErrorData, LeverRefusal, LeverRefusalReason, SessionConfigKind,
    SessionConfigOption, SessionLever, error_codes,
};

use crate::rpc::RpcError;
use crate::session_config::{MODE_OPTION_ID, chosen_value};

/// How long the agent may take to answer each lever (design D3). A session
/// parameter rather than a constant at the call, so the tests can shorten it.
pub const LEVER_WAIT: Duration = Duration::from_secs(30);

/// How many values or options a refusal lists. An agent may announce hundreds;
/// the rest are counted, never dropped in silence (design D4).
pub const LISTED: usize = 20;

/// The model and the effort a session asks of the agent's announcement — and,
/// once the agent confirmed them, what governed. Empty asks for nothing and
/// sets nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnnouncedLevers {
    /// The model, verbatim as asked.
    pub model: Option<String>,
    /// The effort, verbatim as asked.
    pub effort: Option<String>,
}

impl AnnouncedLevers {
    /// Whether nothing is asked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.model.is_none() && self.effort.is_none()
    }

    /// The levers asked for, in the order they are set: the model first,
    /// because fixing it can change what the effort offers (design D3).
    #[must_use]
    pub fn in_order(&self) -> Vec<(SessionLever, &str)> {
        [
            (SessionLever::Model, &self.model),
            (SessionLever::Effort, &self.effort),
        ]
        .into_iter()
        .filter_map(|(lever, value)| value.as_deref().map(|value| (lever, value)))
        .collect()
    }
}

/// The ACP category that says which option carries a lever.
///
/// The category is the key, and deliberately so: ACP presents it as interface
/// data, but nothing else tells "the model" apart without guessing by id or by
/// value, and guessing is what §5 forbids (design D1).
#[must_use]
pub const fn category(lever: SessionLever) -> &'static str {
    match lever {
        SessionLever::Model => "model",
        SessionLever::Effort => "thought_level",
    }
}

/// A lever as it was set: on which option, asking for what. The option is
/// what the agent's answer is read by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// The lever.
    pub lever: SessionLever,
    /// The id of the option it was set on.
    pub option_id: String,
    /// What was asked, verbatim.
    pub requested: String,
}

/// The one option that carries the lever.
///
/// None, or more than one, is refused rather than guessed at, naming every
/// option the agent announced — so the human can pick one by its id in a live
/// session (design D1).
///
/// # Errors
///
/// The refusal, `not_announced` or `ambiguous`, with the options listed.
pub fn option_for<'a>(
    options: &'a [SessionConfigOption],
    lever: SessionLever,
    requested: &str,
) -> Result<&'a SessionConfigOption, LeverRefusal> {
    let wanted = category(lever);
    let mut carrying = options
        .iter()
        .filter(|option| option.category.as_deref() == Some(wanted));
    match (carrying.next(), carrying.next()) {
        (Some(option), None) => Ok(option),
        (first, _) => {
            let (listed, more) = bounded(options.iter().map(|option| AnnouncedOption {
                id: option.id.clone(),
                name: option.name.clone(),
                category: option.category.clone(),
            }));
            let reason = if first.is_some() {
                LeverRefusalReason::Ambiguous
            } else {
                LeverRefusalReason::NotAnnounced
            };
            let mut refused = refusal(lever, requested, reason, None);
            refused.options = listed;
            refused.more = more;
            Err(refused)
        }
    }
}

/// The value to send for the lever, checked against what its option
/// announced: a value the selector lists, or `true`/`false` for a toggle —
/// the agent's own words, never a translation of ours (design D1).
///
/// # Errors
///
/// The refusal, `not_a_value`, listing what the option accepts.
pub fn value_for(
    option: &SessionConfigOption,
    lever: SessionLever,
    requested: &str,
) -> Result<AcpValue, LeverRefusal> {
    chosen_value(option, requested).map_err(|_| {
        let (values, more) = bounded(accepted(option).into_iter());
        let mut refused = refusal(lever, requested, LeverRefusalReason::NotAValue, None);
        refused.values = values;
        refused.more = more;
        refused
    })
}

/// What an option holds now, in the words a lever asks with: a selector's
/// value id, or a toggle's `true`/`false`.
#[must_use]
pub fn current_of(option: &SessionConfigOption) -> String {
    match &option.kind {
        SessionConfigKind::Select { current_value, .. } => current_value.clone(),
        SessionConfigKind::Boolean { current_value } => current_value.to_string(),
    }
}

/// Whether the agent's answer holds every lever set so far exactly as asked.
///
/// The answer is the whole announcement as it now stands, and the last one
/// decides: a value the agent adjusted, fell back from, or left out is refused
/// naming what it left, because the first prompt carries the repository and
/// goes only to the model that was chosen (design D3).
///
/// # Errors
///
/// The refusal, `not_confirmed`, for the first lever the answer does not hold.
pub fn confirmed(answer: &[SessionConfigOption], asked: &[Asked]) -> Result<(), LeverRefusal> {
    for lever in asked {
        let left = answer
            .iter()
            .find(|option| option.id == lever.option_id)
            .map(current_of);
        if left.as_deref() != Some(lever.requested.as_str()) {
            return Err(refusal(
                lever.lever,
                &lever.requested,
                LeverRefusalReason::NotConfirmed,
                left,
            ));
        }
    }
    Ok(())
}

/// The value of an announcement's mode option, found by its category.
#[must_use]
pub fn mode_of(options: &[SessionConfigOption]) -> Option<String> {
    options
        .iter()
        .find(|option| option.category.as_deref() == Some(MODE_OPTION_ID))
        .map(current_of)
}

/// Whether setting a lever moved the mode sideways (design D3).
///
/// Only a mode that was announced can move. One that vanished from the answer
/// moved too: what the agent does without asking can no longer be told, and
/// the mode is what decides it.
#[must_use]
pub fn mode_moved(before: Option<&str>, after: Option<&str>) -> bool {
    before.is_some_and(|before| after != Some(before))
}

/// A refusal naming the lever, what it asked for and why — and what the agent
/// left instead, when it left something.
#[must_use]
pub fn refusal(
    lever: SessionLever,
    requested: &str,
    reason: LeverRefusalReason,
    left: Option<String>,
) -> LeverRefusal {
    LeverRefusal {
        lever,
        requested: requested.to_string(),
        reason,
        values: Vec::new(),
        options: Vec::new(),
        more: 0,
        left,
    }
}

/// The 2007 that answers a verb whose session refused a lever when it opened,
/// carrying the refusal as data — the same record its log keeps, so each
/// surface can say it in its own words (design D4). The detail and the remedy
/// are the English fallback for a surface that does not.
#[must_use]
pub fn refused_error(refusal: LeverRefusal) -> RpcError {
    let lever = lever_word(refusal.lever);
    let requested = &refusal.requested;
    let (detail, remedy) = match refusal.reason {
        LeverRefusalReason::NotAnnounced => (
            format!(
                "the agent announced no option of category `{}`, so the {lever} `{requested}` \
                 had nowhere to go",
                category(refusal.lever)
            ),
            format!(
                "Start the session without `{lever}`, or set the agent's own option in a live \
                 session with `set-option <id> <value>`."
            ),
        ),
        LeverRefusalReason::Ambiguous => (
            format!(
                "the agent announced more than one option of category `{}`, and choosing which \
                 one is the {lever} would be a guess",
                category(refusal.lever)
            ),
            "Set the option you mean in a live session with `set-option <id> <value>`.".into(),
        ),
        LeverRefusalReason::NotAValue => (
            format!("`{requested}` is not a value the agent announced for its {lever}"),
            format!("Ask for one of the values the agent announced for its {lever}."),
        ),
        LeverRefusalReason::NotConfirmed => (
            match &refusal.left {
                Some(left) => format!(
                    "the agent was asked for the {lever} `{requested}` and kept `{left}` instead"
                ),
                None => format!(
                    "the agent was asked for the {lever} `{requested}` and its answer did not \
                     carry it"
                ),
            },
            format!("Ask for the {lever} the agent keeps, or start without `{lever}`."),
        ),
        LeverRefusalReason::ModeChanged => (
            match &refusal.left {
                Some(left) => format!(
                    "setting the {lever} moved the agent's mode to `{left}`, which nobody asked for"
                ),
                None => format!(
                    "setting the {lever} took the agent's mode out of its announcement, and \
                     what it does without asking can no longer be told"
                ),
            },
            format!(
                "Start without `{lever}`, and change it in a live session where the mode can be \
                 watched."
            ),
        ),
        LeverRefusalReason::NoAnswer => (
            format!("the agent did not answer the {lever} `{requested}` in time"),
            format!("Try again, or start without `{lever}`."),
        ),
    };
    let data = ErrorData {
        kind: "lever_refused".into(),
        detail,
        remedy: Some(remedy),
        candidates: None,
        lever_refused: Some(refusal),
    };
    RpcError {
        code: error_codes::CONFIG_OPTION_NOT_ANNOUNCED,
        message: "the agent did not take the lever".into(),
        data: Some(serde_json::to_value(data).expect("ErrorData serializes")),
    }
}

/// The lever's name in the contract's own words.
fn lever_word(lever: SessionLever) -> &'static str {
    match lever {
        SessionLever::Model => "model",
        SessionLever::Effort => "effort",
    }
}

/// What an option accepts, in the words a lever asks with.
fn accepted(option: &SessionConfigOption) -> Vec<String> {
    match &option.kind {
        SessionConfigKind::Select { values, .. } => {
            values.iter().map(|value| value.id.clone()).collect()
        }
        SessionConfigKind::Boolean { .. } => vec!["true".into(), "false".into()],
    }
}

/// The first [`LISTED`] items, and how many were left out.
fn bounded<T>(items: impl Iterator<Item = T>) -> (Vec<T>, u32) {
    let mut listed = Vec::new();
    let mut more: u32 = 0;
    for item in items {
        if listed.len() < LISTED {
            listed.push(item);
        } else {
            more = more.saturating_add(1);
        }
    }
    (listed, more)
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::SessionConfigValue;

    fn select(
        id: &str,
        category: Option<&str>,
        current: &str,
        values: &[&str],
    ) -> SessionConfigOption {
        SessionConfigOption {
            id: id.into(),
            name: format!("The {id}"),
            description: None,
            category: category.map(str::to_string),
            kind: SessionConfigKind::Select {
                current_value: current.into(),
                values: values
                    .iter()
                    .map(|value| SessionConfigValue {
                        id: (*value).to_string(),
                        name: value.to_uppercase(),
                        description: None,
                    })
                    .collect(),
            },
        }
    }

    fn toggle(id: &str, category: &str, current: bool) -> SessionConfigOption {
        SessionConfigOption {
            id: id.into(),
            name: format!("The {id}"),
            description: None,
            category: Some(category.into()),
            kind: SessionConfigKind::Boolean {
                current_value: current,
            },
        }
    }

    fn asked(lever: SessionLever, option_id: &str, requested: &str) -> Asked {
        Asked {
            lever,
            option_id: option_id.into(),
            requested: requested.into(),
        }
    }

    #[test]
    fn the_model_is_set_before_the_effort() {
        let both = AnnouncedLevers {
            model: Some("slow".into()),
            effort: Some("high".into()),
        };
        assert_eq!(
            both.in_order(),
            vec![
                (SessionLever::Model, "slow"),
                (SessionLever::Effort, "high")
            ]
        );
        let effort_only = AnnouncedLevers {
            model: None,
            effort: Some("high".into()),
        };
        assert_eq!(effort_only.in_order(), vec![(SessionLever::Effort, "high")]);
        assert!(AnnouncedLevers::default().is_empty());
        assert!(!effort_only.is_empty());
    }

    #[test]
    fn the_category_is_the_key_and_exactly_one_option_carries_the_lever() {
        let options = vec![
            select("mode", Some("mode"), "ask", &["ask", "auto"]),
            select("llm", Some("model"), "fast", &["fast", "slow"]),
            toggle("thinking", "thought_level", false),
        ];
        // The id does not matter; the category does.
        assert_eq!(
            option_for(&options, SessionLever::Model, "slow")
                .unwrap()
                .id,
            "llm"
        );
        assert_eq!(
            option_for(&options, SessionLever::Effort, "true")
                .unwrap()
                .id,
            "thinking"
        );

        // None of the category: refused, naming every option announced, so
        // the human can pick one by its id.
        let without = vec![options[0].clone()];
        let refused = option_for(&without, SessionLever::Model, "slow").unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotAnnounced);
        assert_eq!(refused.requested, "slow");
        assert_eq!(
            refused.options,
            vec![AnnouncedOption {
                id: "mode".into(),
                name: "The mode".into(),
                category: Some("mode".into()),
            }]
        );

        // Two of the category: a guess, so refused the same way.
        let mut twins = options.clone();
        twins.push(select("backup", Some("model"), "fast", &["fast", "slow"]));
        let refused = option_for(&twins, SessionLever::Model, "slow").unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::Ambiguous);
        assert_eq!(refused.options.len(), 4);

        // An option without a category is never taken for the lever.
        let uncategorized = vec![select("model", None, "fast", &["fast", "slow"])];
        let refused = option_for(&uncategorized, SessionLever::Model, "slow").unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotAnnounced);
        assert_eq!(refused.options[0].category, None);
    }

    #[test]
    fn a_value_the_option_did_not_announce_is_refused_listing_what_it_did() {
        let model = select("model", Some("model"), "fast", &["fast", "slow"]);
        assert_eq!(
            value_for(&model, SessionLever::Model, "slow").unwrap(),
            AcpValue::value_id("slow".to_string())
        );
        let refused = value_for(&model, SessionLever::Model, "gpt-x").unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotAValue);
        assert_eq!(refused.values, vec!["fast", "slow"]);
        assert_eq!(refused.more, 0);

        // A toggle takes the agent's own two words, and nothing else.
        let thinking = toggle("thinking", "thought_level", false);
        assert_eq!(
            value_for(&thinking, SessionLever::Effort, "true").unwrap(),
            AcpValue::boolean(true)
        );
        let refused = value_for(&thinking, SessionLever::Effort, "high").unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotAValue);
        assert_eq!(refused.values, vec!["true", "false"]);
    }

    #[test]
    fn a_long_announcement_is_listed_to_twenty_and_the_rest_counted() {
        let many: Vec<String> = (0..317).map(|n| format!("provider/model-{n}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let model = select("model", Some("model"), "provider/model-0", &many);
        let refused = value_for(&model, SessionLever::Model, "ollama/qwen").unwrap_err();
        assert_eq!(refused.values.len(), LISTED);
        assert_eq!(refused.values[0], "provider/model-0");
        assert_eq!(refused.more, 297);

        let options: Vec<SessionConfigOption> = (0..25)
            .map(|n| select(&format!("opt-{n}"), None, "a", &["a"]))
            .collect();
        let refused = option_for(&options, SessionLever::Model, "slow").unwrap_err();
        assert_eq!(refused.options.len(), LISTED);
        assert_eq!(refused.more, 5);
    }

    #[test]
    fn the_last_answer_decides_and_names_what_the_agent_left() {
        let set = [asked(SessionLever::Model, "model", "slow")];
        let kept = vec![select("model", Some("model"), "slow", &["fast", "slow"])];
        assert!(confirmed(&kept, &set).is_ok());

        // The agent adjusted it: refused, naming what it left.
        let clamped = vec![select("model", Some("model"), "fast", &["fast", "slow"])];
        let refused = confirmed(&clamped, &set).unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotConfirmed);
        assert_eq!(refused.requested, "slow");
        assert_eq!(refused.left.as_deref(), Some("fast"));

        // The answer no longer carries the option at all: nothing was left.
        let refused = confirmed(&[], &set).unwrap_err();
        assert_eq!(refused.reason, LeverRefusalReason::NotConfirmed);
        assert_eq!(refused.left, None);

        // Setting the effort put the model back: the model is refused, even
        // though its own answer had confirmed it.
        let both = [
            asked(SessionLever::Model, "model", "slow"),
            asked(SessionLever::Effort, "reasoning", "high"),
        ];
        let reset = vec![
            select("model", Some("model"), "fast", &["fast", "slow"]),
            select("reasoning", Some("thought_level"), "high", &["low", "high"]),
        ];
        let refused = confirmed(&reset, &both).unwrap_err();
        assert_eq!(refused.lever, SessionLever::Model);
        assert_eq!(refused.left.as_deref(), Some("fast"));

        // A toggle is confirmed in its own words.
        let thinking = [asked(SessionLever::Effort, "thinking", "true")];
        assert!(confirmed(&[toggle("thinking", "thought_level", true)], &thinking).is_ok());
        let refused =
            confirmed(&[toggle("thinking", "thought_level", false)], &thinking).unwrap_err();
        assert_eq!(refused.left.as_deref(), Some("false"));
    }

    #[test]
    fn the_mode_moves_only_when_it_was_announced_and_changed_or_vanished() {
        let options = vec![
            select("mode", Some("mode"), "ask", &["ask", "auto"]),
            select("model", Some("model"), "fast", &["fast", "slow"]),
        ];
        assert_eq!(mode_of(&options).as_deref(), Some("ask"));
        assert_eq!(mode_of(&options[1..]), None);

        assert!(!mode_moved(Some("ask"), Some("ask")));
        assert!(mode_moved(Some("ask"), Some("auto")));
        // Vanished: what the agent does without asking can no longer be told.
        assert!(mode_moved(Some("ask"), None));
        // Never announced: nothing could move.
        assert!(!mode_moved(None, None));
        assert!(!mode_moved(None, Some("auto")));
    }

    #[test]
    fn the_refusal_answers_as_a_2007_that_carries_it_as_data() {
        let mut refused = refusal(
            SessionLever::Model,
            "slow",
            LeverRefusalReason::NotConfirmed,
            Some("fast".into()),
        );
        refused.values = vec!["fast".into(), "slow".into()];
        let error = refused_error(refused.clone());
        assert_eq!(error.code, error_codes::CONFIG_OPTION_NOT_ANNOUNCED);
        let data: ErrorData = serde_json::from_value(error.data.expect("data")).unwrap();
        assert_eq!(data.kind, "lever_refused");
        assert_eq!(data.lever_refused, Some(refused));
        assert!(data.detail.contains("slow"), "{}", data.detail);
        assert!(data.detail.contains("fast"), "{}", data.detail);
        assert!(data.remedy.is_some());

        // Every reason has words of its own.
        for reason in [
            LeverRefusalReason::NotAnnounced,
            LeverRefusalReason::Ambiguous,
            LeverRefusalReason::NotAValue,
            LeverRefusalReason::NotConfirmed,
            LeverRefusalReason::ModeChanged,
            LeverRefusalReason::NoAnswer,
        ] {
            let error = refused_error(refusal(SessionLever::Effort, "high", reason, None));
            let data: ErrorData = serde_json::from_value(error.data.expect("data")).unwrap();
            assert!(
                data.detail.contains("effort"),
                "{reason:?}: {}",
                data.detail
            );
        }
    }
}
