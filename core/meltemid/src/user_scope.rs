// SPDX-License-Identifier: Apache-2.0

//! Projecting the user's own harness into the agents' user-scope instruction
//! files (harness-global-y-por-agente design D6/D7).
//!
//! This is the other half of the projection, and the half that touches config
//! Meltemi does not own. Three rules govern it, and none of them is optional:
//!
//! 1. **Only what the user wrote globally travels here.** Nothing of a
//!    repository reaches these files, which is the mirror of the guard that
//!    keeps the user's own content out of a repository.
//! 2. **Only verified destinations are written.** A path guessed from a
//!    pattern would put a team's instructions somewhere no agent reads, and
//!    look like it worked.
//! 3. **The first write to an agent's file is consented to explicitly and
//!    recorded.** Someone else's configuration is not ours to start editing
//!    because the feature exists (§2).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::harness::{Layer, discover};

/// Where consent is remembered, inside the user's own configuration.
const CONSENT_FILE: &str = "harness-consent.toml";

/// One verified user-scope destination, as the embedded map declares it.
#[derive(Debug, Clone, Deserialize)]
pub struct UserTarget {
    /// The fleet catalog id this destination belongs to.
    pub agent: String,
    /// The file, relative to the user's home directory.
    pub path: String,
    /// When the path was read from the provider's documentation.
    pub verified_on: String,
    /// The page it was read from, so the claim can be re-checked.
    pub source: String,
    /// An environment variable that relocates the agent's home, when the
    /// provider documents one.
    #[serde(default)]
    pub home_env: Option<String>,
    /// A sibling file whose presence makes the agent ignore this destination.
    #[serde(default)]
    pub ignored_when_present: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawUserTargets {
    #[serde(default)]
    user_target: Vec<UserTarget>,
}

/// The verified user-scope destinations, parsed from the embedded map.
///
/// # Panics
///
/// Panics if the embedded map does not parse, which a unit test prevents.
#[must_use]
pub fn user_targets() -> Vec<UserTarget> {
    let raw: RawUserTargets = toml::from_str(crate::context::EMBEDDED_TARGETS)
        .expect("embedded context targets are valid");
    raw.user_target
}

/// What happened with one user-scope destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserState {
    /// The file was updated.
    Written,
    /// The managed block already carried this content.
    Unchanged,
    /// The agent's file has never been consented to. Nothing was written.
    AwaitingConsent,
    /// The agent reads a different file while this one is present, so writing
    /// here would write into something nobody reads.
    IgnoredByAgent {
        /// The file that takes precedence.
        overridden_by: String,
    },
}

/// One destination and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserWritten {
    /// The catalog id whose file this is.
    pub agent: String,
    /// The resolved absolute path.
    pub path: PathBuf,
    /// What happened.
    pub state: UserState,
}

/// Reads the set of agent ids whose user-scope file the user has consented to.
#[must_use]
pub fn consented(config_dir: &Path) -> BTreeSet<String> {
    #[derive(Deserialize)]
    struct Consent {
        #[serde(default)]
        agents: Vec<String>,
    }
    std::fs::read_to_string(config_dir.join(CONSENT_FILE))
        .ok()
        .and_then(|text| toml::from_str::<Consent>(&text).ok())
        .map(|consent| consent.agents.into_iter().collect())
        .unwrap_or_default()
}

/// Records consent for a set of agent ids, keeping whatever was consented to
/// before. Consent is only ever added here; withdrawing it is editing the file.
///
/// # Errors
///
/// Propagates the write error.
pub fn record_consent(config_dir: &Path, agents: &BTreeSet<String>) -> std::io::Result<()> {
    let mut all = consented(config_dir);
    all.extend(agents.iter().cloned());
    let listed = all
        .iter()
        .map(|agent| format!("  \"{agent}\",\n"))
        .collect::<String>();
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(
        config_dir.join(CONSENT_FILE),
        format!(
            "# Agents whose user-scope instruction file Meltemi may write into.\n\
             #\n\
             # Written when consent was given, and read before every projection of\n\
             # the user's own harness. Removing an entry withdraws the consent:\n\
             # the file itself is never deleted, and Meltemi's managed block in it\n\
             # simply stops being updated.\n\
             \nagents = [\n{listed}]\n"
        ),
    )
}

/// Resolves a destination's absolute path, honouring the agent's own home
/// variable when the provider documents one.
#[must_use]
pub fn resolve(target: &UserTarget) -> Option<PathBuf> {
    let rest = target.path.strip_prefix("~/")?;
    // A documented home variable relocates the agent's whole directory, so the
    // first segment of the path is what it replaces.
    if let Some(name) = &target.home_env
        && let Ok(home) = std::env::var(name)
        && !home.trim().is_empty()
    {
        let inside = rest.split_once('/').map_or(rest, |(_, tail)| tail);
        return Some(PathBuf::from(home).join(inside));
    }
    let base = directories::BaseDirs::new()?;
    Some(base.home_dir().join(rest))
}

/// Projects the user's own harness into every verified destination the user has
/// consented to, reporting each one either way.
///
/// The repository is not reachable from here: `discover` is called with no
/// project root, so nothing of a repository can travel into a user's file —
/// the mirror of the guard in [`crate::context::compile`].
///
/// # Errors
///
/// Propagates write errors of the files it was allowed to write.
pub fn project_user_scope(
    config_dir: &Path,
    known_agents: &[String],
) -> std::io::Result<Vec<UserWritten>> {
    let allowed = consented(config_dir);
    let harness = discover(Some(config_dir), None, known_agents);
    debug_assert!(
        harness
            .rules
            .iter()
            .all(|resolved| resolved.rule.layer.is_user_scope()),
        "a project-scope rule reached the user projection"
    );

    let mut out = Vec::new();
    for target in user_targets() {
        if !known_agents.iter().any(|id| id == &target.agent) {
            continue;
        }
        let Some(path) = resolve(&target) else {
            continue;
        };

        // A rule under `per-agent/<other>` is not this agent's harness.
        let rules: Vec<crate::harness::Rule> = harness
            .usable()
            .filter(|rule| match &rule.layer {
                Layer::User => true,
                Layer::UserAgent(id) => id == &target.agent,
                Layer::Project | Layer::ProjectAgent(_) => false,
            })
            .cloned()
            .collect();
        if rules.is_empty() {
            continue;
        }

        // The agent tells us, in its own documentation, when it reads
        // something else instead. Writing anyway would produce a file that
        // looks configured and is never read.
        if let Some(other) = &target.ignored_when_present
            && let Some(dir) = path.parent()
            && dir.join(other).is_file()
        {
            out.push(UserWritten {
                agent: target.agent.clone(),
                path,
                state: UserState::IgnoredByAgent {
                    overridden_by: other.clone(),
                },
            });
            continue;
        }

        if !allowed.contains(&target.agent) {
            out.push(UserWritten {
                agent: target.agent.clone(),
                path,
                state: UserState::AwaitingConsent,
            });
            continue;
        }

        let content = compile_user(&rules);
        let wrote = crate::context::write_managed_block(&path, &content)?;
        out.push(UserWritten {
            agent: target.agent.clone(),
            path,
            state: if wrote {
                UserState::Written
            } else {
                UserState::Unchanged
            },
        });
    }
    Ok(out)
}

/// Compiles the user's own rules into the block that goes in their agents'
/// files. Deliberately narrow: no constitution, no rumbo, no active change —
/// none of that is the user's, and none of it belongs in a file that applies to
/// every repository they open.
fn compile_user(rules: &[crate::harness::Rule]) -> String {
    let projected: Vec<meltemi_spec::ProjectedRule> = rules
        .iter()
        .map(|rule| meltemi_spec::ProjectedRule {
            name: &rule.name,
            scope: rule.scope(),
            description: rule.description(),
            body: &rule.body,
        })
        .collect();
    meltemi_spec::project_user_rules(&projected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_map_parses_and_every_entry_carries_its_citation() {
        let targets = user_targets();
        assert!(
            targets.len() >= 3,
            "the verified destinations are present: {targets:?}"
        );
        for target in &targets {
            assert!(
                target.path.starts_with("~/"),
                "`{}` is relative to the home directory",
                target.agent
            );
            assert!(
                target.source.starts_with("https://"),
                "`{}` says where the path was read from",
                target.agent
            );
            assert!(
                target.verified_on.len() == 10,
                "`{}` says when it was read: {:?}",
                target.agent,
                target.verified_on
            );
        }
    }

    #[test]
    fn the_map_declares_no_destination_it_has_not_verified() {
        // Every entry must name an agent the catalog knows: a destination for
        // an id nobody ships is a path that will never be checked again.
        let registry = crate::fleet::parse_registry(crate::fleet::EMBEDDED_REGISTRY)
            .expect("the registry parses");
        let known: Vec<&str> = registry
            .agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect();
        for target in user_targets() {
            assert!(
                known.contains(&target.agent.as_str()),
                "`{}` is not an id of the fleet catalog: {known:?}",
                target.agent
            );
        }
    }

    #[test]
    fn consent_is_remembered_and_only_ever_added_to() {
        let dir = std::env::temp_dir().join(format!("meltemi-consent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");

        assert!(
            consented(&dir).is_empty(),
            "nothing is consented by default"
        );

        let mut first = BTreeSet::new();
        first.insert("claude-code".to_string());
        record_consent(&dir, &first).expect("record");
        assert!(consented(&dir).contains("claude-code"));

        let mut second = BTreeSet::new();
        second.insert("codex-cli".to_string());
        record_consent(&dir, &second).expect("record");
        let now = consented(&dir);
        assert!(
            now.contains("claude-code") && now.contains("codex-cli"),
            "consenting to one does not withdraw another: {now:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
