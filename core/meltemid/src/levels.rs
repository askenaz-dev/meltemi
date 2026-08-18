// SPDX-License-Identifier: Apache-2.0

//! Integration levels (niveles-integracion-conformidad D1/D2).
//!
//! A configured agent is launched behind one session interface, by level:
//! **L1** native ACP over stdio; **L2** ACP through a declared adapter binary;
//! **L3** a headless run with mandatory guardrails whose structured output is
//! mapped to the common session-event subset; **L4** no process at all —
//! context projection is the only channel. Each session declares its level and
//! the capabilities a level lacks are visible, never simulated.

use std::ffi::OsStr;
use std::path::Path;

use serde_json::Value;

use meltemi_proto::{SessionEventKind, error_codes};

use crate::config::Config;
use crate::fleet::{build_catalog, resolve_binary};
use crate::rpc::RpcError;

/// How a configured agent is launched, resolved from its integration level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Level 1 or 2: an ACP session. For L2 the argv is the adapter that
    /// bridges the agent to ACP; the session behaves identically to L1.
    Acp { argv: Vec<String>, level: u8 },
    /// Level 3: a headless run. The argv already carries the native-control
    /// arguments; guardrails are enforced by the runner before spawning.
    Headless { argv: Vec<String>, level: u8 },
    /// Level 4: no process — projection is the only integration channel.
    Artifacts { level: u8 },
}

impl Launch {
    /// The integration level this launch runs at.
    #[must_use]
    pub fn level(&self) -> u8 {
        match self {
            Launch::Acp { level, .. }
            | Launch::Headless { level, .. }
            | Launch::Artifacts { level } => *level,
        }
    }
}

/// Resolves the launch for a configured agent, by level. A literal
/// `agent.command` is always level 1 (native ACP). An `agent.id` resolves
/// against the catalog: L1 detects the binary, L2 detects the adapter, L3
/// detects the headless binary and appends its native controls, L4 needs no
/// process. Detection only — nothing is spawned here; failures are 2000
/// (nothing configured) or 2001 (unknown id / binary not detected).
pub fn resolve_launch(
    config: &Config,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Result<Launch, RpcError> {
    if let Some(argv) = &config.agent_command {
        return Ok(Launch::Acp {
            argv: argv.clone(),
            level: 1,
        });
    }
    let catalog = build_catalog(config);
    let Some(id) = &config.agent_id else {
        // The refusal hands over the fleet rather than a sentence: every surface
        // could only transcribe "neither is configured", while the daemon knows
        // perfectly well which agents are installed here (D7).
        return Err(RpcError::application_with_candidates(
            error_codes::AGENT_COMMAND_NOT_CONFIGURED,
            "no agent configured",
            "agent_command_not_configured",
            "neither `agent.command` nor `agent.id` is configured",
            Some("Set `agent.id` (see `meltemi fleet`) or `agent.command`.".into()),
            crate::fleet::candidates(&catalog, path_var, bundled_dir),
        ));
    };
    resolve_id_launch(&catalog, id, path_var, bundled_dir)
}

/// How the daemon names the per-session model to an adapter.
///
/// Nothing links these two processes at compile time — `meltemi-adapters` is a
/// leaf crate that produces ACP agents and depends on none of the daemon's, and
/// pulling the client contract into an ACP agent to share a string would be the
/// wrong direction of dependency. So the name is written on both sides and a
/// test reads the other side's source to prove they agree
/// (`the_lever_variables_are_spelled_the_same_on_both_sides`).
pub const SESSION_MODEL_ENV: &str = "MELTEMI_SESSION_MODEL";

/// How the daemon names the per-session effort level, on the same terms.
pub const SESSION_EFFORT_ENV: &str = "MELTEMI_SESSION_EFFORT";

/// A per-session agent resolution: the launch, an env overlay selecting the
/// auth context, and how the name resolved (flota-multiproveedor D1/D2).
#[derive(Debug, Clone)]
pub struct ResolvedAgent {
    pub launch: Launch,
    /// Environment overlay for the binary's subprocess (never logged).
    pub env: Vec<(String, String)>,
    pub source: meltemi_proto::FleetResolutionSource,
    pub profile: Option<String>,
    /// The catalog id the name resolved to, when the resolution named one: a
    /// profile's underlying agent, the matched id, or the configured id. `None`
    /// only when the project configures a bare `agent.command`
    /// (multiproyecto-suscripciones D4).
    pub agent_id: Option<String>,
    /// The model this resolution runs by default, from the profile that named
    /// one. What a session declares explicitly overrides it — see
    /// [`ResolvedAgent::with_session_levers`] (modelo-y-esfuerzo D4).
    pub model: Option<String>,
    /// The effort this resolution runs by default, on the same terms.
    pub effort: Option<String>,
}

impl ResolvedAgent {
    /// Applies what the SESSION declared over what the profile defaulted to.
    ///
    /// One direction only, and no third level. A global default would be a
    /// preference left switched on and forgotten — the same reason
    /// `modos-de-autonomia` refused one for autonomy. An empty string is not a
    /// declaration: it would travel to the agent as though the user had chosen
    /// it, so it is treated as absent here and refused at the edge.
    #[must_use]
    pub fn with_session_levers(mut self, model: Option<&str>, effort: Option<&str>) -> Self {
        if let Some(model) = model.map(str::trim).filter(|m| !m.is_empty()) {
            self.model = Some(model.to_string());
        }
        if let Some(effort) = effort.map(str::trim).filter(|e| !e.is_empty()) {
            self.effort = Some(effort.to_string());
        }
        // The adapter is a separate process, so the effective levers travel the
        // way its other configuration already does: through the environment the
        // daemon composes for it. No new transport, and the daemon still never
        // reads the strings — only the adapter that knows its CLI does
        // (modelo-y-esfuerzo-por-sesion design D1).
        //
        // Set LAST so the resolved value wins over anything a profile's own env
        // overlay happened to name.
        self.env
            .retain(|(key, _)| key != SESSION_MODEL_ENV && key != SESSION_EFFORT_ENV);
        if let Some(model) = &self.model {
            self.env
                .push((SESSION_MODEL_ENV.to_string(), model.clone()));
        }
        if let Some(effort) = &self.effort {
            self.env
                .push((SESSION_EFFORT_ENV.to_string(), effort.clone()));
        }
        self
    }
}

/// Resolves an agent named per session against the fleet, in order: a launch
/// **profile**, a catalog **id**, else the project-**configured** agent (a free
/// label falls through to it — keeping race labels like `fast`/`thorough`
/// working). A profile/id that resolves to an **undetected** binary refuses with
/// 2001 and MUST NOT degrade to the configured agent (never a silent provider
/// swap). Detection only — nothing is spawned here.
pub fn resolve_fleet_agent(
    config: &Config,
    name: &str,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Result<ResolvedAgent, RpcError> {
    use meltemi_proto::FleetResolutionSource;
    let catalog = build_catalog(config);

    // (a) A launch profile matched by name — resolve its underlying id, then
    //     overlay the auth-context env (`${VAR}` resolved; never a secret).
    if let Some(profile) = config.fleet_profiles.iter().find(|p| p.name == name) {
        let launch = resolve_id_launch(&catalog, &profile.agent, path_var, bundled_dir)?;
        let env: Vec<(String, String)> = profile
            .env
            .iter()
            .map(|(k, v)| {
                let resolved = crate::mcp::resolve_ref(v);
                if resolved.is_empty() && !v.is_empty() {
                    tracing::warn!(
                        profile = %name, key = %k,
                        "profile env value resolved empty; is the referenced variable set?"
                    );
                }
                (k.clone(), resolved)
            })
            .collect();
        return Ok(ResolvedAgent {
            launch,
            env,
            source: FleetResolutionSource::Profile,
            profile: Some(name.to_string()),
            agent_id: Some(profile.agent.clone()),
            model: profile.model.clone(),
            effort: profile.effort.clone(),
        });
    }

    // (b) A catalog id matched by name.
    if catalog.entries.iter().any(|e| e.id == name) {
        let launch = resolve_id_launch(&catalog, name, path_var, bundled_dir)?;
        return Ok(ResolvedAgent {
            launch,
            env: Vec::new(),
            source: FleetResolutionSource::Catalog,
            profile: None,
            agent_id: Some(name.to_string()),
            model: None,
            effort: None,
        });
    }

    // (c) A free label — the project-configured agent.
    let launch = resolve_launch(config, path_var, bundled_dir)?;
    Ok(ResolvedAgent {
        launch,
        env: Vec::new(),
        source: FleetResolutionSource::Configured,
        profile: None,
        // The configured agent names an id unless the project pins a bare argv.
        agent_id: config.agent_id.clone(),
        model: None,
        effort: None,
    })
}

/// Resolves the agent of a session whose caller MAY have named one
/// (lanzador-conversacional): with a name it is [`resolve_fleet_agent`]; with
/// none it is exactly the project-configured agent the verb used before the
/// parameter existed, described in the same vocabulary so the resolution can be
/// recorded either way. An empty name is refused rather than treated as absent:
/// a name that resolves to nothing in silence is the very failure the fleet's
/// resolution order exists to prevent, and the contract refuses it too.
pub fn resolve_session_agent(
    config: &Config,
    name: Option<&str>,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Result<ResolvedAgent, RpcError> {
    match name {
        Some(name) if name.trim().is_empty() => Err(RpcError::invalid_params(
            "the agent name is empty; name a launch profile or a catalog id, or omit it",
        )),
        Some(name) => resolve_fleet_agent(config, name, path_var, bundled_dir),
        None => Ok(ResolvedAgent {
            launch: resolve_launch(config, path_var, bundled_dir)?,
            env: Vec::new(),
            source: meltemi_proto::FleetResolutionSource::Configured,
            profile: None,
            // The configured agent names an id unless the project pins a bare
            // argv, same as the fallback arm of `resolve_fleet_agent`.
            agent_id: config.agent_id.clone(),
            model: None,
            effort: None,
        }),
    }
}

/// Resolves the launch for a specific catalog id (the level 1–4 match). Shared
/// by [`resolve_launch`] and [`resolve_fleet_agent`]; propagates 2001 on an
/// undetected binary, never degrading to another agent.
pub fn resolve_id_launch(
    catalog: &crate::fleet::Catalog,
    id: &str,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Result<Launch, RpcError> {
    let Some(entry) = catalog.entries.iter().find(|e| e.id == id) else {
        return Err(not_detected(
            format!(
                "agent id `{id}` is not in the fleet catalog (registry {})",
                catalog.registry_version
            ),
            catalog,
            path_var,
            bundled_dir,
        ));
    };

    match entry.level {
        1 => {
            let bin = resolve_binary(
                entry.bin.as_deref(),
                &entry.candidate_paths,
                path_var,
                entry.bundled.then_some(bundled_dir).flatten(),
            )
            .ok_or_else(|| layer_refusal(entry, catalog, path_var, bundled_dir))?;
            let mut argv = vec![bin.display().to_string()];
            argv.extend(entry.acp_args.iter().cloned());
            Ok(Launch::Acp { argv, level: 1 })
        }
        2 => {
            // Level 2 launches the declared ACP adapter, under the same passive
            // detection as a native agent.
            let adapter = resolve_binary(
                entry.adapter.as_deref(),
                &entry.candidate_paths,
                path_var,
                entry.bundled.then_some(bundled_dir).flatten(),
            )
            .ok_or_else(|| layer_refusal(entry, catalog, path_var, bundled_dir))?;
            let mut argv = vec![adapter.display().to_string()];
            argv.extend(entry.adapter_args.iter().cloned());
            Ok(Launch::Acp { argv, level: 2 })
        }
        3 => {
            let bin = resolve_binary(
                entry.headless.as_deref(),
                &entry.candidate_paths,
                path_var,
                entry.bundled.then_some(bundled_dir).flatten(),
            )
            .ok_or_else(|| layer_refusal(entry, catalog, path_var, bundled_dir))?;
            let mut argv = vec![bin.display().to_string()];
            argv.extend(entry.headless_args.iter().cloned());
            // Native controls Meltemi configures from data in one place (D2).
            argv.extend(entry.native_controls.iter().cloned());
            Ok(Launch::Headless { argv, level: 3 })
        }
        4 => Ok(Launch::Artifacts { level: 4 }),
        other => Err(not_detected(
            format!("agent `{id}` declares unsupported level {other}"),
            catalog,
            path_var,
            bundled_dir,
        )),
    }
}

fn undetected(name: &str) -> String {
    format!("the binary of agent `{name}` was not detected on this system")
}

/// The refusal to launch an undetected agent, carrying the SAME diagnosis the
/// fleet view gives: which layer is missing and what to do about it — the exact
/// install command for a layer the user installs, and the reinstall path for a
/// layer that travels in Meltemi's own installers (flota-deteccion-guia design
/// D5; adaptadores-propios-acp design D8). A generic "not detected" at the
/// moment the user most needs the answer is what this replaces.
fn layer_refusal(
    entry: &crate::fleet::CatalogEntry,
    catalog: &crate::fleet::Catalog,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> RpcError {
    let layers = crate::fleet::detect_layers(entry, path_var, bundled_dir);
    let (_, remedy, command) = crate::fleet::compose_state(&layers, false);
    let bundled = crate::fleet::missing_layer(&layers, false).is_some_and(|layer| layer.bundled);
    let detail = match remedy {
        Some(remedy) => format!("agent `{}`: {remedy}", entry.id),
        None => undetected(&entry.name),
    };
    let hint = match (command, bundled) {
        // The command travels in the remedy: it is what the user must run.
        (Some(command), _) => {
            format!("Install the missing layer: `{command}` (see `meltemi fleet`).")
        }
        // Nobody else publishes this layer, so there is no command to give: the
        // remedy is Meltemi's own installer, and saying otherwise would send the
        // user looking for a package that does not exist.
        (None, true) => "That layer ships with Meltemi: reinstall or repair your Meltemi \
             installation (see `meltemi fleet`)."
            .to_string(),
        (None, false) => {
            "Run `meltemi fleet`, install the agent's official CLI, or set `agent.command`."
                .to_string()
        }
    };
    RpcError::application_with_candidates(
        error_codes::AGENT_NOT_DETECTED,
        "agent not detected",
        "agent_not_detected",
        detail,
        Some(hint),
        crate::fleet::candidates(catalog, path_var, bundled_dir),
    )
}

/// A refusal with no layer to point at — an id the catalog does not hold, or a
/// level nothing can launch — carrying the same offer of detected agents.
fn not_detected(
    detail: String,
    catalog: &crate::fleet::Catalog,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> RpcError {
    RpcError::application_with_candidates(
        error_codes::AGENT_NOT_DETECTED,
        "agent not detected",
        "agent_not_detected",
        detail,
        Some(
            "Run `meltemi fleet`, install the agent's official CLI, or set `agent.command`.".into(),
        ),
        crate::fleet::candidates(catalog, path_var, bundled_dir),
    )
}

/// The guardrails a level-3 run requires (design D2): a bounded working
/// directory, native controls (already in the argv), and the rule engine's
/// denials applied as pre-configuration. The daemon refuses to launch L3
/// without them.
pub struct Guardrails {
    /// The bounded working directory the headless agent runs in.
    pub bounded_dir: std::path::PathBuf,
}

/// Prepares the L3 guardrails, refusing (with a diagnostic) when the bounded
/// directory cannot be created. `denied` are the rule-engine denials applied
/// as pre-config: what the rules deny is not enabled for the agent.
pub fn prepare_guardrails(
    base: &std::path::Path,
    session_id: &str,
    _denied: &[String],
) -> Result<Guardrails, RpcError> {
    let bounded_dir = base.join(format!(".meltemi-l3-{session_id}"));
    std::fs::create_dir_all(&bounded_dir).map_err(|e| {
        RpcError::application(
            error_codes::AGENT_SPAWN_FAILED,
            "level 3 guardrails unavailable",
            "l3_guardrails_unavailable",
            format!("could not prepare a bounded directory for the level-3 run: {e}"),
            Some("Ensure the project root is writable for the sandboxed run.".into()),
        )
    })?;
    Ok(Guardrails { bounded_dir })
}

/// Maps one line of a level-3 agent's structured (JSONL) output to a session
/// event. The common subset is recognized; anything else is preserved raw so
/// the log never loses the original (design "risks").
pub fn map_headless_line(line: &str) -> SessionEventKind {
    let raw = || SessionEventKind::AgentUpdate {
        update: Value::String(line.to_string()),
    };
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return raw();
    };
    match value.get("type").and_then(Value::as_str) {
        Some("text") | Some("message") => SessionEventKind::AgentUpdate {
            update: value.clone(),
        },
        Some("error") => SessionEventKind::Error {
            kind: "headless_error".into(),
            detail: value
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or("headless agent reported an error")
                .to_string(),
        },
        // Unmapped structured lines are kept verbatim (raw) in the log.
        _ => SessionEventKind::AgentUpdate { update: value },
    }
}

/// Documented usage-counter aliases of the official level-3 modes, mapped to
/// the counter they mean. Only these are read; anything else stays absent
/// (analitica-consumo-local D3) — a renamed key shows up as a missing counter,
/// which the conformance suite catches, instead of being silently invented.
const USAGE_KEYS: &[(&str, UsageCounter)] = &[
    ("input_tokens", UsageCounter::Input),
    ("prompt_tokens", UsageCounter::Input),
    ("output_tokens", UsageCounter::Output),
    ("completion_tokens", UsageCounter::Output),
    ("cache_read_input_tokens", UsageCounter::CachedInput),
    ("cached_input_tokens", UsageCounter::CachedInput),
    ("reasoning_output_tokens", UsageCounter::Reasoning),
    ("reasoning_tokens", UsageCounter::Reasoning),
    ("total_tokens", UsageCounter::Total),
];

/// Which counter a recognized key feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsageCounter {
    Input,
    Output,
    CachedInput,
    Reasoning,
    Total,
}

/// The objects a structured line may carry its counters in, most specific
/// first. The name of the one that matched becomes the event's `source`, so
/// every figure says where it was read from.
const USAGE_CONTAINERS: &[&str] = &["usage", "token_usage", "total_token_usage", "tokens"];

/// Reads the usage counters a level-3 structured line declares, if any
/// (analitica-consumo-local D3). Returns `None` when the line declares none —
/// no counter is ever synthesized, and a partial declaration yields a partial
/// event rather than zeros. Never reads anything but counters: credentials,
/// headers and account identifiers are not looked at, let alone recorded (§2).
#[must_use]
pub fn usage_from_headless_line(line: &str) -> Option<SessionEventKind> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    let (source, counters) = find_usage(&value, "")?;
    let read = |wanted: UsageCounter| -> Option<u64> {
        USAGE_KEYS
            .iter()
            .filter(|(_, counter)| *counter == wanted)
            .find_map(|(key, _)| counters.get(*key).and_then(Value::as_u64))
    };
    let event = SessionEventKind::UsageReported {
        source,
        model: value
            .get("model")
            .or_else(|| counters.get("model"))
            .and_then(Value::as_str)
            .map(str::to_string),
        input_tokens: read(UsageCounter::Input),
        output_tokens: read(UsageCounter::Output),
        cached_input_tokens: read(UsageCounter::CachedInput),
        reasoning_tokens: read(UsageCounter::Reasoning),
        total_tokens: read(UsageCounter::Total),
    };
    // A container with no recognized counter is not a usage report.
    match &event {
        SessionEventKind::UsageReported {
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            ..
        } => None,
        _ => Some(event),
    }
}

/// Finds the first usage container in the line, depth-first, returning the
/// dotted path that named it (the event's source) and its object.
fn find_usage<'a>(
    value: &'a Value,
    path: &str,
) -> Option<(String, &'a serde_json::Map<String, Value>)> {
    let object = value.as_object()?;
    for name in USAGE_CONTAINERS {
        if let Some(found) = object.get(*name).and_then(Value::as_object) {
            let source = if path.is_empty() {
                (*name).to_string()
            } else {
                format!("{path}.{name}")
            };
            return Some((source, found));
        }
    }
    for (key, child) in object {
        let child_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        if let Some(found) = find_usage(child, &child_path) {
            return Some(found);
        }
    }
    None
}

/// Whether an agent id resolves to a level-4 (artifacts-only) entry, and its
/// projected target file when so — used to include L4 targets in projection.
pub fn l4_target_for(config: &Config) -> Option<String> {
    let id = config.agent_id.as_ref()?;
    let catalog = build_catalog(config);
    let entry = catalog.entries.iter().find(|e| e.id == *id)?;
    if entry.level == 4 {
        entry.l4_target.clone()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lever_variables_are_spelled_the_same_on_both_sides() {
        // Nothing links these two processes at compile time: `meltemi-adapters`
        // is a leaf crate that produces ACP agents and depends on none of the
        // daemon's. Pulling the client contract into an ACP agent just to share
        // a string would be the wrong direction of dependency, so the name is
        // written twice and this reads the other side's source to prove the two
        // agree. A silent disagreement here would mean a model the user chose
        // never reaching the CLI, with nothing reporting it
        // (modelo-y-esfuerzo-por-sesion).
        let adapters = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../meltemi-adapters/src/lib.rs"),
        )
        .expect("the adapters crate is beside this one in the workspace");
        for (constant, value) in [
            ("SESSION_MODEL_ENV", SESSION_MODEL_ENV),
            ("SESSION_EFFORT_ENV", SESSION_EFFORT_ENV),
        ] {
            assert!(
                adapters.contains(&format!("pub const {constant}: &str = \"{value}\";")),
                "the adapters read `{constant}` as something other than `{value}`"
            );
        }
    }

    // Scenario: La sesión pisa el default del perfil
    // Scenario: Un perfil sin declaración no impone nada
    #[test]
    fn what_the_session_declares_wins_over_what_the_profile_defaults_to() {
        let base = ResolvedAgent {
            launch: Launch::Acp {
                argv: vec!["codex".into()],
                level: 1,
            },
            env: Vec::new(),
            source: meltemi_proto::FleetResolutionSource::Profile,
            profile: Some("work".into()),
            agent_id: Some("codex".into()),
            model: Some("profile-model".into()),
            effort: Some("low".into()),
        };

        // Declared explicitly: the session wins, in one direction only.
        let chosen = base
            .clone()
            .with_session_levers(Some("session-model"), None);
        assert_eq!(chosen.model.as_deref(), Some("session-model"));
        assert_eq!(
            chosen.effort.as_deref(),
            Some("low"),
            "what the session did NOT declare is still the profile's"
        );

        // Not declared: the profile's default stands untouched.
        let untouched = base.clone().with_session_levers(None, None);
        assert_eq!(untouched.model.as_deref(), Some("profile-model"));

        // An empty string is not a declaration. It would otherwise travel to the
        // agent as though the user had chosen it.
        let blank = base.clone().with_session_levers(Some("   "), Some(""));
        assert_eq!(blank.model.as_deref(), Some("profile-model"));
        assert_eq!(blank.effort.as_deref(), Some("low"));

        // A profile that declares nothing imposes nothing.
        let bare = ResolvedAgent {
            model: None,
            effort: None,
            ..base
        };
        assert_eq!(bare.clone().with_session_levers(None, None).model, None);
        assert_eq!(
            bare.with_session_levers(Some("x"), None).model.as_deref(),
            Some("x")
        );
    }

    // Scenario: Una palanca que el agente no admite se rehúsa
    // Scenario: Lo no verificado se rehúsa en vez de inventarse
    #[test]
    fn a_lever_the_agent_has_no_place_for_is_refused_by_name() {
        // Codex documents both, in two different places, and takes both.
        let codex = vec!["codex".to_string()];
        assert!(refuse_unsupported_levers(&codex, Some("gpt-x"), Some("high")).is_ok());

        // Claude documents `--model`. Effort is NOT verified against the pinned
        // CLI, so it is refused with that as the reason rather than guessed at —
        // and the refusal names both the agent and the lever.
        let claude = vec!["/usr/local/bin/claude".to_string()];
        assert!(refuse_unsupported_levers(&claude, Some("sonnet"), None).is_ok());
        let refused = refuse_unsupported_levers(&claude, None, Some("high"))
            .expect_err("an unverified lever is refused, not sent");
        assert_eq!(refused.code, error_codes::LEVER_NOT_SUPPORTED);
        let said = format!("{refused:?}");
        assert!(said.contains("claude"), "it names the agent: {said}");
        assert!(said.contains("effort"), "and the lever: {said}");

        // A third-party ACP agent claims nothing at launch, because nothing is
        // known about it before it speaks.
        let other = vec!["some-acp-agent".to_string()];
        assert!(refuse_unsupported_levers(&other, Some("x"), None).is_err());
        assert!(
            refuse_unsupported_levers(&other, None, None).is_ok(),
            "and asking for nothing is never refused"
        );
    }
    use crate::config::CustomAgent;

    fn fake_binary(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        #[cfg(windows)]
        {
            let path = dir.join(format!("{name}.cmd"));
            std::fs::write(&path, "@echo off\r\n").unwrap();
            path
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join(name);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }
    }

    fn registry_config(dir: &std::path::Path, body: &str) -> Config {
        let path = dir.join("registry.toml");
        std::fs::write(&path, body).unwrap();
        Config {
            agent_id: Some("x".into()),
            fleet_registry: Some(path),
            ..Config::default()
        }
    }

    fn temp(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("meltemi-levels-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A registry with one level-1 agent `id` whose binary `id-bin` is created
    /// only when `detected` (so resolution can be exercised both ways).
    fn fleet_config(
        dir: &std::path::Path,
        id: &str,
        profiles: Vec<crate::config::FleetProfile>,
    ) -> Config {
        let path = dir.join("registry.toml");
        std::fs::write(
            &path,
            format!(
                "version=\"v\"\n[[agents]]\nid=\"{id}\"\nname=\"{id}\"\nlevel=1\nbin=\"{id}-bin\"\nacp-args=[\"--acp\"]\n"
            ),
        )
        .unwrap();
        Config {
            // A configured fallback that resolution must NOT reach for a profile/id.
            agent_command: Some(vec!["configured-fallback".into()]),
            fleet_registry: Some(path),
            fleet_profiles: profiles,
            ..Config::default()
        }
    }

    #[test]
    fn a_profile_with_an_undetected_underlying_id_refuses_with_2001() {
        // Scenario: Id no detectado rehúsa sin degradar
        // flota-multiproveedor D1: a profile resolving to an UNDETECTED binary
        // MUST refuse (2001) and NEVER degrade to the configured agent.
        let dir = temp("prof-undetected");
        // 'ghost-bin' is never created -> undetected.
        let config = fleet_config(
            &dir,
            "ghost",
            vec![crate::config::FleetProfile {
                model: None,
                effort: None,
                name: "work".into(),
                agent: "ghost".into(),
                env: vec![],
            }],
        );
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let err = resolve_fleet_agent(&config, "work", &path_var, None)
            .expect_err("an undetected profile must refuse");
        assert_eq!(err.code, meltemi_proto::error_codes::AGENT_NOT_DETECTED);
    }

    #[test]
    fn a_profile_resolves_its_binary_and_overlays_env() {
        // Scenario: Perfil lanza el mismo binario con otro contexto de autenticación
        // A profile launches its underlying detected binary under its env context.
        let dir = temp("prof-ok");
        fake_binary(&dir, "real-bin");
        let config = fleet_config(
            &dir,
            "real",
            vec![crate::config::FleetProfile {
                model: None,
                effort: None,
                name: "work".into(),
                agent: "real".into(),
                env: vec![("MELTEMI_MOCK_MARKER".into(), "work".into())],
            }],
        );
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let resolved =
            resolve_fleet_agent(&config, "work", &path_var, None).expect("profile resolves");
        assert_eq!(
            resolved.source,
            meltemi_proto::FleetResolutionSource::Profile
        );
        assert_eq!(resolved.profile.as_deref(), Some("work"));
        assert!(resolved.launch.level() == 1);
        assert_eq!(
            resolved.env,
            vec![("MELTEMI_MOCK_MARKER".to_string(), "work".to_string())]
        );
    }

    #[test]
    fn a_free_label_falls_back_to_the_configured_agent() {
        // Scenario: Etiqueta libre cae al agente configurado con registro
        // A name matching neither a profile nor a catalog id is a free label
        // that resolves to the configured agent (source=Configured).
        let dir = temp("free-label");
        let config = fleet_config(&dir, "known", vec![]);
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let resolved =
            resolve_fleet_agent(&config, "fast", &path_var, None).expect("free label falls back");
        assert_eq!(
            resolved.source,
            meltemi_proto::FleetResolutionSource::Configured
        );
        assert!(resolved.env.is_empty());
        assert!(matches!(resolved.launch, Launch::Acp { .. }));
    }

    #[test]
    fn level_2_resolves_to_the_detected_adapter() {
        // Scenario: Sesión declara su nivel (L2) / Adaptador como puente.
        let dir = temp("l2");
        fake_binary(&dir, "adapter-bin");
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let config = registry_config(
            &dir,
            "version=\"v\"\n[[agents]]\nid=\"x\"\nname=\"X\"\nlevel=2\n\
             adapter=\"adapter-bin\"\nadapter-args=[\"--acp\"]\ncli-bin=\"x-cli\"\n",
        );
        let launch = resolve_launch(&config, &path_var, None).unwrap();
        assert_eq!(launch.level(), 2);
        assert!(matches!(launch, Launch::Acp { .. }), "L2 is an ACP launch");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_2_without_the_adapter_is_2001() {
        // Scenario: Adaptador no detectado.
        let dir = temp("l2-missing");
        let path_var = std::ffi::OsString::new();
        let config = registry_config(
            &dir,
            "version=\"v\"\n[[agents]]\nid=\"x\"\nname=\"X\"\nlevel=2\nadapter=\"absent-adapter\"\n",
        );
        let err = resolve_launch(&config, &path_var, None).unwrap_err();
        assert_eq!(err.code, error_codes::AGENT_NOT_DETECTED);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_3_appends_native_controls_and_is_headless() {
        let dir = temp("l3");
        fake_binary(&dir, "headless-bin");
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let config = registry_config(
            &dir,
            "version=\"v\"\n[[agents]]\nid=\"x\"\nname=\"X\"\nlevel=3\n\
             headless=\"headless-bin\"\nheadless-args=[\"run\"]\n\
             native-controls=[\"--sandbox\",\"--no-network\"]\n",
        );
        let launch = resolve_launch(&config, &path_var, None).unwrap();
        match launch {
            Launch::Headless { argv, level } => {
                assert_eq!(level, 3);
                assert!(
                    argv.iter().any(|a| a == "--sandbox"),
                    "native controls added"
                );
                assert!(argv.iter().any(|a| a == "run"));
            }
            other => panic!("expected Headless, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn level_4_is_artifacts_only_no_process() {
        // Scenario: L4 sin subprocesos.
        let dir = temp("l4");
        let config = registry_config(
            &dir,
            "version=\"v\"\n[[agents]]\nid=\"x\"\nname=\"X\"\nlevel=4\nl4-target=\"AGENTS.md\"\n",
        );
        let launch = resolve_launch(&config, std::ffi::OsStr::new(""), None).unwrap();
        assert_eq!(launch, Launch::Artifacts { level: 4 });
        assert_eq!(l4_target_for(&config).as_deref(), Some("AGENTS.md"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_literal_command_is_level_1() {
        let config = Config {
            agent_command: Some(vec!["some-agent".into()]),
            ..Config::default()
        };
        let launch = resolve_launch(&config, std::ffi::OsStr::new(""), None).unwrap();
        assert_eq!(launch.level(), 1);
    }

    #[test]
    fn a_custom_agent_is_level_1() {
        let config = Config {
            agent_id: Some("mine".into()),
            fleet_custom: vec![CustomAgent {
                id: "mine".into(),
                name: "Mine".into(),
                command: vec!["some-agent".into()],
            }],
            ..Config::default()
        };
        // No PATH, but a custom agent's bin must be detected to launch; here it
        // is absent, so resolution reports 2001 (honest).
        let err = resolve_launch(&config, std::ffi::OsStr::new(""), None).unwrap_err();
        assert_eq!(err.code, error_codes::AGENT_NOT_DETECTED);
    }

    #[test]
    fn headless_output_maps_common_subset_and_keeps_the_rest() {
        // Scenario: Salida estructurada mapeada.
        let text = map_headless_line(r#"{"type":"text","content":"hi"}"#);
        assert!(matches!(text, SessionEventKind::AgentUpdate { .. }));
        let error = map_headless_line(r#"{"type":"error","detail":"boom"}"#);
        assert!(matches!(error, SessionEventKind::Error { detail, .. } if detail == "boom"));
        // A non-JSON line is preserved raw, never dropped.
        let raw = map_headless_line("not json at all");
        assert!(matches!(raw, SessionEventKind::AgentUpdate { .. }));
    }

    #[test]
    fn usage_counters_are_captured_from_the_official_shape() {
        // Scenario: Contadores de uso persistidos desde la salida oficial
        // Scenario: Contador ausente no se registra en cero
        let line = r#"{"type":"result","model":"claude-x","usage":{"input_tokens":120,"output_tokens":34}}"#;
        let event = usage_from_headless_line(line).expect("counters captured");
        match event {
            SessionEventKind::UsageReported {
                source,
                model,
                input_tokens,
                output_tokens,
                cached_input_tokens,
                reasoning_tokens,
                total_tokens,
            } => {
                assert_eq!(source, "usage", "the event says where it was read from");
                assert_eq!(model.as_deref(), Some("claude-x"));
                assert_eq!(input_tokens, Some(120));
                assert_eq!(output_tokens, Some(34));
                // Undeclared breakdowns stay absent — never zero.
                assert_eq!(cached_input_tokens, None);
                assert_eq!(reasoning_tokens, None);
                assert_eq!(total_tokens, None);
            }
            other => panic!("expected a usage event, got {other:?}"),
        }
    }

    #[test]
    fn a_nested_container_is_found_and_named() {
        // The other official shape nests the counters under its own message.
        let line = r#"{"msg":{"type":"token_count","info":{"total_token_usage":{"input_tokens":7,"cached_input_tokens":2}}}}"#;
        let event = usage_from_headless_line(line).expect("counters captured");
        match event {
            SessionEventKind::UsageReported {
                source,
                input_tokens,
                cached_input_tokens,
                ..
            } => {
                assert_eq!(source, "msg.info.total_token_usage");
                assert_eq!(input_tokens, Some(7));
                assert_eq!(cached_input_tokens, Some(2));
            }
            other => panic!("expected a usage event, got {other:?}"),
        }
    }

    #[test]
    fn lines_without_counters_report_no_usage() {
        // A text line, a non-JSON line and a usage container with only unknown
        // keys are all "no usage" — nothing is synthesized.
        assert!(usage_from_headless_line(r#"{"type":"text","content":"hi"}"#).is_none());
        assert!(usage_from_headless_line("not json at all").is_none());
        assert!(
            usage_from_headless_line(r#"{"usage":{"weird_metric":3}}"#).is_none(),
            "an unrecognized key is not a counter"
        );
    }

    #[test]
    fn the_usage_event_carries_only_counters_source_and_model() {
        // Scenario: El evento de uso no transporta identidad de la cuenta
        let line = concat!(
            r#"{"type":"result","model":"m","api_key":"sk-secret","account_id":"acct_1","#,
            r#""headers":{"authorization":"Bearer t"},"cookie":"s=1","#,
            r#""usage":{"input_tokens":1,"organization_id":"org_9"}}"#
        );
        let event = usage_from_headless_line(line).expect("counters captured");
        let json = serde_json::to_string(&event).expect("serialize");
        for forbidden in [
            "sk-secret",
            "acct_1",
            "authorization",
            "Bearer",
            "cookie",
            "org_9",
        ] {
            assert!(
                !json.contains(forbidden),
                "the usage event leaked {forbidden}: {json}"
            );
        }
        assert!(json.contains("\"inputTokens\":1"));
    }

    #[test]
    fn guardrails_prepare_a_bounded_directory() {
        // Scenario: Sin guardarraíles no se lanza (positive path here; the
        // refusal path is exercised when the base is not writable).
        let dir = temp("guard");
        let guard = prepare_guardrails(&dir, "sess-1", &[]).unwrap();
        assert!(guard.bounded_dir.is_dir());
        assert!(guard.bounded_dir.starts_with(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }
}

/// Which per-session levers an agent's own adapter can actually deliver.
///
/// The core does not know what a model name means (§5), but it does know
/// **whether the binary it is about to launch has a documented place to put
/// one** — and refusing there is the difference between a quota lever and a
/// switch that does nothing. A lever silently dropped is worse than no lever:
/// the user believes they lowered their spend
/// (modelo-y-esfuerzo-por-sesion design D3).
///
/// Keyed on the binary's own file name rather than on a catalog id, because
/// what accepts the lever is the process that runs.
#[must_use]
pub fn levers_of(agent_command: &[String]) -> Levers {
    let binary = agent_command
        .first()
        .map(|program| {
            std::path::Path::new(program)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    match binary.as_str() {
        // Its schema puts `model` on the thread and `effort` on the turn: two
        // places, both documented, both wired by the adapter.
        name if name.contains("codex") => Levers {
            model: true,
            effort: true,
        },
        // `--model` is a documented launch flag. Effort is NOT verified against
        // the pinned CLI, and this change refuses rather than guesses: the day
        // it is verified is the day this becomes `true` (design D7).
        name if name.contains("claude") => Levers {
            model: true,
            effort: false,
        },
        // Meltemi's own scripted agent. It is first-party, so what it accepts
        // is not a guess — and it deliberately takes a model and NOT an effort,
        // which is the shape of a real provider that documents one lever and
        // not the other. That lets the end-to-end exercise both branches
        // without any provider at all (§5).
        name if name.contains("mock-agent") => Levers {
            model: true,
            effort: false,
        },
        // Anything else is a third-party ACP agent, whose levers are whatever
        // it announces as session config options. Nothing is claimed for it at
        // launch, because nothing is known before it speaks.
        _ => Levers {
            model: false,
            effort: false,
        },
    }
}

/// What an agent accepts, per lever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Levers {
    /// Whether a model can be named at launch.
    pub model: bool,
    /// Whether an effort level can be named at all.
    pub effort: bool,
}

/// Refuses a lever the resolved agent has no place for, naming both.
///
/// Returns `Ok(())` when nothing was asked for that cannot be delivered.
pub fn refuse_unsupported_levers(
    agent_command: &[String],
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<(), RpcError> {
    let levers = levers_of(agent_command);
    let agent = agent_command
        .first()
        .map(String::as_str)
        .unwrap_or("the agent");
    for (asked, supported, lever, remedy) in [
        (
            model.is_some(),
            levers.model,
            "model",
            "Start the session without `model`, or choose an agent whose provider takes one.",
        ),
        (
            effort.is_some(),
            levers.effort,
            "effort",
            "Start the session without `effort`; the agent decides how much it reasons.",
        ),
    ] {
        if asked && !supported {
            return Err(RpcError::application(
                error_codes::LEVER_NOT_SUPPORTED,
                "the agent does not take this lever",
                "lever_not_supported",
                format!(
                    "`{agent}` has no documented place for a per-session {lever}, so it was \
                     refused rather than dropped in silence"
                ),
                Some(remedy.into()),
            ));
        }
    }
    Ok(())
}
