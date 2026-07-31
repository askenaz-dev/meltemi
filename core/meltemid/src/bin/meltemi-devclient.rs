// SPDX-License-Identifier: Apache-2.0

//! `meltemi-devclient` (task 5.4): a disposable JSON-RPC client for driving
//! `meltemid` during development and the manual end-to-end milestone.
//!
//! It is NOT the shipping CLI — that is specified by the `cli-contrato`
//! change in Fase 1 (design D15). It starts the daemon on demand, runs one of
//! `status`, `propose` or `shutdown`, auto-approves any permission request,
//! and prints streamed session events.
//!
//! Usage:
//!   meltemi-devclient status
//!   meltemi-devclient propose "<idea>" [project-root]
//!   meltemi-devclient shutdown

use serde_json::{Value, json};

use meltemi_proto::{
    InitializeParams, PROTOCOL_VERSION, PeerInfo, PermissionOptionKind, PermissionOutcome,
    PermissionRequestParams, PermissionRequestResult, ProposeParams, methods,
};
use meltemid::bootstrap;
use meltemid::paths;
use meltemid::rpc::{Incoming, Peer, RpcError};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("status");

    let endpoint = paths::endpoint();
    let stream = bootstrap::connect_or_start(&endpoint).await?;
    let (peer, incoming) = Peer::start(stream);

    // Handle daemon -> client traffic (permission passthrough, session events)
    // concurrently with the main request/response below.
    let bg_peer = peer.clone();
    let bg = tokio::spawn(handle_incoming(bg_peer, incoming));

    // Every connection must initialize first.
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi-devclient".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .map_err(rpc_err)?;

    match command {
        "status" => {
            let result = peer
                .request(methods::STATUS, &json!({}))
                .await
                .map_err(rpc_err)?;
            println!("{result:#}");
        }
        "propose" => {
            let idea = args
                .get(1)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("usage: propose \"<idea>\" [project-root]"))?;
            let project_root = match args.get(2) {
                Some(root) => root.clone(),
                None => std::env::current_dir()?.display().to_string(),
            };
            let result = peer
                .request(
                    methods::PROPOSE,
                    &ProposeParams {
                        idea,
                        project_root,
                        agent: None,
                    },
                )
                .await
                .map_err(rpc_err)?;
            println!("{result:#}");
        }
        "shutdown" => {
            peer.request(methods::SHUTDOWN, &json!({}))
                .await
                .map_err(rpc_err)?;
            println!("shutdown requested");
        }
        other => anyhow::bail!("unknown command `{other}` (expected status|propose|shutdown)"),
    }

    peer.close();
    bg.abort();
    Ok(())
}

/// Answers daemon-initiated requests (permissions) and prints notifications
/// (session events) until the connection closes.
async fn handle_incoming(peer: Peer, mut incoming: tokio::sync::mpsc::UnboundedReceiver<Incoming>) {
    while let Some(message) = incoming.recv().await {
        match message {
            Incoming::Request { id, method, params } if method == methods::PERMISSION_REQUEST => {
                let decision = decide_permission(&params);
                let value = serde_json::to_value(decision).expect("decision serializes");
                eprintln!("permission requested -> auto-approving");
                peer.respond(id, Ok(value));
            }
            Incoming::Request { id, method, .. } => {
                peer.respond(id, Err(RpcError::method_not_found(&method)));
            }
            Incoming::Notification { method, params } if method == methods::SESSION_EVENT => {
                if let Some(kind) = params
                    .get("event")
                    .and_then(|e| e.get("type"))
                    .and_then(Value::as_str)
                {
                    eprintln!("event: {kind}");
                }
            }
            Incoming::Notification { .. } => {}
        }
    }
}

/// Auto-approves by selecting the first allow-kind option the agent offered.
fn decide_permission(params: &Value) -> PermissionRequestResult {
    let outcome = serde_json::from_value::<PermissionRequestParams>(params.clone())
        .ok()
        .and_then(|request| {
            request
                .options
                .into_iter()
                .find(|option| {
                    matches!(
                        option.kind,
                        PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                    )
                })
                .map(|option| PermissionOutcome::Selected {
                    option_id: option.option_id,
                })
        })
        .unwrap_or(PermissionOutcome::Cancelled);
    PermissionRequestResult { outcome }
}

fn rpc_err(e: RpcError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}
