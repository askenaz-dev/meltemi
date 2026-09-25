// SPDX-License-Identifier: Apache-2.0

//! Deciding a waiting permission request from any shell (verbos-del-agent-boss):
//! which of the agent's options `approve` and `deny` choose, the refusals that
//! name the agent's options instead of guessing, and the listing of what waits.
//!
//! Pure, so every rule is testable without a daemon. The daemon does not check
//! that a decided option is one the agent offered — the trays never send any
//! other, because they only draw those — so this module is where a typed id is
//! held to the agent's list (design D2).

use meltemi_proto::{PendingPermission, PermissionOption, PermissionOptionKind};
use serde_json::{Value, json};

use crate::approver::{clean, kind_words, reject_option};

/// Which way a decision goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    Deny,
}

impl Verdict {
    /// The verb that says it.
    #[must_use]
    pub fn verb(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Deny => "deny",
        }
    }

    /// Whether an option of this class can say this verdict: an approval
    /// approves, a rejection denies, and neither says the other.
    fn admits(self, kind: PermissionOptionKind) -> bool {
        let approves = matches!(
            kind,
            PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
        );
        match self {
            Self::Approve => approves,
            Self::Deny => !approves,
        }
    }

    /// The class of option this verdict takes, in words.
    fn class_words(self) -> &'static str {
        match self {
            Self::Approve => "an approval",
            Self::Deny => "a rejection",
        }
    }
}

/// What a verb decides with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// One of the options the agent offered for the request.
    Option(PermissionOption),
    /// No option: the request is cancelled. Only denying chooses this, and
    /// only when the agent offered no rejection — exactly what denying does in
    /// the tray and in the terminal.
    Cancel,
}

impl Choice {
    /// The option id that travels in `permission/decide`; `None` cancels.
    #[must_use]
    pub fn option_id(&self) -> Option<&str> {
        match self {
            Self::Option(option) => Some(option.option_id.as_str()),
            Self::Cancel => None,
        }
    }
}

/// Why a verb decided nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `--option` named an id the agent did not offer for this request.
    NotOffered { option: String },
    /// `--option` named an option of the other class.
    WrongClass {
        option: String,
        kind: PermissionOptionKind,
    },
    /// `approve` without `--option`, and the agent offered no single one-time
    /// approval: none at all, or several to choose between.
    NoSingleApproval { count: usize },
}

/// The option a verdict decides with, from what the agent offered for this
/// request and what the command line named.
///
/// Without `--option`: approving takes the one-time approval when the agent
/// offered exactly one — with several, picking the first would be a guess, and
/// a permanent approval is never taken by default —; denying takes the agent's
/// rejection, the same one the tray and the terminal take, or cancels when it
/// offered none. With `--option`: the named option, if the agent offered it for
/// this request and it says this verdict.
///
/// # Errors
///
/// A [`Refusal`] when the command line does not say enough to decide without
/// guessing, or names what the agent did not offer.
pub fn choose(
    verdict: Verdict,
    options: &[PermissionOption],
    named: Option<&str>,
) -> Result<Choice, Refusal> {
    if let Some(named) = named {
        let Some(option) = options.iter().find(|option| option.option_id == named) else {
            return Err(Refusal::NotOffered {
                option: named.to_string(),
            });
        };
        if !verdict.admits(option.kind) {
            return Err(Refusal::WrongClass {
                option: named.to_string(),
                kind: option.kind,
            });
        }
        return Ok(Choice::Option(option.clone()));
    }
    match verdict {
        Verdict::Approve => {
            let mut once = options
                .iter()
                .filter(|option| option.kind == PermissionOptionKind::AllowOnce);
            match (once.next(), once.next()) {
                (Some(only), None) => Ok(Choice::Option(only.clone())),
                _ => Err(Refusal::NoSingleApproval {
                    count: options
                        .iter()
                        .filter(|option| option.kind == PermissionOptionKind::AllowOnce)
                        .count(),
                }),
            }
        }
        Verdict::Deny => {
            Ok(reject_option(options)
                .map_or(Choice::Cancel, |option| Choice::Option(option.clone())))
        }
    }
}

/// The agent's options in words, each with the id `--option` takes.
#[must_use]
pub fn option_list(options: &[PermissionOption]) -> String {
    if options.is_empty() {
        return "the agent offered none".to_string();
    }
    options
        .iter()
        .map(|option| {
            format!(
                "`{}` {} ({})",
                clean(&option.option_id),
                clean(&option.name),
                kind_words(option.kind)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A refusal in words: what the command line lacked, and the agent's options.
#[must_use]
pub fn refusal_message(
    verdict: Verdict,
    request: &str,
    options: &[PermissionOption],
    refusal: &Refusal,
) -> String {
    let request = clean(request);
    let listed = option_list(options);
    match refusal {
        Refusal::NotOffered { option } => format!(
            "the agent offered no option `{}` for {request}; its options: {listed} — nothing was \
             decided",
            clean(option)
        ),
        Refusal::WrongClass { option, kind } => {
            let other = match verdict {
                Verdict::Approve => Verdict::Deny,
                Verdict::Deny => Verdict::Approve,
            };
            format!(
                "`{}` is {} ({}), not {}: `meltemi {} {request} --option {}` decides with it — \
                 nothing was decided",
                clean(option),
                other.class_words(),
                kind_words(*kind),
                verdict.class_words(),
                other.verb(),
                clean(option)
            )
        }
        Refusal::NoSingleApproval { count: 0 } => format!(
            "the agent offered no one-time approval for {request}; name one of its options with \
             `--option` (naming it is consent): {listed} — nothing was decided"
        ),
        Refusal::NoSingleApproval { count } => format!(
            "the agent offered {count} one-time approvals for {request}, and choosing one for you \
             would be a guess; name it with `--option`: {listed} — nothing was decided"
        ),
    }
}

/// Why a decision was not applied: the request no longer waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotWaiting {
    /// The queue does not list it.
    Absent,
    /// The queue lists it as expired.
    Expired,
    /// It was waiting when read, and the daemon answered that another decision
    /// resolved it first.
    LostRace,
}

/// A decision that was not applied, in words.
#[must_use]
pub fn not_waiting_message(request: &str, why: NotWaiting) -> String {
    let request = clean(request);
    match why {
        NotWaiting::Absent => format!(
            "{request} is not waiting for a decision: it was decided on another surface, its \
             session stopped, or no such request exists — nothing was decided (`meltemi \
             permissions` lists what waits)"
        ),
        NotWaiting::Expired => format!(
            "{request} is not waiting for a decision: it expired under the daemon's wait policy — \
             nothing was decided"
        ),
        NotWaiting::LostRace => format!(
            "{request} was resolved elsewhere a moment before this decision reached it: the first \
             decision wins, and this one was not applied"
        ),
    }
}

/// What a decision that was applied says, in words.
#[must_use]
pub fn decided_line(verdict: Verdict, entry: &PendingPermission, choice: &Choice) -> String {
    let request = clean(&entry.request_id);
    let session = clean(&entry.session_id);
    let done = match verdict {
        Verdict::Approve => "approved",
        Verdict::Deny => "denied",
    };
    match choice {
        Choice::Option(option) => format!(
            "{done} {request} with `{}` — {} ({}) · session {session}",
            clean(&option.option_id),
            clean(&option.name),
            kind_words(option.kind)
        ),
        Choice::Cancel => format!(
            "{done} {request}: the agent offered no rejection, so it was cancelled · session \
             {session}"
        ),
    }
}

/// What a decision that was applied says, as data.
#[must_use]
pub fn decided_value(verdict: Verdict, entry: &PendingPermission, choice: &Choice) -> Value {
    let kind = match choice {
        Choice::Option(option) => serde_json::to_value(option.kind).unwrap_or(Value::Null),
        Choice::Cancel => Value::Null,
    };
    json!({
        "requestId": entry.request_id,
        "sessionId": entry.session_id,
        "decision": verdict.verb(),
        "optionId": choice.option_id(),
        "optionKind": kind,
        "status": "applied",
    })
}

/// How long something has waited, in the words a column can hold.
fn span(seconds: u64) -> String {
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m{:02}s", seconds / 60, seconds % 60),
        _ => format!("{}h{:02}m", seconds / 3600, (seconds % 3600) / 60),
    }
}

/// When a request stops waiting, in words.
fn expiry(entry: &PendingPermission) -> String {
    if entry.expired {
        return "expired".to_string();
    }
    match entry.expires_in_seconds {
        Some(seconds) if seconds > 0 => format!("expires in {}", span(seconds.unsigned_abs())),
        Some(_) => "expiring".to_string(),
        None => "no deadline".to_string(),
    }
}

/// The listing of what waits (design D10): the totals first, the columns as
/// wide as their content, the agent's options under each request with the id
/// `--option` takes, and every word of the agent's escaped.
#[must_use]
pub fn render_pending(pending: &[PendingPermission]) -> String {
    use std::fmt::Write;
    let expired = pending.iter().filter(|entry| entry.expired).count();
    let waiting = pending.len() - expired;
    if pending.is_empty() {
        return "nothing is waiting for a decision".to_string();
    }
    let mut out = format!("{waiting} waiting for a decision");
    if expired > 0 {
        let _ = write!(
            out,
            " · {expired} expired (no longer decidable; shown until the daemon prunes them)"
        );
    }

    let rows: Vec<[String; 6]> = pending
        .iter()
        .map(|entry| {
            [
                clean(&entry.request_id),
                format!("waited {}", span(entry.waiting_seconds)),
                expiry(entry),
                clean(&entry.session_id),
                clean(&entry.tool),
                clean(&entry.summary),
            ]
        })
        .collect();
    // Every column but the last is as wide as its widest cell; the title is
    // last and never padded, because trailing spaces are noise.
    let mut widths = [0usize; 5];
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row.iter()) {
            *width = (*width).max(cell.chars().count());
        }
    }
    for (row, entry) in rows.iter().zip(pending) {
        let _ = write!(out, "\n  ");
        for (cell, width) in row.iter().take(5).zip(widths) {
            let _ = write!(out, "{cell:<width$}  ");
        }
        let _ = write!(out, "{}", row[5]);
        let _ = write!(out, "\n      options: {}", option_list(&entry.options));
    }
    if waiting > 0 {
        let _ = write!(
            out,
            "\ndecide one: meltemi approve <request> | meltemi deny <request> [--option \
             <option-id>]\nthe full request (command, paths) is in its session's record: meltemi \
             log <session>"
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption {
            option_id: id.into(),
            name: name.into(),
            kind,
        }
    }

    fn usual() -> Vec<PermissionOption> {
        vec![
            option("allow", "Allow", PermissionOptionKind::AllowOnce),
            option("reject", "Reject", PermissionOptionKind::RejectOnce),
        ]
    }

    fn entry(request: &str, title: &str, options: Vec<PermissionOption>) -> PendingPermission {
        PendingPermission {
            request_id: request.into(),
            session_id: "0b7c2f1e-sess".into(),
            tool: "execute".into(),
            summary: title.into(),
            options,
            waiting_seconds: 123,
            expires_in_seconds: None,
            expired: false,
            suggested_rule: None,
        }
    }

    #[test]
    fn approving_takes_the_single_one_time_approval_and_denying_the_rejection() {
        assert_eq!(
            choose(Verdict::Approve, &usual(), None),
            Ok(Choice::Option(usual()[0].clone()))
        );
        assert_eq!(
            choose(Verdict::Deny, &usual(), None),
            Ok(Choice::Option(usual()[1].clone()))
        );
        // Without a rejection, denying cancels — as in the tray and the
        // terminal.
        let only_allow = vec![option("allow", "Allow", PermissionOptionKind::AllowOnce)];
        assert_eq!(choose(Verdict::Deny, &only_allow, None), Ok(Choice::Cancel));
        assert_eq!(Choice::Cancel.option_id(), None);
    }

    #[test]
    fn a_permanent_approval_is_never_taken_by_default() {
        let always = vec![
            option("always", "Always allow", PermissionOptionKind::AllowAlways),
            option("reject", "Reject", PermissionOptionKind::RejectOnce),
        ];
        assert_eq!(
            choose(Verdict::Approve, &always, None),
            Err(Refusal::NoSingleApproval { count: 0 })
        );
        // Naming it is consent.
        assert_eq!(
            choose(Verdict::Approve, &always, Some("always")),
            Ok(Choice::Option(always[0].clone()))
        );
        let said = refusal_message(
            Verdict::Approve,
            "perm-2",
            &always,
            &Refusal::NoSingleApproval { count: 0 },
        );
        assert!(said.contains("no one-time approval for perm-2"), "{said}");
        assert!(
            said.contains("`always` Always allow (allow always)"),
            "{said}"
        );
        assert!(said.contains("nothing was decided"), "{said}");
    }

    #[test]
    fn several_approvals_demand_that_one_be_named() {
        // Scenario: Varias aprobaciones exigen nombrar una
        let two = vec![
            option("option-0", "Rewrite it", PermissionOptionKind::AllowOnce),
            option("option-1", "Patch it", PermissionOptionKind::AllowOnce),
        ];
        let refused = choose(Verdict::Approve, &two, None);
        assert_eq!(refused, Err(Refusal::NoSingleApproval { count: 2 }));
        let said = refusal_message(Verdict::Approve, "perm-5", &two, &refused.unwrap_err());
        assert!(said.contains("2 one-time approvals"), "{said}");
        assert!(said.contains("would be a guess"), "{said}");
        assert!(
            said.contains("`option-0` Rewrite it (allow once)"),
            "{said}"
        );
        assert!(said.contains("`option-1` Patch it (allow once)"), "{said}");
        // Named, either one decides.
        assert_eq!(
            choose(Verdict::Approve, &two, Some("option-1")),
            Ok(Choice::Option(two[1].clone()))
        );
    }

    #[test]
    fn an_option_the_agent_did_not_offer_or_of_the_other_class_is_refused() {
        assert_eq!(
            choose(Verdict::Approve, &usual(), Some("allow_always")),
            Err(Refusal::NotOffered {
                option: "allow_always".into()
            })
        );
        // Ids are the agent's, compared exactly.
        assert!(choose(Verdict::Approve, &usual(), Some("Allow")).is_err());
        let wrong = choose(Verdict::Approve, &usual(), Some("reject"));
        assert_eq!(
            wrong,
            Err(Refusal::WrongClass {
                option: "reject".into(),
                kind: PermissionOptionKind::RejectOnce,
            })
        );
        let said = refusal_message(Verdict::Approve, "perm-1", &usual(), &wrong.unwrap_err());
        assert!(
            said.contains("`reject` is a rejection (reject once), not an approval"),
            "{said}"
        );
        assert!(
            said.contains("`meltemi deny perm-1 --option reject`"),
            "{said}"
        );
        assert_eq!(
            choose(Verdict::Deny, &usual(), Some("allow")),
            Err(Refusal::WrongClass {
                option: "allow".into(),
                kind: PermissionOptionKind::AllowOnce,
            })
        );
    }

    #[test]
    fn the_listing_opens_with_its_totals_and_aligns_by_content() {
        let mut long = entry("perm-12", "Run cargo test", usual());
        long.tool = "a-much-longer-tool".into();
        let mut old = entry("perm-3", "Edit notes/plan.md", usual());
        old.expired = true;
        old.waiting_seconds = 7;
        let listing = render_pending(&[entry("perm-4", "Read the tree", usual()), long, old]);
        let lines: Vec<&str> = listing.lines().collect();
        assert_eq!(
            lines[0],
            "2 waiting for a decision · 1 expired (no longer decidable; shown until the daemon \
             prunes them)"
        );
        // The titles start in one column whatever the tool's length.
        let rows: Vec<&&str> = lines
            .iter()
            .filter(|line| line.starts_with("  perm"))
            .collect();
        assert_eq!(rows.len(), 3, "{listing}");
        let column = |row: &str, title: &str| row.find(title).expect("the title is shown");
        assert_eq!(
            column(rows[0], "Read the tree"),
            column(rows[1], "Run cargo test")
        );
        assert_eq!(
            column(rows[1], "Run cargo test"),
            column(rows[2], "Edit notes/plan.md")
        );
        assert!(rows[0].contains("waited 2m03s"), "{listing}");
        assert!(rows[0].contains("no deadline"), "{listing}");
        assert!(rows[2].contains("expired"), "{listing}");
        assert!(
            listing.contains("options: `allow` Allow (allow once) · `reject` Reject (reject once)"),
            "{listing}"
        );
        assert!(listing.contains("meltemi log <session>"), "{listing}");
        assert_eq!(render_pending(&[]), "nothing is waiting for a decision");
    }

    #[test]
    fn the_listing_escapes_what_the_agent_wrote() {
        // Scenario: El listado escapa el texto del agente
        let mut hostile = entry(
            "perm-9",
            "Run \u{1b}[2J rm -rf \u{202e}txt.exe",
            vec![option(
                "allow\u{7}",
                "Allow\u{1b}]0;owned\u{7}",
                PermissionOptionKind::AllowOnce,
            )],
        );
        hostile.tool = "exec\u{200b}ute".into();
        let listing = render_pending(&[hostile]);
        assert!(
            !listing.contains('\u{1b}')
                && !listing.contains('\u{7}')
                && !listing.contains('\u{202e}')
                && !listing.contains('\u{200b}'),
            "no character the terminal would obey: {listing:?}"
        );
        assert!(
            listing.contains("Run \\u{001b}[2J rm -rf \\u{202e}txt.exe"),
            "{listing}"
        );
        assert!(listing.contains("exec\\u{200b}ute"), "{listing}");
        assert!(listing.contains("`allow\\u{0007}`"), "{listing}");
    }

    #[test]
    fn the_decision_names_the_option_or_says_it_cancelled() {
        let approved = entry("perm-1", "t", usual());
        let choice = choose(Verdict::Approve, &usual(), None).unwrap();
        assert_eq!(
            decided_line(Verdict::Approve, &approved, &choice),
            "approved perm-1 with `allow` — Allow (allow once) · session 0b7c2f1e-sess"
        );
        let value = decided_value(Verdict::Approve, &approved, &choice);
        assert_eq!(value["optionId"], "allow");
        assert_eq!(value["optionKind"], "allow_once");
        assert_eq!(value["decision"], "approve");
        assert_eq!(value["status"], "applied");

        let cancelled = decided_line(Verdict::Deny, &approved, &Choice::Cancel);
        assert!(cancelled.contains("offered no rejection, so it was cancelled"));
        let value = decided_value(Verdict::Deny, &approved, &Choice::Cancel);
        assert_eq!(value["optionId"], Value::Null);
        assert_eq!(value["decision"], "deny");
    }

    #[test]
    fn a_decision_not_applied_says_why() {
        assert!(not_waiting_message("perm-1", NotWaiting::Absent).contains("nothing was decided"));
        assert!(not_waiting_message("perm-1", NotWaiting::Expired).contains("expired"));
        assert!(
            not_waiting_message("perm-1", NotWaiting::LostRace).contains("the first decision wins")
        );
        // An id is echoed escaped: it came from the command line, and it is
        // printed back to a terminal.
        assert!(!not_waiting_message("perm\u{1b}[2J", NotWaiting::Absent).contains('\u{1b}'));
    }
}
