// SPDX-License-Identifier: Apache-2.0

//! Permission rules engine (proxy-permisos D1): allow/ask/deny decided in the
//! daemon before a request is escalated to the human.
//!
//! - Rules persist as TOML — global in the user config dir
//!   (`permissions.toml`) and per project in `.meltemi/permissions.toml`. The
//!   scope is *derived from the file*, never written in it.
//! - Loading is tolerant: a whole-file parse error, or a single malformed
//!   entry, is reported as a diagnostic and the remaining valid rules still
//!   apply (the engine never falls over).
//! - Evaluation precedence: project rules over global, and `deny` over `allow`
//!   on a tie. With no rule applicable the request escalates (`ask`).
//! - A rule never grants an option the agent did not offer — it only decides
//!   among the offered options (enforced by the ACP bridge, task 1.3).

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use meltemi_proto::{PermissionRule, PermissionRuleEffect, PermissionRuleScope};

/// The facts extracted from an ACP tool call, matched against rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestFacts {
    /// The tool/operation kind (ACP `toolCall.kind`).
    pub tool: Option<String>,
    /// The command being run, when the tool call carries one.
    pub command: Option<String>,
    /// The affected path, when the tool call carries one.
    pub path: Option<String>,
}

impl RequestFacts {
    /// Extracts the matchable facts from a forwarded ACP tool call value.
    /// The shape is owned by the ACP contract; we read best-effort and treat
    /// anything absent as "no fact on that dimension".
    pub fn from_tool_call(tool_call: &Value) -> Self {
        let str_at = |v: &Value, key: &str| {
            v.get(key)
                .and_then(Value::as_str)
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
        };
        let tool = str_at(tool_call, "kind").or_else(|| str_at(tool_call, "title"));
        let raw_input = tool_call.get("rawInput");
        let command = raw_input.and_then(|ri| str_at(ri, "command"));
        let path = raw_input
            .and_then(|ri| str_at(ri, "path"))
            .or_else(|| {
                tool_call
                    .get("locations")
                    .and_then(Value::as_array)
                    .and_then(|locs| locs.first())
                    .and_then(|loc| loc.get("path"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
            })
            .filter(|s| !s.is_empty());
        Self {
            tool,
            command,
            path,
        }
    }
}

/// The engine's verdict for a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleDecision {
    /// A rule grants the request (the winning rule, for provenance).
    Allow(PermissionRule),
    /// A rule refuses the request (the winning rule, for provenance).
    Deny(PermissionRule),
    /// No rule applies: escalate to the human.
    Ask,
}

/// A loaded rule set with any parse/validation diagnostics.
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    /// The valid rules, global first then project (order within a scope is the
    /// file order); precedence is decided at evaluation, not by position.
    pub rules: Vec<PermissionRule>,
    /// Human-readable diagnostics for entries that were skipped.
    pub diagnostics: Vec<String>,
}

impl RuleSet {
    /// Evaluates a request: project over global, `deny` over `allow` on a tie,
    /// else `ask`.
    #[must_use]
    pub fn evaluate(&self, facts: &RequestFacts) -> RuleDecision {
        let matching: Vec<&PermissionRule> = self
            .rules
            .iter()
            .filter(|r| rule_matches(r, facts))
            .collect();
        if matching.is_empty() {
            return RuleDecision::Ask;
        }
        // Project scope takes precedence over global entirely.
        let scope = if matching
            .iter()
            .any(|r| r.scope == PermissionRuleScope::Project)
        {
            PermissionRuleScope::Project
        } else {
            PermissionRuleScope::Global
        };
        let in_scope = matching.into_iter().filter(|r| r.scope == scope);
        // `deny` wins the tie within the winning scope.
        let mut first_allow: Option<&PermissionRule> = None;
        for rule in in_scope {
            match rule.effect {
                PermissionRuleEffect::Deny => return RuleDecision::Deny(rule.clone()),
                PermissionRuleEffect::Allow => {
                    if first_allow.is_none() {
                        first_allow = Some(rule);
                    }
                }
            }
        }
        RuleDecision::Allow(
            first_allow
                .expect("a non-empty scope has an allow if no deny")
                .clone(),
        )
    }
}

/// Whether a rule matches the facts. Matchers are ANDed; an omitted matcher
/// matches anything on that dimension (a rule with no matcher matches all).
fn rule_matches(rule: &PermissionRule, facts: &RequestFacts) -> bool {
    if let Some(tool) = &rule.tool
        && facts.tool.as_deref() != Some(tool.as_str())
    {
        return false;
    }
    if let Some(prefix) = &rule.command_prefix
        && !facts
            .command
            .as_deref()
            .is_some_and(|c| c.starts_with(prefix))
    {
        return false;
    }
    if let Some(prefix) = &rule.path_prefix
        && !facts.path.as_deref().is_some_and(|p| p.starts_with(prefix))
    {
        return false;
    }
    true
}

/// The TOML shape of one rule entry. The scope is derived from the file, so it
/// is not part of the on-disk form. `effect` is parsed as a string and
/// validated, so a bad value skips one entry instead of failing the file.
#[derive(Debug, Deserialize)]
struct RawRule {
    effect: String,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default, rename = "command-prefix")]
    command_prefix: Option<String>,
    #[serde(default, rename = "path-prefix")]
    path_prefix: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRules {
    #[serde(default)]
    rule: Vec<RawRule>,
}

/// Loads the rules from the global config dir and the optional project root,
/// collecting diagnostics for anything skipped.
pub fn load_rules(config_dir: &Path, project_root: Option<&Path>) -> RuleSet {
    let mut set = RuleSet::default();
    load_file(
        &config_dir.join("permissions.toml"),
        PermissionRuleScope::Global,
        &mut set,
    );
    if let Some(root) = project_root {
        load_file(
            &root.join(".meltemi").join("permissions.toml"),
            PermissionRuleScope::Project,
            &mut set,
        );
    }
    set
}

fn load_file(path: &Path, scope: PermissionRuleScope, set: &mut RuleSet) {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return; // absent file is not an error
    };
    let raw: RawRules = match toml::from_str(&contents) {
        Ok(raw) => raw,
        Err(e) => {
            set.diagnostics
                .push(format!("{}: malformed rules file: {e}", path.display()));
            return;
        }
    };
    for (index, raw_rule) in raw.rule.into_iter().enumerate() {
        match convert_rule(raw_rule, scope) {
            Ok(rule) => set.rules.push(rule),
            Err(reason) => set.diagnostics.push(format!(
                "{} rule #{}: {reason}; skipped",
                path.display(),
                index + 1
            )),
        }
    }
}

fn convert_rule(raw: RawRule, scope: PermissionRuleScope) -> Result<PermissionRule, String> {
    let effect = match raw.effect.as_str() {
        "allow" => PermissionRuleEffect::Allow,
        "deny" => PermissionRuleEffect::Deny,
        other => return Err(format!("unknown effect `{other}` (expected allow|deny)")),
    };
    let non_empty = |o: Option<String>| o.filter(|s| !s.trim().is_empty());
    Ok(PermissionRule {
        effect,
        tool: non_empty(raw.tool),
        command_prefix: non_empty(raw.command_prefix),
        path_prefix: non_empty(raw.path_prefix),
        scope,
    })
}

/// Serializes one rule to the TOML entry form (scope is implied by the file,
/// so it is not written).
fn rule_to_toml(rule: &PermissionRule) -> String {
    let effect = match rule.effect {
        PermissionRuleEffect::Allow => "allow",
        PermissionRuleEffect::Deny => "deny",
    };
    let mut out = format!("[[rule]]\neffect = \"{effect}\"\n");
    let mut kv = |key: &str, value: &Option<String>| {
        if let Some(v) = value {
            out.push_str(&format!("{key} = {}\n", toml_string(v)));
        }
    };
    kv("tool", &rule.tool);
    kv("command-prefix", &rule.command_prefix);
    kv("path-prefix", &rule.path_prefix);
    out
}

/// A minimal TOML basic-string literal (quotes and backslashes escaped).
fn toml_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// Appends a rule to the file for its scope, creating it if needed. Returns
/// the path written. The project scope requires a project root.
pub fn persist_rule(
    config_dir: &Path,
    project_root: Option<&Path>,
    rule: &PermissionRule,
) -> std::io::Result<std::path::PathBuf> {
    let path = match rule.scope {
        PermissionRuleScope::Global => config_dir.join("permissions.toml"),
        PermissionRuleScope::Project => {
            let root = project_root.ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "a project-scoped rule needs a project root",
                )
            })?;
            root.join(".meltemi").join("permissions.toml")
        }
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut body = String::new();
    if !existing.is_empty() && !existing.ends_with('\n') {
        body.push('\n');
    }
    if !existing.is_empty() {
        body.push('\n');
    }
    body.push_str(&rule_to_toml(rule));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(body.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule(
        effect: PermissionRuleEffect,
        tool: Option<&str>,
        cmd: Option<&str>,
        path: Option<&str>,
        scope: PermissionRuleScope,
    ) -> PermissionRule {
        PermissionRule {
            effect,
            tool: tool.map(Into::into),
            command_prefix: cmd.map(Into::into),
            path_prefix: path.map(Into::into),
            scope,
        }
    }

    fn facts(tool: &str, cmd: Option<&str>, path: Option<&str>) -> RequestFacts {
        RequestFacts {
            tool: Some(tool.into()),
            command: cmd.map(Into::into),
            path: path.map(Into::into),
        }
    }

    #[test]
    fn no_rule_asks() {
        // Scenario: Sin regla se pregunta.
        let set = RuleSet::default();
        assert_eq!(set.evaluate(&facts("edit", None, None)), RuleDecision::Ask);
    }

    #[test]
    fn matching_allow_grants_without_escalation() {
        // Scenario: Regla permite sin escalar.
        let set = RuleSet {
            rules: vec![rule(
                PermissionRuleEffect::Allow,
                None,
                Some("cargo test"),
                None,
                PermissionRuleScope::Project,
            )],
            diagnostics: vec![],
        };
        let decision = set.evaluate(&facts("execute", Some("cargo test --all"), None));
        assert!(matches!(decision, RuleDecision::Allow(_)));
    }

    #[test]
    fn project_deny_beats_global_allow() {
        // Scenario: Deny gana el empate (project deny vs global allow).
        let set = RuleSet {
            rules: vec![
                rule(
                    PermissionRuleEffect::Allow,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Global,
                ),
                rule(
                    PermissionRuleEffect::Deny,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Project,
                ),
            ],
            diagnostics: vec![],
        };
        assert!(matches!(
            set.evaluate(&facts("edit", None, None)),
            RuleDecision::Deny(_)
        ));
    }

    #[test]
    fn deny_beats_allow_within_the_same_scope() {
        let set = RuleSet {
            rules: vec![
                rule(
                    PermissionRuleEffect::Allow,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Global,
                ),
                rule(
                    PermissionRuleEffect::Deny,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Global,
                ),
            ],
            diagnostics: vec![],
        };
        assert!(matches!(
            set.evaluate(&facts("edit", None, None)),
            RuleDecision::Deny(_)
        ));
    }

    #[test]
    fn project_allow_beats_global_deny() {
        // Project precedence is entire: a project allow wins over a global deny.
        let set = RuleSet {
            rules: vec![
                rule(
                    PermissionRuleEffect::Deny,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Global,
                ),
                rule(
                    PermissionRuleEffect::Allow,
                    Some("edit"),
                    None,
                    None,
                    PermissionRuleScope::Project,
                ),
            ],
            diagnostics: vec![],
        };
        assert!(matches!(
            set.evaluate(&facts("edit", None, None)),
            RuleDecision::Allow(_)
        ));
    }

    #[test]
    fn prefix_matchers_are_prefixes_not_equality() {
        let set = RuleSet {
            rules: vec![rule(
                PermissionRuleEffect::Allow,
                None,
                None,
                Some("/repo/src"),
                PermissionRuleScope::Project,
            )],
            diagnostics: vec![],
        };
        assert!(matches!(
            set.evaluate(&facts("edit", None, Some("/repo/src/main.rs"))),
            RuleDecision::Allow(_)
        ));
        // A path outside the prefix does not match.
        assert_eq!(
            set.evaluate(&facts("edit", None, Some("/repo/tests/x.rs"))),
            RuleDecision::Ask
        );
    }

    #[test]
    fn facts_are_extracted_from_the_tool_call() {
        let tool_call = json!({
            "toolCallId": "c1",
            "kind": "execute",
            "rawInput": { "command": "rm -rf /", "path": "/danger" },
        });
        let facts = RequestFacts::from_tool_call(&tool_call);
        assert_eq!(facts.tool.as_deref(), Some("execute"));
        assert_eq!(facts.command.as_deref(), Some("rm -rf /"));
        assert_eq!(facts.path.as_deref(), Some("/danger"));

        // Falls back to locations[0].path when rawInput has no path.
        let with_location = json!({
            "toolCallId": "c2",
            "kind": "edit",
            "locations": [ { "path": "/repo/a.rs" } ],
        });
        let facts = RequestFacts::from_tool_call(&with_location);
        assert_eq!(facts.path.as_deref(), Some("/repo/a.rs"));
        assert_eq!(facts.command, None);
    }

    #[test]
    fn loading_reports_malformed_entries_and_keeps_valid_ones() {
        // Scenario: Regla malformada no derriba el motor.
        let dir = std::env::temp_dir().join(format!("meltemi-perm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("permissions.toml"),
            "[[rule]]\neffect = \"allow\"\ntool = \"edit\"\n\n\
             [[rule]]\neffect = \"maybe\"\ntool = \"execute\"\n\n\
             [[rule]]\neffect = \"deny\"\npath-prefix = \"/etc\"\n",
        )
        .unwrap();

        let set = load_rules(&dir, None);
        assert_eq!(set.rules.len(), 2, "the valid rules survive");
        assert_eq!(set.diagnostics.len(), 1, "the bad entry is reported");
        assert!(
            set.diagnostics[0].contains("rule #2") && set.diagnostics[0].contains("maybe"),
            "diagnostic locates the entry: {:?}",
            set.diagnostics
        );
        assert_eq!(set.rules[0].scope, PermissionRuleScope::Global);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn whole_file_parse_error_is_a_single_diagnostic() {
        let dir = std::env::temp_dir().join(format!("meltemi-perm-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("permissions.toml"), "this is not toml = = =").unwrap();
        let set = load_rules(&dir, None);
        assert!(set.rules.is_empty());
        assert_eq!(set.diagnostics.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn project_rules_load_with_project_scope() {
        let dir = std::env::temp_dir().join(format!("meltemi-perm-proj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = dir.join("project");
        std::fs::create_dir_all(project.join(".meltemi")).unwrap();
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(
            project.join(".meltemi").join("permissions.toml"),
            "[[rule]]\neffect = \"deny\"\ntool = \"execute\"\n",
        )
        .unwrap();
        let set = load_rules(&dir.join("config"), Some(&project));
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].scope, PermissionRuleScope::Project);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn persisted_rule_round_trips_through_loading() {
        let dir = std::env::temp_dir().join(format!("meltemi-perm-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let saved = rule(
            PermissionRuleEffect::Allow,
            Some("edit"),
            None,
            Some(r"C:\repo\src"),
            PermissionRuleScope::Global,
        );
        persist_rule(&dir, None, &saved).unwrap();
        // Appending a second rule keeps the first.
        persist_rule(
            &dir,
            None,
            &rule(
                PermissionRuleEffect::Deny,
                None,
                Some("rm"),
                None,
                PermissionRuleScope::Global,
            ),
        )
        .unwrap();

        let set = load_rules(&dir, None);
        assert_eq!(set.diagnostics, Vec::<String>::new());
        assert_eq!(set.rules.len(), 2);
        assert_eq!(set.rules[0], saved, "the round-tripped rule is identical");
        std::fs::remove_dir_all(&dir).ok();
    }
}
