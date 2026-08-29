// SPDX-License-Identifier: Apache-2.0

//! The harness: what equips the fleet, composed from four scopes
//! (harness-global-y-por-agente design D1/D2/D4/D5).
//!
//! Meltemi reads these files, validates their **form**, and resolves which one
//! governs. It never interprets what a rule means, and it never decides who a
//! rule is for from anything written inside the file — the per-agent axis is a
//! directory, because a directory is form and a field is semantics.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use meltemi_spec::frontmatter::{FrontMatter, parse_front_matter};

/// The file that carries a rule, inside a directory named after the rule.
const RULE_FILE: &str = "RULE.md";
/// The directory that holds the rules of one scope.
const RULES_DIR: &str = "rules";
/// The directory that opens the per-agent axis of a scope.
const PER_AGENT_DIR: &str = "per-agent";
/// The harness root inside a project.
const PROJECT_HARNESS: &str = ".meltemi/harness";
/// The harness root inside the user's configuration directory.
const USER_HARNESS: &str = "harness";

/// Where a piece came from, most general first.
///
/// The order of the variants IS the precedence: derived once here so a caller
/// cannot sort them differently and quietly invert which layer wins.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// `<config>/meltemi/harness/`
    User,
    /// `<config>/meltemi/harness/per-agent/<id>/`
    UserAgent(String),
    /// `<repo>/.meltemi/harness/`
    Project,
    /// `<repo>/.meltemi/harness/per-agent/<id>/`
    ProjectAgent(String),
}

impl Layer {
    /// A stable identifier for the contract and for display.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::UserAgent(_) => "user_agent",
            Self::Project => "project",
            Self::ProjectAgent(_) => "project_agent",
        }
    }

    /// The agent this layer is specific to, when it is.
    #[must_use]
    pub fn agent(&self) -> Option<&str> {
        match self {
            Self::UserAgent(id) | Self::ProjectAgent(id) => Some(id),
            Self::User | Self::Project => None,
        }
    }

    /// Whether this layer belongs to the user's own configuration rather than
    /// to a repository. The projection guard reads this
    /// (harness-global-y-por-agente design D6).
    #[must_use]
    pub fn is_user_scope(&self) -> bool {
        matches!(self, Self::User | Self::UserAgent(_))
    }
}

/// One rule as it was found on disk, before resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The rule's identity: the name of its directory.
    pub name: String,
    /// Where it was found.
    pub layer: Layer,
    /// The `RULE.md` itself.
    pub path: PathBuf,
    /// Its front-matter, entries in source order, including the keys the core
    /// does not use — they are shown, never dropped (design D2).
    pub front: FrontMatter,
    /// The Markdown body.
    pub body: String,
    /// Why this rule cannot be used, when it cannot. Empty means usable.
    pub problems: Vec<String>,
}

impl Rule {
    /// The glob patterns the rule declares it applies to.
    #[must_use]
    pub fn scope(&self) -> &[String] {
        self.front.list("scope").unwrap_or_default()
    }

    /// The severity the rule declares, when it declares one.
    #[must_use]
    pub fn severity(&self) -> Option<&str> {
        self.front.scalar("severity")
    }

    /// The description the rule declares, when it declares one.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.front.scalar("description")
    }

    /// Whether this rule can be projected: it must be readable and its declared
    /// name must be the one its location gives it.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.problems.is_empty()
    }
}

/// One rule after resolution: the one that governs, and the ones it covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The rule in force.
    pub rule: Rule,
    /// The same rule as found in less specific layers, most general first.
    /// Kept so the view can answer "why is mine not the one applying", which
    /// is the question people arrive with (design D5).
    pub shadowed: Vec<Rule>,
}

/// What a scan of the four scopes found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Harness {
    /// The rules that govern, by name.
    pub rules: Vec<Resolved>,
    /// Per-agent directories whose identifier the fleet catalog does not know,
    /// with the path that declares them. Listed, never projected: writing for
    /// an agent the core cannot name is writing blind (design D1).
    pub unknown_agents: Vec<(String, PathBuf)>,
}

impl Harness {
    /// The rules that can actually be projected.
    pub fn usable(&self) -> impl Iterator<Item = &Rule> {
        self.rules
            .iter()
            .map(|resolved| &resolved.rule)
            .filter(|rule| rule.is_usable())
    }
}

/// The identifiers the fleet catalog knows, which are the only ones the
/// per-agent axis accepts.
///
/// Read from the catalog rather than kept as a second list here: two lists of
/// agent identifiers is exactly how the per-agent axis would start applying
/// rules to an agent this build no longer ships.
#[must_use]
pub fn known_agent_ids(config: &crate::config::Config) -> Vec<String> {
    crate::fleet::build_catalog(config)
        .entries
        .into_iter()
        .map(|agent| agent.id)
        .collect()
}

/// Reads every scope that applies and resolves them into one harness.
///
/// `project_root` absent means only the user's own scopes are read, which is
/// what a caller asking "what do I have globally" means. `known_agents` is the
/// fleet catalog: an identifier outside it is reported, not read.
#[must_use]
pub fn discover(
    config_dir: Option<&Path>,
    project_root: Option<&Path>,
    known_agents: &[String],
) -> Harness {
    let mut found: Vec<Rule> = Vec::new();
    let mut unknown_agents: Vec<(String, PathBuf)> = Vec::new();

    if let Some(config) = config_dir {
        let root = config.join(USER_HARNESS);
        found.extend(read_scope(&root, Layer::User));
        collect_per_agent(&root, known_agents, &mut found, &mut unknown_agents, false);
    }
    if let Some(project) = project_root {
        let root = project.join(PROJECT_HARNESS);
        found.extend(read_scope(&root, Layer::Project));
        collect_per_agent(&root, known_agents, &mut found, &mut unknown_agents, true);
    }

    Harness {
        rules: resolve(found),
        unknown_agents,
    }
}

/// Resolution by name: the most specific layer wins ENTIRE.
///
/// No field is merged across layers. A partial merge would produce a rule
/// nobody wrote — the `scope` of one layer with the body of another — and
/// nobody could explain where it came from (design D5).
fn resolve(found: Vec<Rule>) -> Vec<Resolved> {
    let mut by_name: BTreeMap<String, Vec<Rule>> = BTreeMap::new();
    for rule in found {
        by_name.entry(rule.name.clone()).or_default().push(rule);
    }
    by_name
        .into_values()
        .map(|mut candidates| {
            candidates.sort_by(|a, b| a.layer.cmp(&b.layer));
            let rule = candidates
                .pop()
                .expect("a name exists because a rule has it");
            Resolved {
                rule,
                shadowed: candidates,
            }
        })
        .collect()
}

/// Reads `<root>/per-agent/<id>/rules/` for every identifier present.
fn collect_per_agent(
    root: &Path,
    known_agents: &[String],
    found: &mut Vec<Rule>,
    unknown: &mut Vec<(String, PathBuf)>,
    project_scope: bool,
) {
    let Ok(entries) = std::fs::read_dir(root.join(PER_AGENT_DIR)) else {
        return;
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            Some((name, path))
        })
        .collect();
    dirs.sort();
    for (id, path) in dirs {
        if !known_agents.iter().any(|known| known == &id) {
            unknown.push((id, path));
            continue;
        }
        let layer = if project_scope {
            Layer::ProjectAgent(id)
        } else {
            Layer::UserAgent(id)
        };
        found.extend(read_scope(&path, layer));
    }
}

/// Reads `<scope_root>/rules/<name>/RULE.md` for every directory present.
fn read_scope(scope_root: &Path, layer: Layer) -> Vec<Rule> {
    let Ok(entries) = std::fs::read_dir(scope_root.join(RULES_DIR)) else {
        return Vec::new();
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_string();
            Some((name, path))
        })
        .collect();
    dirs.sort();
    dirs.into_iter()
        .filter_map(|(name, dir)| read_rule(&name, &dir.join(RULE_FILE), layer.clone()))
        .collect()
}

/// Reads one `RULE.md`, keeping whatever is wrong with it rather than dropping
/// the file: a rule that cannot be used still has to be explainable.
fn read_rule(name: &str, path: &Path, layer: Layer) -> Option<Rule> {
    let content = std::fs::read_to_string(path).ok()?;
    let (front, body) = parse_front_matter(&content).unwrap_or_else(|| {
        let mut empty = FrontMatter::default();
        empty
            .problems
            .push("the file has no front-matter block".to_string());
        (empty, content.as_str())
    });

    let mut problems = front.problems.clone();
    // The directory names the rule, and the block must agree. They are two
    // statements of the same identity, and a disagreement would make the view
    // and the resolution talk about different things (design D2).
    match front.scalar("name") {
        Some(declared) if declared == name => {}
        Some(declared) => problems.push(format!(
            "the front-matter declares `{declared}` but the directory names it `{name}`; \
             the directory is the identity resolution uses"
        )),
        None => problems.push("the front-matter declares no `name`".to_string()),
    }

    Some(Rule {
        name: name.to_string(),
        layer,
        path: path.to_path_buf(),
        front,
        body: body.to_string(),
        problems,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agents() -> Vec<String> {
        vec!["claude-code".to_string(), "codex-cli".to_string()]
    }

    fn write_rule(root: &Path, scope: &str, name: &str, front: &str) {
        let dir = root.join(scope).join(RULES_DIR).join(name);
        std::fs::create_dir_all(&dir).expect("fixture dir");
        std::fs::write(
            dir.join(RULE_FILE),
            format!("---\n{front}\n---\n\n# {name}\n\nbody of {name}\n"),
        )
        .expect("fixture rule");
    }

    fn fixture(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "meltemi-harness-{}-{tag}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("fixture root");
        root
    }

    // Scenario: Lo específico pisa lo general
    #[test]
    fn the_more_specific_layer_wins_whole_and_the_covered_one_stays_visible() {
        let root = fixture("precedence");
        let config = root.join("config");
        let project = root.join("repo");
        write_rule(
            &config,
            USER_HARNESS,
            "no-any-cast",
            "name: no-any-cast\nseverity: warning",
        );
        write_rule(
            &project,
            PROJECT_HARNESS,
            "no-any-cast",
            "name: no-any-cast\nseverity: error",
        );

        let harness = discover(Some(&config), Some(&project), &agents());
        assert_eq!(harness.rules.len(), 1, "one name, one governing rule");
        let resolved = &harness.rules[0];
        assert_eq!(resolved.rule.layer, Layer::Project);
        assert_eq!(resolved.rule.severity(), Some("error"));
        // Whole, not merged: the covered rule keeps its own values.
        assert_eq!(resolved.shadowed.len(), 1);
        assert_eq!(resolved.shadowed[0].layer, Layer::User);
        assert_eq!(resolved.shadowed[0].severity(), Some("warning"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_per_agent_axis_of_a_project_beats_every_other_layer() {
        let root = fixture("axis");
        let config = root.join("config");
        let project = root.join("repo");
        write_rule(&config, USER_HARNESS, "r", "name: r\nseverity: a");
        write_rule(
            &config,
            &format!("{USER_HARNESS}/{PER_AGENT_DIR}/claude-code"),
            "r",
            "name: r\nseverity: b",
        );
        write_rule(&project, PROJECT_HARNESS, "r", "name: r\nseverity: c");
        write_rule(
            &project,
            &format!("{PROJECT_HARNESS}/{PER_AGENT_DIR}/claude-code"),
            "r",
            "name: r\nseverity: d",
        );

        let harness = discover(Some(&config), Some(&project), &agents());
        let resolved = &harness.rules[0];
        assert_eq!(
            resolved.rule.layer,
            Layer::ProjectAgent("claude-code".into())
        );
        assert_eq!(resolved.rule.severity(), Some("d"));
        // The three it covered are kept, most general first.
        let covered: Vec<&str> = resolved
            .shadowed
            .iter()
            .map(|rule| rule.layer.as_str())
            .collect();
        assert_eq!(covered, ["user", "user_agent", "project"]);

        let _ = std::fs::remove_dir_all(&root);
    }

    // Scenario: Sin proyecto solo se responde lo global
    #[test]
    fn without_a_project_only_the_user_scopes_are_read() {
        let root = fixture("global-only");
        let config = root.join("config");
        let project = root.join("repo");
        write_rule(&config, USER_HARNESS, "mine", "name: mine");
        write_rule(&project, PROJECT_HARNESS, "theirs", "name: theirs");

        let harness = discover(Some(&config), None, &agents());
        let names: Vec<&str> = harness
            .rules
            .iter()
            .map(|resolved| resolved.rule.name.as_str())
            .collect();
        assert_eq!(names, ["mine"]);

        let _ = std::fs::remove_dir_all(&root);
    }

    // Scenario: Un agente que el catálogo no conoce no recibe proyección
    #[test]
    fn an_agent_the_catalog_does_not_know_is_listed_and_not_read() {
        let root = fixture("unknown-agent");
        let project = root.join("repo");
        write_rule(
            &project,
            &format!("{PROJECT_HARNESS}/{PER_AGENT_DIR}/some-agent-9000"),
            "r",
            "name: r",
        );

        let harness = discover(None, Some(&project), &agents());
        assert!(
            harness.rules.is_empty(),
            "nothing was read for it: {:?}",
            harness.rules
        );
        assert_eq!(harness.unknown_agents.len(), 1);
        assert_eq!(harness.unknown_agents[0].0, "some-agent-9000");
        assert!(
            harness.unknown_agents[0].1.ends_with("some-agent-9000"),
            "the path that declares it is reported so it can be found"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // Scenario: Un nombre que no coincide con su directorio se rehúsa
    #[test]
    fn a_declared_name_that_contradicts_its_directory_is_refused() {
        let root = fixture("name-mismatch");
        let project = root.join("repo");
        write_rule(
            &project,
            PROJECT_HARNESS,
            "no-any-cast",
            "name: something-else",
        );

        let harness = discover(None, Some(&project), &agents());
        let rule = &harness.rules[0].rule;
        assert!(!rule.is_usable(), "it cannot be projected");
        assert!(
            rule.problems[0].contains("something-else") && rule.problems[0].contains("no-any-cast"),
            "the diagnostic names both: {:?}",
            rule.problems
        );
        assert_eq!(
            harness.usable().count(),
            0,
            "and it is not among what gets projected"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // Scenario: Las claves que el núcleo no conoce se conservan
    #[test]
    fn the_keys_the_core_does_not_use_survive_to_be_shown() {
        let root = fixture("extra-keys");
        let project = root.join("repo");
        write_rule(
            &project,
            PROJECT_HARNESS,
            "r",
            "name: r\nkind: rule\nowner_team: appsec\nagents_supported: [claude-code, codex]",
        );

        let harness = discover(None, Some(&project), &agents());
        let rule = &harness.rules[0].rule;
        assert!(rule.is_usable());
        assert_eq!(rule.front.scalar("owner_team"), Some("appsec"));
        // And `agents_supported` governs NOTHING (design D4): the vocabularies
        // do not agree — this file says `codex`, the catalog says `codex-cli` —
        // and inventing the equivalence would apply rules to the wrong agent in
        // silence. It is carried so a human can read it, and that is all.
        assert_eq!(
            rule.front.list("agents_supported"),
            Some(["claude-code".to_string(), "codex".to_string()].as_slice())
        );
        assert_eq!(
            rule.layer,
            Layer::Project,
            "the directory decided, not the field"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_rule_the_reader_cannot_fully_read_is_kept_with_its_diagnostic() {
        let root = fixture("unreadable");
        let project = root.join("repo");
        write_rule(
            &project,
            PROJECT_HARNESS,
            "r",
            "name: r\nscope:\n  - \"src/**\"",
        );

        let harness = discover(None, Some(&project), &agents());
        let rule = &harness.rules[0].rule;
        assert!(!rule.is_usable());
        assert!(
            rule.problems.iter().any(|p| p.contains("scope")),
            "the diagnostic survives from the reader: {:?}",
            rule.problems
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_user_scope_is_the_one_the_projection_guard_asks_about() {
        // The guard reads this and nothing else, so the answer lives with the
        // layer rather than in the guard (design D6).
        assert!(Layer::User.is_user_scope());
        assert!(Layer::UserAgent("claude-code".into()).is_user_scope());
        assert!(!Layer::Project.is_user_scope());
        assert!(!Layer::ProjectAgent("claude-code".into()).is_user_scope());
    }
}
