// SPDX-License-Identifier: Apache-2.0

//! What the terminal says about a session's levers (modelo-por-opcion-anunciada
//! design D4, D6): the model and the effort that governed, and a lever refused
//! when its session opened — the lever, what was asked, why, and what the agent
//! announced or left.
//!
//! The CLI and the TUI transcript say it with the same words. Everything the
//! agent wrote is escaped before it reaches the terminal ([`clean`]), and the
//! lists are bounded here too, whatever the daemon sent: an agent may announce
//! hundreds of values.

use meltemi_proto::{LeverRefusal, LeverRefusalReason, SessionLever};

use crate::approver::clean;
use crate::shell::messages::Lang;

/// At most this many announced values or options are printed; the rest are
/// counted, never dropped in silence.
pub const LISTED: usize = 20;

/// The lever, what was asked and why, in one line.
#[must_use]
pub fn refusal_line(refusal: &LeverRefusal, lang: Lang) -> String {
    let lever = lever_word(refusal.lever, lang);
    let requested = clean(&refusal.requested);
    let left = refusal.left.as_deref().map(clean);
    let why = match (refusal.reason, lang) {
        (LeverRefusalReason::NotAnnounced, Lang::En) => {
            "the agent announces no option for it".to_string()
        }
        (LeverRefusalReason::NotAnnounced, Lang::Es) => {
            "el agente no anuncia una opción para eso".to_string()
        }
        (LeverRefusalReason::Ambiguous, Lang::En) => {
            "the agent announces more than one option for it, and choosing one would be a guess"
                .to_string()
        }
        (LeverRefusalReason::Ambiguous, Lang::Es) => {
            "el agente anuncia más de una opción para eso, y elegir una sería adivinar".to_string()
        }
        (LeverRefusalReason::NotAValue, Lang::En) => {
            "it is not a value the agent announced".to_string()
        }
        (LeverRefusalReason::NotAValue, Lang::Es) => {
            "no es un valor que el agente haya anunciado".to_string()
        }
        (LeverRefusalReason::NotConfirmed, Lang::En) => match &left {
            Some(left) => format!("the agent kept `{left}` instead"),
            None => "the agent's answer did not carry it".to_string(),
        },
        (LeverRefusalReason::NotConfirmed, Lang::Es) => match &left {
            Some(left) => format!("el agente dejó `{left}` en su lugar"),
            None => "la respuesta del agente no lo trae".to_string(),
        },
        (LeverRefusalReason::ModeChanged, Lang::En) => match &left {
            Some(left) => format!("setting it moved the agent's mode to `{left}`"),
            None => "setting it took the agent's mode out of its announcement".to_string(),
        },
        (LeverRefusalReason::ModeChanged, Lang::Es) => match &left {
            Some(left) => format!("fijarlo le cambió el modo al agente a `{left}`"),
            None => "fijarlo sacó el modo del anuncio del agente".to_string(),
        },
        (LeverRefusalReason::NoAnswer, Lang::En) => "the agent did not answer in time".to_string(),
        (LeverRefusalReason::NoAnswer, Lang::Es) => "el agente no contestó a tiempo".to_string(),
    };
    match lang {
        Lang::En => format!("the agent did not take the {lever} `{requested}`: {why}"),
        Lang::Es => format!("el agente no tomó el {lever} `{requested}`: {why}"),
    }
}

/// What the agent announced for the lever — its values, else its options by
/// id — bounded to [`LISTED`] and saying how many more there were. `None` when
/// the refusal names nothing it announced.
#[must_use]
pub fn announced_line(refusal: &LeverRefusal, lang: Lang) -> Option<String> {
    let items: Vec<String> = if !refusal.values.is_empty() {
        refusal.values.iter().map(|value| clean(value)).collect()
    } else if !refusal.options.is_empty() {
        refusal
            .options
            .iter()
            .map(|option| match &option.category {
                Some(category) => format!(
                    "{} ({}, {})",
                    clean(&option.id),
                    clean(&option.name),
                    clean(category)
                ),
                None => format!("{} ({})", clean(&option.id), clean(&option.name)),
            })
            .collect()
    } else {
        return None;
    };
    let shown = items.len().min(LISTED);
    let more = (items.len() - shown) as u64 + u64::from(refusal.more);
    let mut line = items[..shown].join(", ");
    if more > 0 {
        line.push_str(&match lang {
            Lang::En => format!(" and {more} more"),
            Lang::Es => format!(" y {more} más"),
        });
    }
    Some(match lang {
        Lang::En => format!("announced: {line}"),
        Lang::Es => format!("anunció: {line}"),
    })
}

/// The model and the effort that governed, as one line; `None` when neither
/// did, so nothing is invented for a session that declared none.
#[must_use]
pub fn governed_line(model: Option<&str>, effort: Option<&str>, lang: Lang) -> Option<String> {
    let (model_word, effort_word) = match lang {
        Lang::En => ("model", "effort"),
        Lang::Es => ("modelo", "esfuerzo"),
    };
    let parts: Vec<String> = [(model_word, model), (effort_word, effort)]
        .into_iter()
        .filter_map(|(word, value)| value.map(|value| format!("{word}: {}", clean(value))))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn lever_word(lever: SessionLever, lang: Lang) -> &'static str {
    match (lever, lang) {
        (SessionLever::Model, Lang::En) => "model",
        (SessionLever::Model, Lang::Es) => "modelo",
        (SessionLever::Effort, Lang::En) => "effort",
        (SessionLever::Effort, Lang::Es) => "esfuerzo",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::AnnouncedOption;

    fn refusal(reason: LeverRefusalReason) -> LeverRefusal {
        LeverRefusal {
            lever: SessionLever::Model,
            requested: "gpt-x".into(),
            reason,
            values: Vec::new(),
            options: Vec::new(),
            more: 0,
            left: None,
        }
    }

    #[test]
    fn every_reason_is_said_in_both_languages_with_the_lever_and_what_was_asked() {
        for reason in [
            LeverRefusalReason::NotAnnounced,
            LeverRefusalReason::Ambiguous,
            LeverRefusalReason::NotAValue,
            LeverRefusalReason::NotConfirmed,
            LeverRefusalReason::ModeChanged,
            LeverRefusalReason::NoAnswer,
        ] {
            let en = refusal_line(&refusal(reason), Lang::En);
            let es = refusal_line(&refusal(reason), Lang::Es);
            assert!(
                en.contains("model") && en.contains("gpt-x"),
                "{reason:?}: {en}"
            );
            assert!(
                es.contains("modelo") && es.contains("gpt-x"),
                "{reason:?}: {es}"
            );
            assert_ne!(en, es, "{reason:?} has words of its own in each language");
            // Never the contract's identifier in place of words.
            assert!(!en.contains("not_") && !es.contains("not_"), "{en} / {es}");
        }
        let mut kept = refusal(LeverRefusalReason::NotConfirmed);
        kept.left = Some("fast".into());
        assert!(refusal_line(&kept, Lang::En).contains("`fast`"));
        assert!(refusal_line(&kept, Lang::Es).contains("`fast`"));
    }

    #[test]
    fn options_are_named_by_the_id_set_option_takes() {
        let mut twins = refusal(LeverRefusalReason::Ambiguous);
        twins.options = vec![
            AnnouncedOption {
                id: "model".into(),
                name: "Model".into(),
                category: Some("model".into()),
            },
            AnnouncedOption {
                id: "model-backup".into(),
                name: "Backup".into(),
                category: None,
            },
        ];
        assert_eq!(
            announced_line(&twins, Lang::En).as_deref(),
            Some("announced: model (Model, model), model-backup (Backup)")
        );
        assert_eq!(
            announced_line(&refusal(LeverRefusalReason::NoAnswer), Lang::En),
            None
        );
    }

    #[test]
    fn what_governed_is_said_and_nothing_is_invented() {
        assert_eq!(
            governed_line(Some("slow"), Some("high"), Lang::En).as_deref(),
            Some("model: slow · effort: high")
        );
        assert_eq!(
            governed_line(Some("slow"), None, Lang::Es).as_deref(),
            Some("modelo: slow")
        );
        assert_eq!(governed_line(None, None, Lang::En), None);
    }
}
