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
use crate::output::{CliError, Outcome};

/// Executes an RPC-backed subcommand against the daemon at `endpoint`.
pub async fn execute(command: Command, endpoint: &str) -> Result<Outcome, CliError> {
    match command {
        Command::Status => status(endpoint).await,
        Command::Propose { idea, project_root } => propose(idea, project_root, endpoint).await,
        Command::Fleet => fleet(endpoint).await,
        Command::Project { project_root } => project(project_root, endpoint).await,
        Command::Sessions { project_root } => sessions(project_root, endpoint).await,
        Command::Explore { topic } => sdd_verb(endpoint, methods::SDD_EXPLORE, topic, None).await,
        Command::Plan { change } => {
            sdd_verb(endpoint, methods::SDD_PLAN, String::new(), Some(change)).await
        }
        Command::Constitution { topic } => {
            sdd_verb(endpoint, methods::SDD_CONSTITUTION, topic, None).await
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
        Command::Changes => changes(endpoint).await,
        Command::Show { change } => show(change, endpoint).await,
        Command::Specs { capability } => specs(capability, endpoint).await,
        Command::Validate { change } => validate(change, endpoint).await,
        Command::Direct {
            session,
            instruction,
            project_root,
        } => direct(session, instruction, project_root, endpoint).await,
        // `tunnel` is a local formatter: it never touches the daemon.
        Command::Tunnel { target, exec } => tunnel(target, exec),
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
        .request(methods::PROPOSE, &ProposeParams { idea, project_root })
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
) -> Result<Outcome, CliError> {
    let project_root = std::env::current_dir()
        .map_err(CliError::internal)?
        .display()
        .to_string();
    let params = match &change {
        Some(change) => json!({ "projectRoot": project_root, "changeName": change }),
        None => json!({ "projectRoot": project_root, "topic": topic }),
    };

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

async fn changes(endpoint: &str) -> Result<Outcome, CliError> {
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
        human: render_changes(&result),
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

async fn specs(capability: Option<String>, endpoint: &str) -> Result<Outcome, CliError> {
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
            (value.clone(), render_spec_list(&result))
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
fn render_changes(result: &ChangeListResult) -> String {
    use std::fmt::Write;
    let active = result.changes.iter().filter(|c| !c.archived).count();
    let mut out = format!(
        "{} change(s) — {active} active, {} archived",
        result.changes.len(),
        result.changes.len() - active
    );
    for c in &result.changes {
        if c.archived {
            let _ = write!(
                out,
                "\n  archived  {}  {}",
                c.archived_at.as_deref().unwrap_or("—"),
                c.name
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
            let _ = write!(
                out,
                "\n  active    {art}  tasks {}/{}  review {}/{}  verify {}/{}  {}",
                c.tasks_done,
                c.tasks_total,
                c.review_decided,
                c.review_total,
                c.verified,
                c.verify_total,
                c.name
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
fn render_spec_list(result: &SpecListResult) -> String {
    use std::fmt::Write;
    let mut out = format!(
        "{} capabilit(y/ies) in the living truth",
        result.specs.len()
    );
    for s in &result.specs {
        let _ = write!(
            out,
            "\n  {}  {} req  {} scenario(s)",
            s.capability, s.requirements, s.scenarios
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

/// Human rendering of a race: each competitor's changed-file count and the
/// full diff against the common base, ready for an assisted merge decision.
fn render_race(result: &WorktreeDiffResult) -> String {
    use std::fmt::Write;
    let short: String = result.base_rev.chars().take(12).collect();
    let mut out = format!(
        "{} competitor(s) against base {short}",
        result.competitors.len()
    );
    for c in &result.competitors {
        let _ = write!(
            out,
            "\n\n=== {} ({} file(s) changed) — {}\n{}",
            c.agent,
            c.changed_files.len(),
            c.path,
            if c.diff.is_empty() {
                "(no changes)"
            } else {
                c.diff.trim_end()
            }
        );
    }
    out
}
