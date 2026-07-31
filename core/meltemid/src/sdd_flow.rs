// SPDX-License-Identifier: Apache-2.0

//! SDD authoring cycle handlers (ciclo-sdd-autoria D1/D2/D4).
//!
//! The verbs `sdd/explore`, `sdd/constitution`, `sdd/propose`, `sdd/plan` and
//! `sdd/gate` drive the configured agent to author artifacts, validate each
//! with the spec engine before a human gate, and persist the cycle state in
//! the change directory. Gates are **scriptable steps**: each RPC advances the
//! cycle and reports the pending gate, never waiting on interactive input, and
//! the persisted state survives daemon restarts.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use meltemi_proto::{
    SddExploreParams, SddGateParams, SddPlanParams, SddProposeParams, SddResult, SessionEventKind,
    SessionState, error_codes,
};

use crate::acp::{self, SessionParams};
use crate::config::Config;
use crate::permissions::RuleSet;
use crate::rpc::{Peer, RpcError};
use crate::sdd::{Artifact, CycleState, GateOutcome, GateStep, Mode};
use crate::server::DaemonState;
use crate::session_log::SessionLog;

/// `sdd/explore`: deliberate with the agent in streaming, guaranteed to write
/// nothing. Writes are denied by a deny-all posture and the `.meltemi` tree is
/// checked to be unchanged afterward (defense in depth).
pub async fn handle_explore(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddExploreParams = parse(params, "sdd/explore")?;
    let project_root = project_dir(&params.project_root)?;

    let before = tree_fingerprint(&project_root);
    let prompt = format!(
        "Deliberate on the following. Read the repository, weigh options, and \
         propose a direction. DO NOT write or modify any file.\n\nTOPIC: {}\n",
        params.topic
    );
    // Deny-all rules: any write the agent attempts is refused (explore never
    // writes; the deliberation lives only in the session log).
    run_turn(state, peer, &project_root, prompt, RuleSet::deny_all()).await?;
    let after = tree_fingerprint(&project_root);
    if before != after {
        return Err(RpcError::internal(
            "explore modified the project tree, which it must never do",
        ));
    }

    Ok(result(SddResult {
        change_name: String::new(),
        phase: "explored".into(),
        artifact: None,
        mode: None,
        diagnostics: vec![],
        gate_hint: None,
    }))
}

/// `sdd/constitution`: initialize a minimal `.meltemi/` when missing, draft
/// `constitution.md`, and open the final gate before it persists.
pub async fn handle_constitution(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddExploreParams = parse(params, "sdd/constitution")?;
    let project_root = project_dir(&params.project_root)?;

    // Minimal `.meltemi/` structure.
    let meltemi = project_root.join(".meltemi");
    std::fs::create_dir_all(meltemi.join("rumbo")).map_err(RpcError::internal)?;
    std::fs::create_dir_all(meltemi.join("changes")).map_err(RpcError::internal)?;

    // Author the constitution to a staging file, gated before it becomes final.
    let draft = meltemi.join("constitution.draft.md");
    let prompt = format!(
        "Write a project constitution into the file at the path below, using \
         guided sections (principles, non-negotiables). \n\
         CONSTITUTION_PATH: {}\nGUIDANCE: {}\n",
        draft.display(),
        params.topic
    );
    run_turn(state, peer, &project_root, prompt, allow_meltemi_writes()).await?;

    // A single-gate cycle under the synthetic change name "constitution".
    let change_dir = meltemi.join("changes").join("constitution");
    let mut cycle = CycleState::new("constitution", Mode::FastForward, false);
    cycle.open_gate();
    cycle.save(&change_dir).map_err(RpcError::internal)?;

    Ok(result(SddResult {
        change_name: "constitution".into(),
        phase: "gate_pending".into(),
        artifact: Some("constitution".into()),
        mode: Some("fast_forward".into()),
        diagnostics: vec![],
        gate_hint: Some(gate_hint("constitution")),
    }))
}

/// `sdd/propose`: start the authoring cycle. Scaffolds the change, authors the
/// proposal, validates it, and opens the first gate. `force_mode` overrides the
/// default (spec-full) against the eligibility criterion, recorded in state.
pub async fn handle_propose(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddProposeParams = parse(params, "sdd/propose")?;
    let project_root = project_dir(&params.project_root)?;

    let change_name = crate::propose::derive_change_name(&params.idea).ok_or_else(|| {
        RpcError::application(
            error_codes::INVALID_IDEA,
            "invalid idea",
            "invalid_idea",
            "no change name could be derived from the idea",
            Some("Describe the change in a few words.".into()),
        )
    })?;
    let change_dir = project_root
        .join(".meltemi")
        .join("changes")
        .join(&change_name);
    if change_dir.exists() {
        return Err(RpcError::application(
            error_codes::CHANGE_ALREADY_EXISTS,
            "change already exists",
            "change_already_exists",
            format!("`.meltemi/changes/{change_name}` already exists"),
            Some("Try a different idea.".into()),
        ));
    }
    std::fs::create_dir_all(&change_dir).map_err(RpcError::internal)?;

    let (mode, forced) = match params.force_mode.as_deref() {
        Some("fast_forward") => (Mode::FastForward, true),
        Some("spec_full") => (Mode::SpecFull, true),
        // Default: spec-full until deltas exist to assess eligibility (D3).
        _ => (Mode::SpecFull, false),
    };
    let cycle = CycleState::new(&change_name, mode, forced);
    cycle.save(&change_dir).map_err(RpcError::internal)?;

    author_and_gate(state, peer, &project_root, &change_dir, change_name, cycle).await
}

/// `sdd/plan`: refine the design and sequence `tasks.md` by dependencies, then
/// open a gate. Runs as a standalone gated turn on an existing change.
pub async fn handle_plan(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddPlanParams = parse(params, "sdd/plan")?;
    let project_root = project_dir(&params.project_root)?;
    let change_dir = project_root
        .join(".meltemi")
        .join("changes")
        .join(&params.change_name);
    if !change_dir.is_dir() {
        return Err(not_found_change(&params.change_name));
    }
    let tasks = change_dir.join("tasks.md");
    let prompt = format!(
        "Refine the design and sequence tasks.md by declared dependencies, \
         annotating any file overlap between tasks. Write into the path below.\n\
         TASKS_PATH: {}\n",
        tasks.display()
    );
    run_turn(state, peer, &project_root, prompt, allow_meltemi_writes()).await?;

    let mut cycle = CycleState::load(&change_dir)
        .unwrap_or_else(|| CycleState::new(&params.change_name, Mode::SpecFull, false));
    cycle.current = Some(Artifact::Tasks);
    cycle.open_gate();
    cycle.save(&change_dir).map_err(RpcError::internal)?;
    Ok(result(SddResult {
        change_name: params.change_name.clone(),
        phase: "gate_pending".into(),
        artifact: Some("tasks".into()),
        mode: Some(mode_str(cycle.mode)),
        diagnostics: vec![],
        gate_hint: Some(gate_hint(&params.change_name)),
    }))
}

/// `sdd/gate`: decide a pending gate. Approve advances (and authors the next
/// artifact); comment reworks the same artifact without consuming the gate;
/// abort ends the cycle.
pub async fn handle_gate(
    params: Value,
    state: &Arc<DaemonState>,
    peer: &Peer,
) -> Result<Value, RpcError> {
    let params: SddGateParams = parse(params, "sdd/gate")?;
    let project_root = project_dir(&params.project_root)?;
    let change_dir = project_root
        .join(".meltemi")
        .join("changes")
        .join(&params.change_name);
    let mut cycle =
        CycleState::load(&change_dir).ok_or_else(|| not_found_change(&params.change_name))?;
    if !cycle.gate_pending {
        return Err(RpcError::invalid_params(format!(
            "no gate is pending for `{}`",
            params.change_name
        )));
    }

    let outcome = match params.decision.as_str() {
        "approve" => GateOutcome::Approve,
        "abort" => GateOutcome::Abort,
        "comment" => GateOutcome::Comment(params.comment.clone().unwrap_or_default()),
        other => {
            return Err(RpcError::invalid_params(format!(
                "unknown gate decision `{other}` (approve|comment|abort)"
            )));
        }
    };

    match cycle.decide(outcome) {
        GateStep::Completed => {
            cycle.save(&change_dir).map_err(RpcError::internal)?;
            Ok(result(done(&params.change_name, "completed", cycle.mode)))
        }
        GateStep::Aborted => {
            cycle.save(&change_dir).map_err(RpcError::internal)?;
            Ok(result(done(&params.change_name, "aborted", cycle.mode)))
        }
        GateStep::Rework(comment) => {
            // Comment goes back to the agent as a rework instruction on the
            // same artifact; the gate re-opens on the reworked draft.
            let artifact = cycle.current.unwrap_or(Artifact::Proposal);
            author_artifact(
                state,
                peer,
                &project_root,
                &change_dir,
                artifact,
                Some(&comment),
            )
            .await?;
            reopen_and_report(&change_dir, cycle, &params.change_name).await
        }
        GateStep::Advanced(next) => {
            author_artifact(state, peer, &project_root, &change_dir, next, None).await?;
            match validate_artifact(&change_dir, next) {
                Ok(()) => reopen_and_report(&change_dir, cycle, &params.change_name).await,
                Err(diagnostics) => {
                    // Invalid: do not open the gate; return diagnostics.
                    cycle.gate_pending = false;
                    cycle.save(&change_dir).map_err(RpcError::internal)?;
                    Ok(result(SddResult {
                        change_name: params.change_name.clone(),
                        phase: "invalid".into(),
                        artifact: Some(artifact_str(next)),
                        mode: Some(mode_str(cycle.mode)),
                        diagnostics,
                        gate_hint: None,
                    }))
                }
            }
        }
    }
}

/// Authors the current artifact, validates it, and opens the first gate — the
/// shared tail of `sdd/propose`.
async fn author_and_gate(
    state: &Arc<DaemonState>,
    peer: &Peer,
    project_root: &Path,
    change_dir: &Path,
    change_name: String,
    mut cycle: CycleState,
) -> Result<Value, RpcError> {
    let artifact = cycle.current.unwrap_or(Artifact::Proposal);
    // Fast-forward authors all four artifacts before the single gate.
    if cycle.mode == Mode::FastForward {
        for a in [
            Artifact::Proposal,
            Artifact::Specs,
            Artifact::Design,
            Artifact::Tasks,
        ] {
            author_artifact(state, peer, project_root, change_dir, a, None).await?;
        }
    } else {
        author_artifact(state, peer, project_root, change_dir, artifact, None).await?;
    }

    match validate_artifact(change_dir, artifact) {
        Ok(()) => {
            cycle.open_gate();
            cycle.save(change_dir).map_err(RpcError::internal)?;
            Ok(result(SddResult {
                change_name: change_name.clone(),
                phase: "gate_pending".into(),
                artifact: Some(artifact_str(artifact)),
                mode: Some(mode_str(cycle.mode)),
                diagnostics: vec![],
                gate_hint: Some(gate_hint(&change_name)),
            }))
        }
        Err(diagnostics) => {
            cycle.save(change_dir).map_err(RpcError::internal)?;
            Ok(result(SddResult {
                change_name,
                phase: "invalid".into(),
                artifact: Some(artifact_str(artifact)),
                mode: Some(mode_str(cycle.mode)),
                diagnostics,
                gate_hint: None,
            }))
        }
    }
}

/// Opens the gate on the (already-authored) current artifact and reports it.
async fn reopen_and_report(
    change_dir: &Path,
    mut cycle: CycleState,
    change_name: &str,
) -> Result<Value, RpcError> {
    let artifact = cycle.current;
    cycle.open_gate();
    cycle.save(change_dir).map_err(RpcError::internal)?;
    Ok(result(SddResult {
        change_name: change_name.to_string(),
        phase: "gate_pending".into(),
        artifact: artifact.map(artifact_str),
        mode: Some(mode_str(cycle.mode)),
        diagnostics: vec![],
        gate_hint: Some(gate_hint(change_name)),
    }))
}

/// Authors one artifact by running an ACP turn that asks the agent to write it.
async fn author_artifact(
    state: &Arc<DaemonState>,
    peer: &Peer,
    project_root: &Path,
    change_dir: &Path,
    artifact: Artifact,
    rework_comment: Option<&str>,
) -> Result<(), RpcError> {
    let target = change_dir.join(artifact.file_name());
    if artifact == Artifact::Specs {
        std::fs::create_dir_all(&target).map_err(RpcError::internal)?;
    }
    let rework = rework_comment
        .map(|c| format!("\nREWORK INSTRUCTION: {c}\n"))
        .unwrap_or_default();
    let prompt = format!(
        "Author the `{}` artifact of this change into the path below, following \
         the Meltemi artifact format.{rework}\nARTIFACT: {}\nARTIFACT_PATH: {}\n",
        artifact_str(artifact),
        artifact_str(artifact),
        target.display(),
    );
    run_turn(state, peer, project_root, prompt, allow_meltemi_writes()).await?;
    Ok(())
}

/// Validates a drafted artifact with the spec engine. `specs` are validated as
/// EARS deltas (structure + at least one scenario per requirement); the prose
/// artifacts require only non-emptiness.
fn validate_artifact(change_dir: &Path, artifact: Artifact) -> Result<(), Vec<String>> {
    match artifact {
        Artifact::Specs => {
            let specs_dir = change_dir.join("specs");
            let mut diagnostics = Vec::new();
            let mut found_any = false;
            if let Ok(entries) = std::fs::read_dir(&specs_dir) {
                for capability in entries.filter_map(Result::ok) {
                    let spec_file = capability.path().join("spec.md");
                    if !spec_file.is_file() {
                        continue;
                    }
                    found_any = true;
                    if let Ok(spec) = meltemi_spec::parse_spec_file(&spec_file) {
                        for d in meltemi_spec::validate_spec(&spec) {
                            diagnostics.push(format!("{}: {}", spec.capability, d.message));
                        }
                    }
                }
            }
            if !found_any {
                diagnostics.push("no delta spec was written under specs/".into());
            }
            if diagnostics.is_empty() {
                Ok(())
            } else {
                Err(diagnostics)
            }
        }
        prose => {
            let path = change_dir.join(prose.file_name());
            match std::fs::read_to_string(&path) {
                Ok(text) if !text.trim().is_empty() => Ok(()),
                _ => Err(vec![format!("{} is missing or empty", prose.file_name())]),
            }
        }
    }
}

/// Runs one ACP authoring turn with a given rule posture, logging to a session.
async fn run_turn(
    state: &Arc<DaemonState>,
    peer: &Peer,
    project_root: &Path,
    prompt: String,
    rules: RuleSet,
) -> Result<(), RpcError> {
    let config = Config::load(&state.config_dir, Some(project_root));
    // An invalid `[permissions]` value kept its default; say so (espera-humana).
    for diagnostic in &config.permission_diagnostics {
        tracing::warn!(diagnostic, "permissions config");
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let bundled = crate::fleet::bundled_dir();
    let (agent_command, level) =
        match crate::levels::resolve_launch(&config, &path_var, bundled.as_deref())? {
            crate::levels::Launch::Acp { argv, level } => (argv, level),
            _ => {
                return Err(RpcError::application(
                    error_codes::AGENT_COMMAND_NOT_CONFIGURED,
                    "level not pilotable",
                    "level_not_pilotable",
                    "SDD authoring needs an ACP-capable agent (level 1 or 2)",
                    None,
                ));
            }
        };

    let session_id = uuid::Uuid::new_v4().to_string();
    let reg = state
        .sessions
        .register(&session_id, agent_command.clone())
        .await;
    crate::projects::touch(&state.data_dir, project_root);
    let project_key = crate::paths::project_key(project_root);
    let mut log = SessionLog::create(&state.data_dir, &project_key, &session_id)
        .map_err(RpcError::internal)?
        .streaming(state.events.clone(), peer.connection_id(), &session_id);
    let _ = log.append(SessionEventKind::SessionStarted {
        session_id: session_id.clone(),
        agent_command: agent_command.clone(),
        project_root: project_root.display().to_string(),
    });
    let log = Arc::new(Mutex::new(log));
    state
        .sessions
        .set_state(&session_id, SessionState::Active)
        .await;

    // The start record: a crash mid-turn must keep listing as interrupted
    // (that is what the state means), and the end record below completes it —
    // the same shape as `propose`.
    let started_at = crate::clock::now_rfc3339();
    let _ = crate::session_index::append(
        &state.data_dir,
        &project_key,
        &crate::session_index::SessionRecord {
            session_id: session_id.clone(),
            agent_command: agent_command.clone(),
            project_root: project_root.display().to_string(),
            level,
            started_at: started_at.clone(),
            ended_at: None,
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            // Authoring runs the project-configured agent, never a profile.
            agent_id: config.agent_id.clone(),
            profile: None,
        },
    );

    let edit_scope = state.edits.enter(project_root, &session_id, log.clone());
    let outcome = acp::run_session(SessionParams {
        agent_command: agent_command.clone(),
        project_root: project_root.to_path_buf(),
        prompt,
        meltemi_session_id: session_id.clone(),
        peer: peer.clone(),
        log: log.clone(),
        cancel: reg.cancel,
        cancelled: reg.cancelled,
        wait: config.interactive_wait(),
        no_client_grace: config.no_client_grace(),
        clients: state.clients.clone(),
        sessions: state.sessions.clone(),
        rules: Arc::new(rules),
        pending: state.pending.clone(),
        load_session_id: None,
        mcp_servers: config.mcp_servers.clone(),
        env: Vec::new(),
        // Authoring turns (`explore`/`plan`/`constitution`) are single-turn and
        // not directable.
        instruction_queue: None,
        edit_scope: Some(edit_scope.handle()),
    })
    .await;

    // Finalize through the shared tail so an authoring turn ends like any
    // other single-turn run: terminal events in the log, end record in the
    // index, deregister. Without this, every completed authoring session
    // listed as interrupted and its active time never counted.
    let project_root_str = project_root.display().to_string();
    let ctx = crate::session_finalize::SessionContext {
        data_dir: &state.data_dir,
        sessions: &state.sessions,
        log: &log,
        project_key: &project_key,
        session_id: &session_id,
        agent_command: &agent_command,
        project_root: &project_root_str,
        level,
        started_at: &started_at,
        resumed_from: None,
        agent_id: config.agent_id.clone(),
        profile: None,
    };
    match outcome {
        Ok(session_outcome) => {
            let _ = crate::session_finalize::finalize_ok(&ctx, session_outcome).await;
            Ok(())
        }
        Err(e) => {
            crate::session_finalize::finalize_err(&ctx, "agent_session_failed", e.to_string())
                .await;
            Err(RpcError::internal(format!("authoring turn failed: {e}")))
        }
    }
}

/// A rule posture that allows writes under `.meltemi/` (typical authoring) but
/// leaves everything else to escalate.
fn allow_meltemi_writes() -> RuleSet {
    RuleSet::allow_all()
}

fn parse<T: serde::de::DeserializeOwned>(params: Value, method: &str) -> Result<T, RpcError> {
    serde_json::from_value(params).map_err(|e| RpcError::invalid_params(format!("{method}: {e}")))
}

fn project_dir(root: &str) -> Result<PathBuf, RpcError> {
    let path = PathBuf::from(root);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{root}` is not an existing directory"),
            Some("Pass the absolute path to an existing repository root.".into()),
        ))
    }
}

fn not_found_change(name: &str) -> RpcError {
    RpcError::invalid_params(format!("no change `{name}` in authoring"))
}

fn done(change_name: &str, phase: &str, mode: Mode) -> SddResult {
    SddResult {
        change_name: change_name.to_string(),
        phase: phase.into(),
        artifact: None,
        mode: Some(mode_str(mode)),
        diagnostics: vec![],
        gate_hint: None,
    }
}

fn gate_hint(change_name: &str) -> String {
    format!(
        "decide with `meltemi` sdd/gate {{change: {change_name}, decision: approve|comment|abort}}"
    )
}

fn artifact_str(a: Artifact) -> String {
    a.as_str().to_string()
}

fn mode_str(m: Mode) -> String {
    match m {
        Mode::SpecFull => "spec_full",
        Mode::FastForward => "fast_forward",
    }
    .to_string()
}

fn result(r: SddResult) -> Value {
    serde_json::to_value(r).expect("SddResult serializes")
}

/// A stable fingerprint of the `.meltemi/` tree, to prove `explore` wrote
/// nothing (paths + sizes; content is not needed for the guard).
fn tree_fingerprint(root: &Path) -> Vec<(String, u64)> {
    let mut entries = Vec::new();
    let meltemi = root.join(".meltemi");
    collect(&meltemi, &meltemi, &mut entries);
    entries.sort();
    entries
}

fn collect(base: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(base, &path, out);
        } else if let Ok(meta) = entry.metadata() {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            out.push((rel.to_string_lossy().into_owned(), meta.len()));
        }
    }
}
