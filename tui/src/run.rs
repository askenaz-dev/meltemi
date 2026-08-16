// SPDX-License-Identifier: Apache-2.0

//! Command execution and the command↔RPC mapping (design D5).
//!
//! Every RPC-backed subcommand reuses the daemon's on-demand bootstrap and the
//! JSON-RPC peer, sends `initialize` first, then its method. Connection failure
//! maps to [`ExitCode::Unreachable`]; a contract/RPC error maps to
//! [`ExitCode::Contract`]. The endpoint is injected so the mapping is testable
//! against an ephemeral in-process daemon.

use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use meltemi_client::bootstrap;
use meltemi_client::rpc::{Incoming, Peer, RpcError};
use meltemi_proto::{
    ChangeListParams, ChangeListResult, ChangeShowParams, ChangeShowResult, SddValidateParams,
    SddValidateResult, SpecListParams, SpecListResult, SpecShowParams, SpecShowResult,
};
use meltemi_proto::{
    CheckpointListParams, CheckpointListResult, CheckpointRevertParams, CheckpointRevertResult,
    CommitTaskParams, CommitTaskResult, SddArchiveParams, SddArchiveResult, SddImplementParams,
    SddImplementResult, SddVerifyParams, SddVerifyResult,
};
use meltemi_proto::{
    ContextProjectParams, ContextProjectResult, FleetListParams, FleetListResult, InitializeParams,
    PROTOCOL_VERSION, PeerInfo, PermissionOutcome, PermissionRequestResult, ProposeParams,
    ProposeResult, SessionListParams, SessionListResult, StatusResult, WorktreeAssignParams,
    WorktreeAssignResult, WorktreeDiffParams, WorktreeDiffResult, WorktreeListParams,
    WorktreeListResult, WorktreeTask, methods,
};
use meltemi_proto::{
    EditLogDestination, TreeEditState, WorktreeApplyEditParams, WorktreeApplyEditResult,
    WorktreeDispatchParams, WorktreeDispatchResult,
};

use crate::cli::Command;
use crate::format::{Paint, paint};
use crate::output::{CliError, Outcome};

/// Executes an RPC-backed subcommand against the daemon at `endpoint`.
/// Runs one command. `painting` is whether the human rendering may carry
/// colour — decided once, from the format, the flag, the environment and the
/// terminal (`crate::format::paints`), and passed down because the human text
/// is composed here (salida-que-se-lee D5).
pub async fn execute(
    command: Command,
    endpoint: &str,
    painting: bool,
) -> Result<Outcome, CliError> {
    match command {
        Command::Status => status(endpoint).await,
        Command::Propose {
            idea,
            project_root,
            agent,
        } => propose(idea, project_root, agent, endpoint).await,
        Command::Session {
            instruction,
            project_root,
            agent,
            mode,
        } => session(instruction, project_root, agent, mode, endpoint).await,
        Command::Fleet => fleet(endpoint).await,
        Command::Link { agent, name } => link(agent, name, endpoint).await,
        Command::Unlink { name } => unlink(name, endpoint).await,
        Command::Project { project_root } => project(project_root, endpoint).await,
        Command::Sessions { project_root } => sessions(project_root, endpoint).await,
        Command::Explore { topic, agent } => {
            sdd_verb(endpoint, methods::SDD_EXPLORE, topic, None, agent).await
        }
        Command::Plan { change } => {
            sdd_verb(
                endpoint,
                methods::SDD_PLAN,
                String::new(),
                Some(change),
                None,
            )
            .await
        }
        Command::Constitution { topic } => {
            sdd_verb(endpoint, methods::SDD_CONSTITUTION, topic, None, None).await
        }
        Command::Review { change } => review(change, endpoint).await,
        Command::Worktrees { project_root } => worktrees(project_root, endpoint).await,
        Command::Assign {
            change,
            task,
            agents,
            project_root,
        } => assign(change, task, agents, project_root, endpoint).await,
        Command::Race {
            change,
            task,
            project_root,
        } => race(change, task, project_root, endpoint).await,
        Command::Dispatch {
            change,
            task,
            agent,
            project_root,
        } => dispatch(change, task, agent, project_root, endpoint).await,
        Command::ApplyEdit {
            file,
            target,
            confirm,
        } => apply_edit(file, target, confirm, endpoint).await,
        Command::Checkpoints { change } => checkpoints(change, endpoint).await,
        Command::Revert {
            change,
            task,
            agent,
            confirm,
        } => revert(change, task, agent, confirm, endpoint).await,
        Command::Commit {
            change,
            task,
            agent,
            title,
            confirm,
        } => commit(change, task, agent, title, confirm, endpoint).await,
        Command::Verify { change } => verify(change, endpoint).await,
        Command::Archive { change, confirm } => archive(change, confirm, endpoint).await,
        Command::Implement {
            change,
            agent,
            plan_only,
        } => implement(change, agent, plan_only, endpoint).await,
        Command::Projects => projects(endpoint).await,
        Command::ProjectRegister { path } => project_register(path, endpoint).await,
        Command::ProjectForget { path } => project_forget(path, endpoint).await,
        Command::Usage {
            project_root,
            granularity,
            since,
            until,
        } => usage(project_root, granularity, since, until, endpoint).await,
        Command::Changes => changes(endpoint, painting).await,
        Command::Show { change } => show(change, endpoint).await,
        Command::Specs { capability } => specs(capability, endpoint, painting).await,
        Command::Validate { change } => validate(change, endpoint).await,
        Command::Direct {
            session,
            instruction,
            project_root,
        } => direct(session, instruction, project_root, endpoint).await,
        // `tunnel` is a local formatter: it never touches the daemon.
        Command::Tunnel { target, exec } => tunnel(target, exec),
        // The bridge writes the daemon's bytes to the process's own locked
        // stdout, so the dispatcher handles it where that writer lives.
        Command::Bridge => Err(CliError::internal(
            "`bridge` is dispatched with the process writer, not through `execute`",
        )),
        Command::Stop => stop(endpoint).await,
        // Reserved subcommands are handled by the dispatcher before reaching
        // here; this arm keeps `execute` total.
        Command::Reserved(name) => Err(CliError::internal(format!(
            "reserved subcommand `{name}` is not executable"
        ))),
    }
}

/// Connects (starting the daemon on demand), spawns the background handler for
/// daemon-initiated traffic, and performs the mandatory `initialize`.
async fn connect_and_init(endpoint: &str) -> Result<(Peer, JoinHandle<()>), CliError> {
    let stream = bootstrap::connect_or_start(endpoint)
        .await
        .map_err(CliError::unreachable)?;
    let (peer, incoming) = Peer::start(stream);
    let background = tokio::spawn(handle_incoming(peer.clone(), incoming));

    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .map_err(CliError::contract)?;

    Ok((peer, background))
}

/// Answers daemon-initiated requests. In scriptable mode there is no
/// interactive approver yet (#6/#9), so permission requests are denied by
/// default — the safe posture. Rich permission UX arrives in a later change.
async fn handle_incoming(peer: Peer, mut incoming: UnboundedReceiver<Incoming>) {
    while let Some(message) = incoming.recv().await {
        match message {
            Incoming::Request { id, method, .. } if method == methods::PERMISSION_REQUEST => {
                eprintln!(
                    "meltemi: agent requested a permission; denied by default \
                     (interactive approval arrives in a later release)"
                );
                let result = PermissionRequestResult {
                    outcome: PermissionOutcome::Cancelled,
                };
                peer.respond(
                    id,
                    Ok(serde_json::to_value(result).expect("decision serializes")),
                );
            }
            Incoming::Request { id, method, .. } => {
                peer.respond(id, Err(RpcError::method_not_found(&method)));
            }
            Incoming::Notification { .. } => {}
        }
    }
}

async fn status(endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(methods::STATUS, &json!({})).await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let status: StatusResult = serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_status(&status),
        json: value,
    })
}

async fn propose(
    idea: String,
    project_root: Option<String>,
    agent: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => root,
        None => std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string(),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::PROPOSE,
            &ProposeParams {
                idea,
                project_root,
                agent,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ProposeResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_propose(&result),
        json: value,
    })
}

/// `session`: start a free session on a project (`session/start`). No change,
/// no task, no specification and no gate — and nothing of the government
/// relaxed to get there. The call blocks until the turn ends, like every one of
/// its siblings, so what comes back is final: the session's id, how the turn
/// ended, how many permissions were denied, and the restore point or the reason
/// there is none.
async fn session(
    instruction: String,
    project_root: Option<String>,
    agent: Option<String>,
    mode: Option<meltemi_proto::AutonomyMode>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SESSION_START,
            &meltemi_proto::SessionStartParams {
                // The scriptable verb waits for the outcome, so it is not
                // staying: the session ends with its turn, exactly as before
                // (sesion-que-espera design D3).
                detach: false,
                mode,
                project_root,
                instruction,
                agent,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: meltemi_proto::SessionStartResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_session(&result),
        json: value,
    })
}

/// Human rendering of a free session: the id first (it is what every other verb
/// takes as an argument), the turn's outcome, and the restore point — or its
/// absence with the remedy that fits the cause, never the other cause's.
fn render_session(result: &meltemi_proto::SessionStartResult) -> String {
    use meltemi_proto::CheckpointUnavailable;
    // A status is printed only when there is one. The CLI never detaches, so in
    // practice this is always the reassuring branch; the other exists so that a
    // result without a status prints "started" rather than a word invented for
    // a turn that has not happened (sesion-que-espera design D3).
    let mut out = format!(
        "session {} [{}]\n{}",
        result.session_id,
        result
            .status
            .map_or_else(|| "started".to_string(), status_word),
        result.agent_command.join(" ")
    );
    match (&result.checkpoint_ref, result.checkpoint_unavailable) {
        (Some(git_ref), _) => out.push_str(&format!("\nrestore point: {git_ref}")),
        (None, cause) => {
            let word = match cause {
                Some(CheckpointUnavailable::NotAGitRepo) => "this root is not a git repository",
                Some(CheckpointUnavailable::NoHistory) => "this repository has no history yet",
                None => "the daemon reported none",
            };
            out.push_str(&format!("\nno restore point: {word}"));
            if let Some(remedy) = &result.checkpoint_remedy {
                out.push_str(&format!("\n  remedy: {remedy}"));
            }
        }
    }
    if result.denied_permissions > 0 {
        out.push_str(&format!(
            "\nwarning: {} permission request(s) denied — the work may be incomplete",
            result.denied_permissions
        ));
    }
    out
}

/// `link`: link a subscription of a catalog agent (`subscription/link`) and
/// print the composed login gesture — the one thing the user must run.
async fn link(agent: String, name: String, endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SUBSCRIPTION_LINK,
            &json!({ "agent": agent, "name": name }),
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let gesture = &value["gesture"];
    let human = format!(
        "linked `{}` -> {}
authenticate it once, then it is ready:
  PowerShell: {}
  POSIX:      {}",
        value["profile"].as_str().unwrap_or("?"),
        value["agent"].as_str().unwrap_or("?"),
        gesture["powershell"].as_str().unwrap_or("?"),
        gesture["posix"].as_str().unwrap_or("?"),
    );
    Ok(Outcome { human, json: value })
}

/// `unlink`: undo a linked subscription (`subscription/unlink`). The context
/// directory is never deleted, and the output says where it stays.
async fn unlink(name: String, endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::SUBSCRIPTION_UNLINK, &json!({ "name": name }))
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let human = format!(
        "unlinked `{}` — its auth context stays at {} (not deleted; remove it yourself if you mean to)",
        value["profile"].as_str().unwrap_or("?"),
        value["contextDir"].as_str().unwrap_or("?"),
    );
    Ok(Outcome { human, json: value })
}

async fn fleet(endpoint: &str) -> Result<Outcome, CliError> {
    // The current directory names the project whose config marks the
    // configured agent (same default as `propose`).
    let project_root = std::env::current_dir()
        .ok()
        .map(|root| root.display().to_string());

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::FLEET_LIST, &FleetListParams { project_root })
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: FleetListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_fleet(&result),
        json: value,
    })
}

async fn project(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => root,
        None => std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string(),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CONTEXT_PROJECT,
            &ContextProjectParams { project_root },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ContextProjectResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_project(&result),
        json: value,
    })
}

async fn sessions(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = match project_root {
        Some(root) => Some(root),
        None => std::env::current_dir()
            .ok()
            .map(|root| root.display().to_string()),
    };

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SESSION_LIST,
            &SessionListParams {
                project_root,
                ..SessionListParams::default()
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SessionListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_sessions(&result),
        json: value,
    })
}

/// Drives an SDD verb (`explore`/`plan`/`constitution`) using the current
/// directory as the project root. Gates are reported, never awaited.
async fn sdd_verb(
    endpoint: &str,
    method: &str,
    topic: String,
    change: Option<String>,
    agent: Option<String>,
) -> Result<Outcome, CliError> {
    let project_root = std::env::current_dir()
        .map_err(CliError::internal)?
        .display()
        .to_string();
    let mut params = match &change {
        Some(change) => json!({ "projectRoot": project_root, "changeName": change }),
        None => json!({ "projectRoot": project_root, "topic": topic }),
    };
    // Additive: absent behaves exactly as it did before the field existed, so
    // the verbs that take no agent send no key at all.
    if let Some(agent) = agent {
        params["agent"] = serde_json::Value::String(agent);
    }

    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(method, &params).await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let phase = value["phase"].as_str().unwrap_or("done");
    let mut human = format!("sdd: {phase}");
    if let Some(artifact) = value["artifact"].as_str() {
        human.push_str(&format!(" — artifact `{artifact}`"));
    }
    if let Some(hint) = value["gateHint"].as_str() {
        human.push_str(&format!("\n{hint}"));
    }
    if let Some(diags) = value["diagnostics"].as_array() {
        for d in diags {
            if let Some(d) = d.as_str() {
                human.push_str(&format!("\n  diagnostic: {d}"));
            }
        }
    }
    Ok(Outcome { human, json: value })
}

/// Reports the review checklist of a change (scriptable; `--json` emits the
/// items and their states, never awaiting interactive input).
async fn review(change: String, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = std::env::current_dir()
        .map_err(CliError::internal)?
        .display()
        .to_string();
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_REVIEW,
            &json!({ "projectRoot": project_root, "changeName": change }),
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    use std::fmt::Write;
    let mut human = format!(
        "review `{change}` — {} pending, {}",
        value["pending"].as_u64().unwrap_or(0),
        if value["canClose"].as_bool().unwrap_or(false) {
            "closable"
        } else {
            "not closable (decide all items)"
        }
    );
    if let Some(items) = value["items"].as_array() {
        for item in items {
            let _ = write!(
                human,
                "\n  [{}] {}/{}",
                item["state"].as_str().unwrap_or("?"),
                item["capability"].as_str().unwrap_or(""),
                item["requirement"].as_str().unwrap_or("")
            );
        }
    }
    Ok(Outcome { human, json: value })
}

/// Resolves an optional project root to the current directory.
fn cwd_or(project_root: Option<String>) -> Result<String, CliError> {
    match project_root {
        Some(root) => Ok(root),
        None => Ok(std::env::current_dir()
            .map_err(CliError::internal)?
            .display()
            .to_string()),
    }
}

async fn worktrees(project_root: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(methods::WORKTREE_LIST, &WorktreeListParams { project_root })
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_worktrees(&result),
        json: value,
    })
}

async fn assign(
    change: String,
    task: String,
    agents: Vec<String>,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_ASSIGN,
            &WorktreeAssignParams {
                project_root,
                tasks: vec![WorktreeTask {
                    change,
                    task,
                    agents,
                    files: Vec::new(),
                }],
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeAssignResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_assign(&result),
        json: value,
    })
}

async fn race(
    change: String,
    task: String,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_DIFF,
            &WorktreeDiffParams {
                project_root,
                change,
                task,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeDiffResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_race(&result),
        json: value,
    })
}

async fn checkpoints(change: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHECKPOINT_LIST,
            &CheckpointListParams {
                project_root,
                change,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: CheckpointListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_checkpoints(&result),
        json: value,
    })
}

async fn revert(
    change: String,
    task: String,
    agent: String,
    confirm: bool,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHECKPOINT_REVERT,
            &CheckpointRevertParams {
                project_root,
                change,
                task,
                agent,
                confirm,
            },
        )
        .await;
    peer.close();
    background.abort();

    match response {
        Ok(value) => {
            let result: CheckpointRevertResult =
                serde_json::from_value(value.clone()).map_err(CliError::internal)?;
            Ok(Outcome {
                human: render_revert(&result),
                json: value,
            })
        }
        // Without `confirm`, the daemon refuses with the honest scope. That is
        // the preview, not a failure: render it and exit successfully.
        Err(err) if !confirm && err.code == meltemi_proto::error_codes::WORKTREE_REFUSED => {
            let detail = err
                .data
                .as_ref()
                .and_then(|d| d["detail"].as_str())
                .unwrap_or(&err.message)
                .to_string();
            Ok(Outcome {
                human: format!(
                    "preview (nothing reverted) — {detail}\nRe-run with a trailing `confirm` to revert."
                ),
                json: json!({ "preview": true, "detail": detail }),
            })
        }
        Err(err) => Err(CliError::contract(err)),
    }
}

async fn commit(
    change: String,
    task: String,
    agent: String,
    title: String,
    confirm: bool,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::COMMIT_TASK,
            &CommitTaskParams {
                project_root,
                change,
                task,
                agent,
                title,
                body: None,
                requirements: Vec::new(),
                declared_files: Vec::new(),
                confirm,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: CommitTaskResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_commit(&result),
        json: value,
    })
}

async fn verify(change: String, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_VERIFY,
            &SddVerifyParams {
                project_root,
                change,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SddVerifyResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_verify(&result),
        json: value,
    })
}

async fn archive(change: String, confirm: bool, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_ARCHIVE,
            &SddArchiveParams {
                project_root,
                change,
                confirm,
                exceptions: Vec::new(),
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SddArchiveResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_archive(&result),
        json: value,
    })
}

async fn implement(
    change: String,
    agent: String,
    plan_only: bool,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_IMPLEMENT,
            &SddImplementParams {
                project_root,
                change,
                agent,
                plan_only,
                autonomous: false,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SddImplementResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_implement(&result),
        json: value,
    })
}

async fn dispatch(
    change: String,
    task: String,
    agent: String,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_DISPATCH,
            &WorktreeDispatchParams {
                // The CLI's dispatch verb takes no mode yet: it would need its
                // own flag, and this change wires the flag onto `session`.
                mode: None,
                project_root,
                change,
                task,
                agent,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeDispatchResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_dispatch(&result),
        json: value,
    })
}

/// `apply-edit`: a traceable human edit through the daemon; the new file
/// content is read whole from stdin (scriptable surface of edit-surface).
async fn apply_edit(
    file: String,
    target: Option<(String, String, String)>,
    confirm: bool,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let content = std::io::read_to_string(std::io::stdin()).map_err(CliError::internal)?;
    let project_root = cwd_or(None)?;
    let (change, task, agent) = match target {
        Some((change, task, agent)) => (Some(change), Some(task), Some(agent)),
        None => (None, None, None),
    };
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::WORKTREE_APPLY_EDIT,
            &WorktreeApplyEditParams {
                project_root,
                change,
                task,
                agent,
                file,
                content,
                confirm,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: WorktreeApplyEditResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_apply_edit(&result),
        json: value,
    })
}

fn render_apply_edit(result: &WorktreeApplyEditResult) -> String {
    let state = match result.tree_state {
        TreeEditState::Free => "free",
        TreeEditState::SessionActive => "session active",
        TreeEditState::TurnInFlight => "turn in flight",
    };
    let destination = match result.logged_to {
        EditLogDestination::Session => "session log",
        EditLogDestination::Project => "project edits log",
    };
    format!(
        "wrote {} ({} bytes) — tree {}; human_edit -> {}",
        result.file, result.bytes_written, state, destination
    )
}

/// `projects`: the repositories this user has pointed Meltemi at, most
/// recently seen first — the catalog the surfaces build their tree from.
async fn projects(endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::PROJECT_LIST,
            &meltemi_proto::ProjectListParams::default(),
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: meltemi_proto::ProjectListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_projects(&result),
        json: value,
    })
}

/// `projects register`: aim Meltemi at a directory explicitly, before anything
/// has ever run in it. The daemon checks the path exists, canonicalizes it and
/// creates nothing inside it — registering is pointing the tool at a folder,
/// not initializing it as a project (design D6).
async fn project_register(path: String, endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::PROJECT_REGISTER,
            &meltemi_proto::ProjectRegisterParams { root: path },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: meltemi_proto::ProjectRegisterResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    let project = &result.project;
    Ok(Outcome {
        human: format!(
            "registered {} — {} session(s), {} live",
            project.root, project.sessions_total, project.active_sessions
        ),
        json: value,
    })
}

/// `projects forget`: one line over the listing, and nothing else. The root
/// need not exist — a directory that vanished is precisely the one worth
/// forgetting — and the answer says out loud what was NOT done, because a
/// command that hides a project is one keystroke away from being read as a
/// command that deletes it.
async fn project_forget(path: String, endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::PROJECT_FORGET,
            &meltemi_proto::ProjectForgetParams { root: path.clone() },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: meltemi_proto::ProjectForgetResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    let human = if result.forgotten {
        format!(
            "forgot {path} — dropped from the project listing.\n\
             Nothing was deleted: its sessions, their logs and the local accounting are \
             untouched, and it reappears the moment the project is used or registered again."
        )
    } else {
        format!("{path} was not in the project listing — nothing changed")
    };
    Ok(Outcome { human, json: value })
}

fn render_projects(result: &meltemi_proto::ProjectListResult) -> String {
    use std::fmt::Write;
    if result.projects.is_empty() {
        return "no projects recorded yet — run a session in a repository".into();
    }
    let mut out = format!("{} project(s), most recent first", result.projects.len());
    for project in &result.projects {
        let mark = if project.exists { "present" } else { "missing" };
        let _ = write!(
            out,
            "
  {mark:<8} {:>3} session(s), {:>2} live, {:>2} resumable  {}",
            project.sessions_total,
            project.active_sessions,
            project.resumable_sessions,
            project.root
        );
    }
    out
}

/// `usage`: local accounting folded from the session records. Tokens are only
/// what an official agent output reported; the disclosure travels with them.
async fn usage(
    project_root: Option<String>,
    granularity: Option<String>,
    since: Option<String>,
    until: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    use meltemi_proto::UsageGranularity;
    let granularity = match granularity.as_deref() {
        None => None,
        Some("day") => Some(UsageGranularity::Day),
        Some("week") => Some(UsageGranularity::Week),
        Some("month") => Some(UsageGranularity::Month),
        Some("total") => Some(UsageGranularity::Total),
        Some(other) => {
            return Err(CliError::usage(format!(
                "unknown granularity `{other}`: use day, week, month or total"
            )));
        }
    };
    // An empty root means "this project"; `--all` sent None.
    let project_root = match project_root {
        Some(root) if root.is_empty() => Some(cwd_or(None)?),
        other => other,
    };
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::ANALYTICS_USAGE,
            &meltemi_proto::AnalyticsUsageParams {
                project_root,
                since,
                until,
                granularity,
                agent: None,
                profile: None,
                limit: None,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: meltemi_proto::AnalyticsUsageResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_usage(&result),
        json: value,
    })
}

/// The disclosure line for a stable key (the CLI's own catalog, §11: the
/// daemon returns keys, each surface writes the words).
fn disclosure_line(key: meltemi_proto::UsageDisclosure) -> &'static str {
    use meltemi_proto::UsageDisclosure as D;
    match key {
        D::ActivityFromLocalRecords => "activity is folded from your local session records",
        D::TokensOnlyWhenOfficialOutputReports => {
            "tokens are counted only where the agent's official output reports them"
        }
        D::NoQuotaBalanceOrBilling => {
            "quota, balance and billing of your provider account are not visible here"
        }
        D::NothingIsEstimated => "nothing is estimated: a counter nobody reported stays absent",
        D::NothingLeavesThisMachine => "no data leaves this machine",
    }
}

/// A measured counter, or the honest absence marker — never a zero standing in
/// for "unknown".
fn measured(value: Option<u64>) -> String {
    match value {
        Some(count) => count.to_string(),
        None => "- not reported".to_string(),
    }
}

fn render_usage(result: &meltemi_proto::AnalyticsUsageResult) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    if result.cells.is_empty() {
        out.push_str("no records for this query — nothing is inferred");
    } else {
        let _ = write!(out, "{} cell(s)", result.cells.len());
        if result.truncated {
            out.push_str(" (truncated by --limit)");
        }
        for cell in &result.cells {
            let profile = cell.profile.as_deref().unwrap_or("-");
            let _ = write!(
                out,
                "
  {:<10} {:<14} {:<12} {:>3} session(s) {:>6}s  in {}  out {}",
                cell.period,
                cell.agent,
                profile,
                cell.activity.sessions,
                cell.activity.active_seconds,
                measured(cell.tokens.as_ref().and_then(|t| t.input)),
                measured(cell.tokens.as_ref().and_then(|t| t.output)),
            );
            if cell.coverage.unreported_sessions > 0 {
                let reasons: Vec<String> = cell
                    .coverage
                    .reasons
                    .iter()
                    .map(|reason| {
                        let kind = serde_json::to_value(reason.kind)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| "unknown".into());
                        format!("{} x {kind}", reason.sessions)
                    })
                    .collect();
                let _ = write!(
                    out,
                    "
             coverage: {} measured, {} without data ({})",
                    cell.coverage.measured_sessions,
                    cell.coverage.unreported_sessions,
                    reasons.join(", ")
                );
            }
        }
    }
    for unattributed in &result.unattributed {
        let _ = write!(
            out,
            "
  unattributed {}: {} human edit(s) with no session",
            unattributed.period, unattributed.human_edits
        );
    }
    // The disclosure travels with the numbers, never behind a flag (design D6).
    for key in &result.disclosure {
        let _ = write!(
            out,
            "
  * {}",
            disclosure_line(*key)
        );
    }
    out
}

async fn changes(endpoint: &str, painting: bool) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHANGE_LIST,
            &ChangeListParams {
                project_root,
                limit: None,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ChangeListResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_changes(&result, painting),
        json: value,
    })
}

async fn show(change: String, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::CHANGE_SHOW,
            &ChangeShowParams {
                project_root,
                change,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: ChangeShowResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_change_show(&result),
        json: value,
    })
}

async fn specs(
    capability: Option<String>,
    endpoint: &str,
    painting: bool,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    // No capability -> list; a capability -> show that one.
    let (value, human) = match capability {
        None => {
            let response = peer
                .request(methods::SPEC_LIST, &SpecListParams { project_root })
                .await;
            peer.close();
            background.abort();
            let value = response.map_err(CliError::contract)?;
            let result: SpecListResult =
                serde_json::from_value(value.clone()).map_err(CliError::internal)?;
            (value.clone(), render_spec_list(&result, painting))
        }
        Some(capability) => {
            let response = peer
                .request(
                    methods::SPEC_SHOW,
                    &SpecShowParams {
                        project_root,
                        capability,
                    },
                )
                .await;
            peer.close();
            background.abort();
            let value = response.map_err(CliError::contract)?;
            let result: SpecShowResult =
                serde_json::from_value(value.clone()).map_err(CliError::internal)?;
            (value.clone(), render_spec_show(&result))
        }
    };
    Ok(Outcome { human, json: value })
}

async fn validate(change: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
    let project_root = cwd_or(None)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SDD_VALIDATE,
            &SddValidateParams {
                project_root,
                change,
            },
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    let result: SddValidateResult =
        serde_json::from_value(value.clone()).map_err(CliError::internal)?;
    Ok(Outcome {
        human: render_validate(&result),
        json: value,
    })
}

async fn stop(endpoint: &str) -> Result<Outcome, CliError> {
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer.request(methods::SHUTDOWN, &json!({})).await;
    peer.close();
    background.abort();

    response.map_err(CliError::contract)?;
    Ok(Outcome {
        human: "daemon shutdown requested".into(),
        json: json!({ "shutdown": "requested" }),
    })
}

/// `direct`: steer an existing session (`session/direct`). Queues the
/// instruction as an active session's next turn, or resumes a resumable one.
async fn direct(
    session: String,
    instruction: String,
    project_root: Option<String>,
    endpoint: &str,
) -> Result<Outcome, CliError> {
    let project_root = cwd_or(project_root)?;
    let (peer, background) = connect_and_init(endpoint).await?;
    let response = peer
        .request(
            methods::SESSION_DIRECT,
            &json!({
                "sessionId": session,
                "instruction": instruction,
                "projectRoot": project_root,
            }),
        )
        .await;
    peer.close();
    background.abort();

    let value = response.map_err(CliError::contract)?;
    Ok(Outcome {
        human: render_direct(&value),
        json: value,
    })
}

fn render_direct(value: &serde_json::Value) -> String {
    match value["disposition"].as_str().unwrap_or("") {
        "queued" => format!(
            "queued — the instruction is turn #{} of session {}",
            value["queuePosition"].as_u64().unwrap_or(0),
            value["sessionId"].as_str().unwrap_or("?"),
        ),
        "resumed" => {
            let denied = value["deniedPermissions"].as_u64().unwrap_or(0);
            let mut out = format!(
                "resumed — session {} continues {} [{}]",
                value["sessionId"].as_str().unwrap_or("?"),
                value["resumedFrom"].as_str().unwrap_or("?"),
                value["status"].as_str().unwrap_or("?"),
            );
            if denied > 0 {
                out.push_str(&format!(
                    "\nwarning: {denied} permission request(s) denied — the turn may be incomplete"
                ));
            }
            out
        }
        _ => "directed".to_string(),
    }
}

/// `tunnel`: compose (or, with `--exec`, run) the `ssh` reverse-forward that
/// exposes this daemon's endpoint to a remote host. Entirely local — it uses the
/// user's own `ssh` and never touches the daemon (control-remoto-asistido D3).
/// The last metre of remote access: connects THIS machine's daemon endpoint
/// and pumps it against stdio until either side closes, so
/// `ssh <pc> meltemi bridge` is a complete JSON-RPC channel — including on
/// Windows, whose named pipe no form of SSH forwarding can reach
/// (acceso-remoto-en-dos-vias D1). No TTY, no prompts: it is built to run
/// under a non-interactive `ssh` exec.
///
/// `out` is the writer the process already locked, and taking it is not a
/// stylistic choice. Rust's stdout lock is a per-process reentrant mutex:
/// `main` holds its guard for the whole dispatch, and an async write through
/// `tokio::io::stdout()` lands on a blocking-pool thread, where the lock is no
/// longer reentrant. Measured against a real daemon: the request crossed, the
/// response was read back from the endpoint, and the write to stdout never
/// returned. Writing through the guard we were given cannot deadlock.
pub async fn bridge(endpoint: &str, out: &mut impl std::io::Write) -> Result<(), CliError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Deliberately NOT `connect_or_start`: a remote connection must not boot
    // daemons implicitly, and the far side of an ssh exec has nobody watching
    // a spinner — refuse immediately with the remedy instead of waiting.
    let stream = meltemi_client::transport::connect(endpoint)
        .await
        .map_err(|e| {
            CliError::unreachable(format!(
                "the local endpoint `{endpoint}` is not accepting connections ({e});                  start the daemon on this machine first (any `meltemi` command boots                  it, e.g. `meltemi status`) and run the bridge again"
            ))
        })?;

    let (mut from_daemon, mut to_daemon) = tokio::io::split(stream);
    let mut stdin = tokio::io::stdin();
    let mut inbound = vec![0u8; 16 * 1024];
    let mut outbound = vec![0u8; 16 * 1024];

    // Every chunk is flushed as it arrives. `tokio::io::copy` would be shorter
    // and wrong: it flushes only at EOF, so a request/response protocol stalls
    // waiting for a buffer that fills only when the conversation continues —
    // which it cannot, because the answer is what is being withheld.
    loop {
        tokio::select! {
            read = stdin.read(&mut inbound) => {
                let read = read.map_err(CliError::internal)?;
                if read == 0 {
                    break;
                }
                to_daemon
                    .write_all(&inbound[..read])
                    .await
                    .map_err(CliError::internal)?;
                to_daemon.flush().await.map_err(CliError::internal)?;
            }
            read = from_daemon.read(&mut outbound) => {
                let read = read.map_err(CliError::internal)?;
                if read == 0 {
                    break;
                }
                out.write_all(&outbound[..read]).map_err(CliError::internal)?;
                out.flush().map_err(CliError::internal)?;
            }
        }
    }
    Ok(())
}

fn tunnel(target: Option<String>, exec: bool) -> Result<Outcome, CliError> {
    let local_endpoint = meltemi_client::paths::endpoint();
    let plan = crate::tunnel::compose(cfg!(windows), &local_endpoint, target.as_deref())
        .map_err(|refusal| CliError::usage(format!("{} — {}", refusal.reason, refusal.remedy)))?;

    if exec {
        // `--exec` runs the user's OWN ssh, visible (inherited stdio), never a
        // silent background tunnel. A placeholder target cannot be dialed.
        let Some(host) = target.as_deref() else {
            return Err(CliError::usage(
                "`tunnel --exec` needs a target: meltemi tunnel <user@host> --exec",
            ));
        };
        let forward = format!("{}:{}", plan.remote_endpoint, plan.local_endpoint);
        let status = std::process::Command::new("ssh")
            .args(["-N", "-R", &forward, host])
            .status()
            .map_err(|e| CliError::internal(format!("could not launch ssh: {e}")))?;
        let code = status.code().unwrap_or(-1);
        return Ok(Outcome {
            human: format!("ran `{}`; ssh exited with status {code}", plan.ssh_command),
            json: json!({
                "sshCommand": plan.ssh_command,
                "executed": true,
                "exitCode": code,
            }),
        });
    }

    let human = format!(
        "Reverse-forward this daemon ({}) to a remote host with your own ssh:\n\n  {}\n\n\
         …or add this to ~/.ssh/config and run `ssh -N meltemi-tunnel`:\n\n{}\n{}",
        plan.local_endpoint, plan.ssh_command, plan.config_snippet, plan.note,
    );
    Ok(Outcome {
        human,
        json: json!({
            "sshCommand": plan.ssh_command,
            "configSnippet": plan.config_snippet,
            "localEndpoint": plan.local_endpoint,
            "remoteEndpoint": plan.remote_endpoint,
            "note": plan.note,
            "executed": false,
        }),
    })
}

/// Human rendering of a propose result: a stable lowercase status word, the
/// normalized artifact path, and — when the turn denied any permission — a
/// visible warning that the proposal may be incomplete (honesty H1/H4/H5).
fn render_propose(result: &ProposeResult) -> String {
    let mut out = format!(
        "proposed `{}` [{}]\n{}",
        result.change_name,
        status_word(result.status),
        result.proposal_path
    );
    if result.denied_permissions > 0 {
        out.push_str(&format!(
            "\nwarning: {} permission request(s) denied — the proposal may be incomplete",
            result.denied_permissions
        ));
    }
    out
}

/// The stable, lowercase word for a turn status (never the capitalized Debug
/// form). Mirrors the contract's snake_case serialization.
fn status_word(status: meltemi_proto::TurnStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Human rendering of the session list: state word, id, agent, and a
/// `resumable` marker, most recent first.
fn render_sessions(result: &SessionListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} session(s)", result.sessions.len());
    for session in &result.sessions {
        let state = session_state_word(session.state);
        let _ = write!(
            out,
            "\n  {state:<12}  {}  {}",
            session.session_id,
            session.agent_command.join(" ")
        );
        if session.resumable {
            let _ = write!(out, "  (resumable)");
        }
    }
    out
}

/// The stable lowercase word for a session state.
fn session_state_word(state: meltemi_proto::SessionState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Human rendering of a context-projection report: which targets were written
/// vs already current, with a short fingerprint.
fn render_project(result: &ContextProjectResult) -> String {
    use std::fmt::Write;
    let written = result.targets.iter().filter(|t| t.written).count();
    let mut out = format!(
        "projected context — {} target(s), {} written",
        result.targets.len(),
        written
    );
    for target in &result.targets {
        let word = if target.written {
            "written    "
        } else {
            "up-to-date "
        };
        let short: String = target.fingerprint.chars().take(12).collect();
        let _ = write!(out, "\n  {word}  {}  {short}", target.path);
    }
    out
}

/// Whether the entry's PILOT layer — the one a launch runs — was found beside
/// the daemon rather than on the user's machine (adaptadores-propios-acp D8).
/// The pilot is the adapter when the entry has one, its single layer otherwise.
fn pilot_is_bundled(layers: &[meltemi_proto::FleetLayer]) -> bool {
    layers
        .iter()
        .find(|layer| layer.kind == meltemi_proto::FleetLayerKind::Adapter)
        .or_else(|| layers.first())
        .is_some_and(|layer| layer.source == Some(meltemi_proto::FleetLayerSource::Bundled))
}

/// Human rendering of the fleet catalog: detection word, declared level,
/// id, name, then the detected path and the configured marker.
fn render_fleet(fleet: &FleetListResult) -> String {
    use std::fmt::Write;
    let detected = fleet.agents.iter().filter(|a| a.detected).count();
    let mut out = String::new();
    let _ = write!(
        out,
        "registry {} — {} agent(s), {} detected",
        fleet.registry_version,
        fleet.agents.len(),
        detected
    );
    let id_width = fleet.agents.iter().map(|a| a.id.len()).max().unwrap_or(0);
    for agent in &fleet.agents {
        let word = if agent.detected {
            "detected    "
        } else {
            "not-detected"
        };
        let verified = match agent.verified_level {
            Some(v) => format!("verified L{v}"),
            None => "unverified".to_string(),
        };
        let _ = write!(
            out,
            "\n  {word}  L{} ({verified})  {:<id_width$}  {}",
            agent.integration_level, agent.id, agent.display_name
        );
        if let Some(path) = &agent.binary_path {
            let _ = write!(out, " — {path}");
            // Where the pilot binary came from, when it came with Meltemi: a
            // path the user never installed deserves to say so
            // (adaptadores-propios-acp D8).
            if pilot_is_bundled(&agent.layers) {
                let _ = write!(out, " (bundled with Meltemi)");
            }
        }
        // Two-layer entries say which layer is missing and how to fix it
        // (flota-deteccion-guia D8): a bare "not detected" is not a diagnosis.
        if let Some(state) = agent.install_state
            && state != meltemi_proto::FleetInstallState::Ready
        {
            let label = match state {
                meltemi_proto::FleetInstallState::AdapterMissing => "adapter missing",
                meltemi_proto::FleetInstallState::CliMissing => "provider CLI missing",
                meltemi_proto::FleetInstallState::NotLaunchable => "installed, not launchable",
                _ => "not detected",
            };
            let _ = write!(out, "\n      state: {label}");
            for layer in &agent.layers {
                let kind = match layer.kind {
                    meltemi_proto::FleetLayerKind::Cli => "cli",
                    meltemi_proto::FleetLayerKind::Adapter => "adapter",
                };
                let found = if layer.detected {
                    if layer.evidence_only {
                        "shim only"
                    } else if layer.source == Some(meltemi_proto::FleetLayerSource::Bundled) {
                        "found (bundled)"
                    } else {
                        "found"
                    }
                } else if layer.bundled {
                    "missing (bundled with Meltemi)"
                } else {
                    "missing"
                };
                let _ = write!(out, "\n      {kind:<8} {:<22} {found}", layer.bin);
                if let Some(path) = &layer.binary_path {
                    let _ = write!(out, " — {path}");
                }
            }
            if let Some(remedy) = &agent.remedy {
                let _ = write!(out, "\n      remedy: {remedy}");
            }
            if let Some(note) = &agent.legal_note {
                let _ = write!(out, "\n      note: {note}");
            }
        }
        // A launch profile shows the underlying agent it runs (flota D4 parity).
        if let Some(underlying) = &agent.underlying_agent {
            let _ = write!(out, " (profile → {underlying})");
        }
        if agent.configured {
            let _ = write!(out, " (configured)");
        }
    }
    out
}

/// Human rendering of the daemon status.
fn render_status(status: &StatusResult) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = write!(
        out,
        "daemon {} — uptime {}s — {} session(s)",
        status.daemon_version,
        status.uptime_seconds,
        status.sessions.len()
    );
    for session in &status.sessions {
        let _ = write!(
            out,
            "\n  {} [{:?}] {}",
            session.session_id,
            session.state,
            session.agent_command.join(" ")
        );
    }
    out
}

/// Human rendering of the managed worktree list: branch, agent and path, with a
/// `race` marker when more than one competes on the same task.
fn render_worktrees(result: &WorktreeListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} managed worktree(s)", result.worktrees.len());
    for w in &result.worktrees {
        let _ = write!(
            out,
            "\n  {}  {}/{}  {}  {}",
            if w.competitor { "race " } else { "solo " },
            w.change,
            w.task,
            w.agent,
            w.path
        );
    }
    out
}

/// Human rendering of an assignment: the fixed base, then each parallel batch
/// (serialized batches show why), and every worktree created.
fn render_assign(result: &WorktreeAssignResult) -> String {
    use std::fmt::Write;
    let short: String = result.base_rev.chars().take(12).collect();
    let mut out = format!(
        "assigned {} worktree(s) from base {short} — {} batch(es)",
        result.worktrees.len(),
        result.batches.len()
    );
    for (i, batch) in result.batches.iter().enumerate() {
        let _ = write!(out, "\n  batch {}: {}", i + 1, batch.tasks.join(", "));
        if let Some(reason) = &batch.serialized_reason {
            let _ = write!(out, "  [{reason}]");
        }
    }
    for w in &result.worktrees {
        let marker = if w.competitor { "race " } else { "solo " };
        let _ = write!(out, "\n  {marker} {}  {}  {}", w.agent, w.branch, w.path);
    }
    out
}

/// Human rendering of a deployment: the mode, an autonomy/degradation notice,
/// and each task's outcome.
fn render_implement(result: &SddImplementResult) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "implement ({} mode) — {} task(s) committed this run",
        result.mode,
        result.committed.len()
    );
    if let Some(reason) = &result.degraded {
        let _ = write!(out, "\n  ! autonomy degraded to supervised: {reason}");
    } else if result.autonomous {
        let _ = write!(out, "\n  autonomous");
    }
    for t in &result.tasks {
        let sha = t
            .sha
            .as_deref()
            .map(|s| format!(" ({})", &s[..s.len().min(12)]))
            .unwrap_or_default();
        let _ = write!(out, "\n  [{}] {} {}{sha}", t.status, t.id, t.description);
    }
    out
}

/// Human rendering of a dispatch: the resolved binary + source, the worktree,
/// the commit outcome, and the explicit "tasks.md untouched" line.
fn render_dispatch(result: &WorktreeDispatchResult) -> String {
    use std::fmt::Write;
    let r = &result.resolution;
    let source = serde_json::to_value(r.source)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    let profile = r
        .profile
        .as_deref()
        .map(|p| format!(" profile={p}"))
        .unwrap_or_default();
    let mut out = format!(
        "dispatch {}/{} [{}] -> {} (source={source}{profile}, L{})",
        result.change, result.task, result.agent, r.binary, r.level
    );
    let _ = write!(out, "\n  worktree: {}", result.worktree);
    if result.committed {
        let sha = result.sha.as_deref().unwrap_or("");
        let _ = write!(
            out,
            "\n  committed {} ({} file(s))",
            &sha[..sha.len().min(12)],
            result.changed_files.len()
        );
    } else {
        let _ = write!(out, "\n  nothing committed ({:?})", result.status);
    }
    let _ = write!(
        out,
        "\n  tasks.md untouched (a competitor does not own the task)"
    );
    out
}

/// Human rendering of the change listing: state columns per change.
///
/// The summary carries the one figure nobody could get from the old listing
/// without reading every row: how many changes are waiting on a human. Colour
/// is decoration over words and figures that already say the same thing
/// (salida-que-se-lee D3, D5).
fn render_changes(result: &ChangeListResult, painting: bool) -> String {
    use std::fmt::Write;
    let active = result.changes.iter().filter(|c| !c.archived).count();
    let waiting = result
        .changes
        .iter()
        .filter(|c| !c.archived && c.gate_pending)
        .count();

    let mut out = paint(painting, Paint::Bold, "Changes");
    let _ = write!(
        out,
        "\n  {} total · {} active · {} archived · {}\n",
        result.changes.len(),
        paint(painting, Paint::Green, &active.to_string()),
        result.changes.len() - active,
        if waiting == 0 {
            "none awaiting you".to_string()
        } else {
            paint(
                painting,
                Paint::Yellow,
                &format!("{waiting} awaiting your decision"),
            )
        }
    );

    // Each counter column is as wide as its widest ratio, so the names below
    // start on one line rather than stepping to the right (design D5).
    let widths = result.changes.iter().filter(|c| !c.archived).fold(
        (1usize, 1usize, 1usize),
        |(t, r, v), c| {
            (
                t.max(format!("{}/{}", c.tasks_done, c.tasks_total).len()),
                r.max(format!("{}/{}", c.review_decided, c.review_total).len()),
                v.max(format!("{}/{}", c.verified, c.verify_total).len()),
            )
        },
    );
    let (tasks_w, review_w, verify_w) = widths;

    for c in &result.changes {
        if c.archived {
            let _ = write!(
                out,
                "\n  {}  {}  {}",
                paint(painting, Paint::Dim, "archived"),
                c.archived_at.as_deref().unwrap_or("—"),
                paint(painting, Paint::Dim, &c.name)
            );
        } else {
            let a = &c.artifacts;
            let art: String = [
                (a.proposal, 'P'),
                (a.design, 'D'),
                (a.specs, 'S'),
                (a.tasks, 'T'),
            ]
            .iter()
            .map(|(present, ch)| if *present { *ch } else { '·' })
            .collect();
            // A pending authoring gate is what awaits a human right now, so it
            // rides with the name instead of hiding in a column (sesion-esperando).
            let gate = match (c.gate_pending, c.gate_artifact.as_deref()) {
                (true, Some(artifact)) => format!("  <- gate: {artifact} awaits you"),
                (true, None) => "  <- gate awaits you".to_string(),
                (false, _) => String::new(),
            };
            // A counter is painted by what its own figures already say: green
            // when it is done, dim when there is nothing to do, plain while it
            // is in progress. Remove the colour and the ratio still says it.
            // Padded BEFORE painting: escapes have no width, so padding a
            // painted string aligns the escapes instead of the digits.
            let counter = |done: u32, total: u32, width: usize| {
                let text = format!("{:width$}", format!("{done}/{total}"));
                if total == 0 {
                    paint(painting, Paint::Dim, &text)
                } else if done >= total {
                    paint(painting, Paint::Green, &text)
                } else {
                    text
                }
            };
            let _ = write!(
                out,
                "\n  {}  {}  tasks {}  review {}  verify {}  {}{}",
                paint(painting, Paint::Green, "active  "),
                paint(painting, Paint::Blue, &art),
                counter(c.tasks_done, c.tasks_total, tasks_w),
                counter(c.review_decided, c.review_total, review_w),
                counter(c.verified, c.verify_total, verify_w),
                // The name is the last column, so it is never padded: trailing
                // spaces are invisible noise a terminal cannot use.
                paint(painting, Paint::Bold, &c.name),
                paint(painting, Paint::Yellow, &gate)
            );
        }
    }
    out
}

/// Human rendering of a change: which artifacts are present and its deltas.
fn render_change_show(result: &ChangeShowResult) -> String {
    use std::fmt::Write;
    let mut out = format!("change `{}`", result.name);
    let _ = write!(
        out,
        "\n  artifacts: {}",
        result
            .artifacts
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for d in &result.deltas {
        let reqs = d.content.matches("### Requirement:").count();
        let _ = write!(out, "\n  delta: {} ({reqs} requirement(s))", d.capability);
    }
    out
}

/// Human rendering of the living-truth capability list.
///
/// Opens with the totals that would otherwise be added by eye, and takes its
/// column widths from the content so thirty-six rows read vertically. Colour
/// marks nothing the figures do not already say (salida-que-se-lee D5).
fn render_spec_list(result: &SpecListResult, painting: bool) -> String {
    use std::fmt::Write;
    let requirements: u32 = result.specs.iter().map(|s| s.requirements).sum();
    let scenarios: u32 = result.specs.iter().map(|s| s.scenarios).sum();

    let mut out = paint(painting, Paint::Bold, "Living truth");
    let _ = write!(
        out,
        "\n  {} capabilities · {requirements} requirements · {scenarios} scenarios\n",
        result.specs.len()
    );

    // Widths from the content, never a fixed number the first long name breaks.
    let name_w = result
        .specs
        .iter()
        .map(|s| s.capability.chars().count())
        .max()
        .unwrap_or(0);
    let req_w = result
        .specs
        .iter()
        .map(|s| s.requirements.to_string().len())
        .max()
        .unwrap_or(1);
    for s in &result.specs {
        // Padded before painting: escapes have no width, and padding a painted
        // string would align the escapes instead of the letters.
        let name = format!("{:name_w$}", s.capability);
        let _ = write!(
            out,
            "\n  {}  {:>req_w$} req  {} scenarios",
            paint(painting, Paint::Cyan, &name),
            s.requirements,
            s.scenarios
        );
    }
    out
}

/// Human rendering of one living capability: its requirements and scenarios.
fn render_spec_show(result: &SpecShowResult) -> String {
    use std::fmt::Write;
    let scenarios: usize = result.requirements.iter().map(|r| r.scenarios.len()).sum();
    let mut out = format!(
        "capability `{}` — {} requirement(s), {scenarios} scenario(s)",
        result.capability,
        result.requirements.len()
    );
    for r in &result.requirements {
        let _ = write!(out, "\n  # {}", r.name);
        for s in &r.scenarios {
            let _ = write!(out, "\n    - {}", s.name);
        }
    }
    out
}

/// Human rendering of a validation: clean, or the findings by capability.
fn render_validate(result: &SddValidateResult) -> String {
    use std::fmt::Write;
    let head = match (&result.scope[..], &result.target) {
        ("change", Some(t)) => format!("validate change `{t}`"),
        _ => "validate living truth".to_string(),
    };
    if result.clean {
        return format!("{head} — clean");
    }
    let mut out = format!("{head} — {} finding(s)", result.diagnostics.len());
    for d in &result.diagnostics {
        let _ = write!(
            out,
            "\n  ! [{}] {} ({})",
            d.capability, d.message, d.location
        );
    }
    out
}

/// Human rendering of a verification checklist: a per-scenario status word and
/// a coverage summary.
fn render_verify(result: &SddVerifyResult) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "verify — {}/{} scenario(s) verified{}",
        result.verified,
        result.total,
        if result.complete { " (complete)" } else { "" }
    );
    for s in &result.scenarios {
        let _ = write!(out, "\n  [{}] {}/{}", s.status, s.requirement, s.scenario);
        if let Some(note) = &s.note {
            let _ = write!(out, " — {note}");
        }
    }
    out
}

/// Human rendering of an archive report: the folded capabilities, where the
/// change was preserved, any exceptions, and whether the projection refreshed.
fn render_archive(result: &SddArchiveResult) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "archived — folded {} capabilit(y/ies) into the living truth",
        result.capabilities.len()
    );
    for c in &result.capabilities {
        let _ = write!(out, "\n  + {c}");
    }
    let _ = write!(out, "\n  preserved at {}", result.archived_to);
    if result.projection_regenerated {
        let _ = write!(out, "\n  projection regenerated");
    }
    for e in &result.excepted {
        let _ = write!(out, "\n  excepted: {e}");
    }
    out
}

/// Human rendering of a per-task commit: preview vs applied, the guaranteed
/// message, changed files, and any scope deviations (never hidden).
fn render_commit(result: &CommitTaskResult) -> String {
    use std::fmt::Write;
    let mut out = if result.committed {
        match &result.sha {
            Some(sha) => format!("committed {}", &sha[..sha.len().min(12)]),
            None => "committed".to_string(),
        }
    } else {
        "preview (nothing committed) — re-run with a trailing `confirm` to apply".to_string()
    };
    let _ = write!(out, "\n--- message ---\n{}", result.message.trim_end());
    let _ = write!(
        out,
        "\n--- {} file(s) changed ---",
        result.changed_files.len()
    );
    for f in &result.changed_files {
        let _ = write!(out, "\n  {f}");
    }
    if !result.deviations.is_empty() {
        let _ = write!(
            out,
            "\n! {} path(s) outside the declared scope (corrective step available):",
            result.deviations.len()
        );
        for d in &result.deviations {
            let _ = write!(out, "\n  ! {d}");
        }
    }
    out
}

/// Human rendering of the checkpoint list: change/task/agent, short ref, moment,
/// and a count of irreversible out-of-tree operations when present.
fn render_checkpoints(result: &CheckpointListResult) -> String {
    use std::fmt::Write;
    let mut out = format!("{} checkpoint(s)", result.checkpoints.len());
    for c in &result.checkpoints {
        let short = c.git_ref.rsplit('/').next().unwrap_or(&c.git_ref);
        let _ = write!(
            out,
            "\n  {}/{}-{}  {short}  {}",
            c.change, c.task, c.agent, c.created_at
        );
        if !c.irreversible.is_empty() {
            let _ = write!(out, "  ({} irreversible)", c.irreversible.len());
        }
    }
    out
}

/// Human rendering of a reversion result with its honest scope: complete vs
/// partial, and the out-of-tree operations that remain in effect.
fn render_revert(result: &CheckpointRevertResult) -> String {
    use std::fmt::Write;
    let mut out = if result.scope.complete {
        "reverted — worktree restored completely".to_string()
    } else {
        "reverted — worktree restored, but NOT a total reversion".to_string()
    };
    for op in &result.scope.irreversible {
        let _ = write!(out, "\n  irreversible: {op}");
    }
    out
}

/// Human rendering of a race: per lane, who ran it and how it ended, then the
/// full diff against that lane's base, ready for an assisted merge decision.
/// Anything the daemon did not report is a dash — the scriptable surface shows
/// the same truth the boards do, and invents no more of it than they do
/// (tablero-de-carrera design D1).
fn render_race(result: &WorktreeDiffResult) -> String {
    use std::fmt::Write;
    let short = |rev: &str| rev.chars().take(12).collect::<String>();
    let mut out = format!(
        "{} competitor(s) against base {}",
        result.competitors.len(),
        short(&result.base_rev)
    );
    for c in &result.competitors {
        let _ = write!(
            out,
            "\n\n=== {} ({} file(s) changed) — {}",
            c.agent,
            c.changed_files.len(),
            c.path
        );

        // Who ran the lane. A lane nobody dispatched says so once, rather than
        // printing four dashes that read like a broken record.
        match c.session_id.as_deref() {
            None => {
                let _ = write!(out, "\n  ran by: — (no dispatch on record)");
            }
            Some(session) => {
                let source = c
                    .source
                    .and_then(|s| serde_json::to_value(s).ok())
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| "—".to_string());
                let level = c
                    .level
                    .map(|l| format!("L{l}"))
                    .unwrap_or_else(|| "—".to_string());
                let _ = write!(
                    out,
                    "\n  ran by: source={source} profile={} {level}  session {session}",
                    c.profile.as_deref().unwrap_or("—")
                );
            }
        }

        // What the lane holds now. `committed` unknown is a dash too: the
        // daemon states it, so a missing one is a fact about the answer.
        let state = match (c.committed, c.sha.as_deref()) {
            (Some(true), Some(sha)) => format!("committed {}", short(sha)),
            (Some(true), None) => "committed".to_string(),
            (Some(false), _) => "not committed".to_string(),
            (None, _) => "—".to_string(),
        };
        let _ = write!(
            out,
            "\n  state: {state}  base {}",
            c.base_rev.as_deref().map(short).unwrap_or("—".to_string())
        );

        let _ = write!(
            out,
            "\n{}",
            if c.diff.is_empty() {
                "(no changes)"
            } else {
                c.diff.trim_end()
            }
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::{FleetResolutionSource, WorktreeCompetitorDiff};

    /// Strips every ANSI escape, so a painted rendering can be compared with
    /// the plain one letter for letter.
    fn undress(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                // Every sequence this surface emits is a CSI ending in a
                // letter; skip to it.
                for inner in chars.by_ref() {
                    if inner.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    fn change(name: &str, done: u32, total: u32, gate: bool) -> meltemi_proto::ChangeInfo {
        meltemi_proto::ChangeInfo {
            name: name.into(),
            archived: false,
            archived_at: None,
            artifacts: meltemi_proto::ChangeArtifacts {
                proposal: true,
                design: true,
                specs: true,
                tasks: true,
            },
            tasks_done: done,
            tasks_total: total,
            review_decided: 0,
            review_total: 2,
            verified: done,
            verify_total: total,
            gate_pending: gate,
            gate_artifact: gate.then(|| "tasks".to_string()),
        }
    }

    // Scenario: Sin color no se pierde información
    #[test]
    fn stripping_the_colour_removes_no_information() {
        let result = ChangeListResult {
            changes: vec![
                change("uno", 4, 4, false),
                change("dos-con-nombre-largo", 1, 9, true),
            ],
        };
        let painted = render_changes(&result, true);
        let plain = render_changes(&result, false);

        assert_ne!(painted, plain, "the painted rendering does carry escapes");
        assert_eq!(
            undress(&painted),
            plain,
            "colour is decoration: removing it leaves the same text"
        );

        // And each distinction the colour marks is marked by something else.
        assert!(plain.contains("active"), "state is a word");
        assert!(
            plain.contains("4/4") && plain.contains("1/9"),
            "progress is figures"
        );
        assert!(
            plain.contains("awaits you"),
            "a waiting gate says so in words"
        );
        assert!(
            plain.contains("PDST"),
            "artifacts are letters, absence a dot"
        );
    }

    // Scenario: El listado abre con su resumen
    #[test]
    fn the_change_listing_opens_with_totals_including_who_waits() {
        let result = ChangeListResult {
            changes: vec![change("uno", 4, 4, false), change("dos", 1, 9, true)],
        };
        let head = render_changes(&result, false);
        let summary = head.lines().nth(1).expect("a summary line");
        assert!(summary.contains("2 total"), "totals: {summary}");
        assert!(summary.contains("2 active"), "actives: {summary}");
        // The figure the old listing never gave without reading every row.
        assert!(
            summary.contains("1 awaiting your decision"),
            "how many wait on a human: {summary}"
        );

        // With nothing pending it says so rather than printing a zero.
        let calm = ChangeListResult {
            changes: vec![change("uno", 4, 4, false)],
        };
        assert!(render_changes(&calm, false).contains("none awaiting you"));
    }

    // Scenario: Las columnas se alinean con el contenido
    #[test]
    fn the_columns_align_to_the_widest_value_not_to_a_fixed_number() {
        let result = ChangeListResult {
            changes: vec![
                change("corto", 4, 4, false),
                change("otro", 111, 111, false),
            ],
        };
        let plain = render_changes(&result, false);
        let rows: Vec<&str> = plain
            .lines()
            .filter(|l| l.starts_with("  active"))
            .collect();
        assert_eq!(rows.len(), 2);
        // The name is the last column and starts at the same offset in both,
        // which is what makes the list readable vertically.
        let at = |row: &str, name: &str| row.find(name).expect("the name is on its row");
        assert_eq!(at(rows[0], "corto"), at(rows[1], "otro"));
        // And no row is padded past its last column.
        for row in rows {
            assert_eq!(row, row.trim_end(), "no trailing padding: {row:?}");
        }
    }

    fn lane(agent: &str) -> WorktreeCompetitorDiff {
        WorktreeCompetitorDiff {
            agent: agent.into(),
            path: format!("/repo/.meltemi/worktrees/dark-mode/1-1-{agent}"),
            changed_files: vec!["src/a.rs".into()],
            diff: "diff --git a/src/a.rs b/src/a.rs\n".into(),
            source: None,
            profile: None,
            level: None,
            session_id: None,
            committed: None,
            sha: None,
            base_rev: None,
        }
    }

    // Scenario: La calle declara procedencia, sesión y estado
    #[test]
    fn the_scriptable_race_shows_the_provenance_and_marks_what_it_lacks() {
        let dispatched = WorktreeCompetitorDiff {
            source: Some(FleetResolutionSource::Profile),
            profile: Some("work".into()),
            level: Some(2),
            session_id: Some("sess-1".into()),
            committed: Some(true),
            sha: Some("b".repeat(40)),
            base_rev: Some("a".repeat(40)),
            ..lane("claude")
        };
        let untouched = WorktreeCompetitorDiff {
            committed: Some(false),
            base_rev: Some("a".repeat(40)),
            diff: String::new(),
            changed_files: Vec::new(),
            ..lane("ghost")
        };
        let rendered = render_race(&WorktreeDiffResult {
            base_rev: "a".repeat(40),
            competitors: vec![dispatched, untouched],
        });

        assert!(
            rendered.contains("source=profile profile=work L2  session sess-1"),
            "the dispatched lane names who ran it and where: {rendered}"
        );
        assert!(
            rendered.contains("state: committed bbbbbbbbbbbb  base aaaaaaaaaaaa"),
            "and how it ended, shortened like every other rev: {rendered}"
        );
        // The lane nobody dispatched says that once, in words — not as four
        // dashes that read like a truncated record.
        assert!(
            rendered.contains("ran by: — (no dispatch on record)"),
            "absence is stated, never invented: {rendered}"
        );
        assert!(
            rendered.contains("state: not committed"),
            "what IS known about it still shows: {rendered}"
        );
        assert!(
            !rendered.contains("profile=ghost"),
            "no lane borrows a provenance it does not have: {rendered}"
        );
        // The diff itself is still the payload of the verb.
        assert!(rendered.contains("diff --git a/src/a.rs"));
        assert!(rendered.contains("(no changes)"));
    }

    #[test]
    fn a_lane_the_daemon_said_nothing_about_shows_dashes_rather_than_zeroes() {
        // A pre-change daemon (or any client of the previous shape) sends no
        // additive field at all. The renderer must not turn that into "not
        // committed at base 000000" — silence is silence.
        let rendered = render_race(&WorktreeDiffResult {
            base_rev: "a".repeat(40),
            competitors: vec![lane("claude")],
        });
        assert!(rendered.contains("state: —  base —"), "{rendered}");
    }
}
