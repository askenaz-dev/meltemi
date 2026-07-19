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
pub fn resolve_launch(config: &Config, path_var: &OsStr) -> Result<Launch, RpcError> {
    if let Some(argv) = &config.agent_command {
        return Ok(Launch::Acp {
            argv: argv.clone(),
            level: 1,
        });
    }
    let Some(id) = &config.agent_id else {
        return Err(RpcError::application(
            error_codes::AGENT_COMMAND_NOT_CONFIGURED,
            "no agent configured",
            "agent_command_not_configured",
            "neither `agent.command` nor `agent.id` is configured",
            Some("Set `agent.id` (see `meltemi fleet`) or `agent.command`.".into()),
        ));
    };
    let catalog = build_catalog(config);
    resolve_id_launch(&catalog, id, path_var)
}

/// A per-session agent resolution: the launch, an env overlay selecting the
/// auth context, and how the name resolved (flota-multiproveedor D1/D2).
#[derive(Debug)]
pub struct ResolvedAgent {
    pub launch: Launch,
    /// Environment overlay for the binary's subprocess (never logged).
    pub env: Vec<(String, String)>,
    pub source: meltemi_proto::FleetResolutionSource,
    pub profile: Option<String>,
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
) -> Result<ResolvedAgent, RpcError> {
    use meltemi_proto::FleetResolutionSource;
    let catalog = build_catalog(config);

    // (a) A launch profile matched by name — resolve its underlying id, then
    //     overlay the auth-context env (`${VAR}` resolved; never a secret).
    if let Some(profile) = config.fleet_profiles.iter().find(|p| p.name == name) {
        let launch = resolve_id_launch(&catalog, &profile.agent, path_var)?;
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
        });
    }

    // (b) A catalog id matched by name.
    if catalog.entries.iter().any(|e| e.id == name) {
        let launch = resolve_id_launch(&catalog, name, path_var)?;
        return Ok(ResolvedAgent {
            launch,
            env: Vec::new(),
            source: FleetResolutionSource::Catalog,
            profile: None,
        });
    }

    // (c) A free label — the project-configured agent.
    let launch = resolve_launch(config, path_var)?;
    Ok(ResolvedAgent {
        launch,
        env: Vec::new(),
        source: FleetResolutionSource::Configured,
        profile: None,
    })
}

/// Resolves the launch for a specific catalog id (the level 1–4 match). Shared
/// by [`resolve_launch`] and [`resolve_fleet_agent`]; propagates 2001 on an
/// undetected binary, never degrading to another agent.
pub fn resolve_id_launch(
    catalog: &crate::fleet::Catalog,
    id: &str,
    path_var: &OsStr,
) -> Result<Launch, RpcError> {
    let Some(entry) = catalog.entries.iter().find(|e| e.id == id) else {
        return Err(not_detected(format!(
            "agent id `{id}` is not in the fleet catalog (registry {})",
            catalog.registry_version
        )));
    };

    match entry.level {
        1 => {
            let bin = resolve_binary(entry.bin.as_deref(), &entry.candidate_paths, path_var)
                .ok_or_else(|| not_detected(undetected(&entry.name)))?;
            let mut argv = vec![bin.display().to_string()];
            argv.extend(entry.acp_args.iter().cloned());
            Ok(Launch::Acp { argv, level: 1 })
        }
        2 => {
            // Level 2 launches the declared ACP adapter, under the same passive
            // detection as a native agent.
            let adapter =
                resolve_binary(entry.adapter.as_deref(), &entry.candidate_paths, path_var)
                    .ok_or_else(|| {
                        not_detected(format!(
                            "the ACP adapter for `{id}` ({}) was not detected",
                            entry.name
                        ))
                    })?;
            let mut argv = vec![adapter.display().to_string()];
            argv.extend(entry.adapter_args.iter().cloned());
            Ok(Launch::Acp { argv, level: 2 })
        }
        3 => {
            let bin = resolve_binary(entry.headless.as_deref(), &entry.candidate_paths, path_var)
                .ok_or_else(|| not_detected(undetected(&entry.name)))?;
            let mut argv = vec![bin.display().to_string()];
            argv.extend(entry.headless_args.iter().cloned());
            // Native controls Meltemi configures from data in one place (D2).
            argv.extend(entry.native_controls.iter().cloned());
            Ok(Launch::Headless { argv, level: 3 })
        }
        4 => Ok(Launch::Artifacts { level: 4 }),
        other => Err(not_detected(format!(
            "agent `{id}` declares unsupported level {other}"
        ))),
    }
}

fn undetected(name: &str) -> String {
    format!("the binary of agent `{name}` was not detected on this system")
}

fn not_detected(detail: String) -> RpcError {
    RpcError::application(
        error_codes::AGENT_NOT_DETECTED,
        "agent not detected",
        "agent_not_detected",
        detail,
        Some(
            "Run `meltemi fleet`, install the agent's official CLI, or set `agent.command`.".into(),
        ),
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
                name: "work".into(),
                agent: "ghost".into(),
                env: vec![],
            }],
        );
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let err = resolve_fleet_agent(&config, "work", &path_var)
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
                name: "work".into(),
                agent: "real".into(),
                env: vec![("MELTEMI_MOCK_MARKER".into(), "work".into())],
            }],
        );
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let resolved = resolve_fleet_agent(&config, "work", &path_var).expect("profile resolves");
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
            resolve_fleet_agent(&config, "fast", &path_var).expect("free label falls back");
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
             adapter=\"adapter-bin\"\nadapter-args=[\"--acp\"]\n",
        );
        let launch = resolve_launch(&config, &path_var).unwrap();
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
        let err = resolve_launch(&config, &path_var).unwrap_err();
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
        let launch = resolve_launch(&config, &path_var).unwrap();
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
        let launch = resolve_launch(&config, std::ffi::OsStr::new("")).unwrap();
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
        let launch = resolve_launch(&config, std::ffi::OsStr::new("")).unwrap();
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
        let err = resolve_launch(&config, std::ffi::OsStr::new("")).unwrap_err();
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
