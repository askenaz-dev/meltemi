// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of a rule by command governing Claude Code's requests
//! (adaptadores-con-rawinput task 3.1), with every layer of the real chain in
//! place except the provider:
//!
//! ```text
//! test client ─JSON-RPC→ meltemid ─ACP→ meltemi-claude-acp ─events→ mock-claude-wire
//! ```
//!
//! Until this change Claude's requests carried their command only as text, so
//! a rule by command prefix could never decide one and every request reached a
//! human. Here a project rule allows `echo`: the request for `echo hola` is
//! decided by it, and the request for `echo hola && rm -rf ~` — the same
//! prefix with another command chained after it — still reaches the human.
//!
//! Its own test binary, because the scripted wire is chosen by an environment
//! variable and the tests of one binary share their environment.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::{Incoming, Peer};
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// One turn that asks about two commands with the same prefix, one through
/// each permission channel the adapter installs.
const TWO_COMMANDS: &str = r#"
{"mock":"await-input"}
{"mock":"once"}
{"type":"system","subtype":"init","cwd":"/mock/project","session_id":"mock-claude-session","tools":["Read","Write","Bash"],"mcp_servers":[],"model":"mock-sonnet","permissionMode":"default","slash_commands":[],"apiKeySource":"none","claude_code_version":"2.0.0-mock"}
{"mock":"ask-hook","tool":"Bash","toolUseId":"toolu_ruled","input":{"command":"echo hola"}}
{"mock":"ask-prompt-tool","tool":"Bash","toolUseId":"toolu_chained","input":{"command":"echo hola && rm -rf ~"}}
{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"mock-claude-session","result":"Done.","usage":{"input_tokens":12,"output_tokens":3}}
"#;

const RULES: &str = "[[rule]]\neffect = \"allow\"\ntool = \"execute\"\ncommand-prefix = \"echo\"\n";

fn workspace_bin(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let path = dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    });
    assert!(
        path.exists(),
        "{name} was not built at {}; run `cargo test` at the workspace root",
        path.display()
    );
    path
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-e2e-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!("meltemid-e2e-{}-{tag}.sock", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }
}

#[tokio::test]
async fn a_rule_by_command_decides_claudes_request_and_not_what_is_chained_after_it() {
    // Scenarios: La petición lleva la entrada y la ruta; Lo encadenado tras el
    // prefijo escala al humano
    let adapter = workspace_bin("meltemi-claude-acp");
    let root = std::env::temp_dir().join(format!("meltemi-e2e-rule-cmd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).unwrap();
    let script = root.join("two-commands.ndjson");
    std::fs::write(&script, TWO_COMMANDS.trim_start()).unwrap();
    // SAFETY: the only test in this binary, set before anything is launched.
    unsafe {
        std::env::set_var("MELTEMI_CLAUDE_BIN", workspace_bin("mock-claude-wire"));
        std::env::set_var("MELTEMI_MOCK_CLAUDE_SCRIPT", &script);
        std::env::remove_var("MELTEMI_AGENT_COMMAND");
    }
    std::fs::write(
        root.join(".meltemi").join("config.toml"),
        format!(
            "[agent]\ncommand = ['{}']\n",
            adapter.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    std::fs::write(root.join(".meltemi").join("permissions.toml"), RULES).unwrap();
    let root_str = root.display().to_string();

    let endpoint = test_endpoint("rule-cmd");
    let listener = Listener::bind(&endpoint).await.expect("bind");
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let state = DaemonState::for_test("rule-cmd", shutdown_tx);
    let daemon = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

    let stream = connect(&endpoint).await.expect("connect");
    let (peer, incoming) = Peer::start(stream);
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "e2e-rule-cmd".into(),
                version: "0".into(),
            },
        },
    )
    .await
    .expect("initialize");

    // The human: records what reached them, and refuses it.
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let human = tokio::spawn({
        let peer = peer.clone();
        let asked = asked.clone();
        async move {
            let mut incoming = incoming;
            while let Some(message) = incoming.recv().await {
                if let Incoming::Request { id, method, params } = message
                    && method == methods::PERMISSION_REQUEST
                {
                    asked.lock().unwrap().push(params["toolCall"].clone());
                    let reject = params["options"]
                        .as_array()
                        .expect("options")
                        .iter()
                        .find(|option| option["kind"] == "reject_once")
                        .expect("a reject option")["optionId"]
                        .clone();
                    peer.respond(
                        id,
                        Ok(json!({ "outcome": { "outcome": "selected", "optionId": reject } })),
                    );
                }
            }
        }
    });

    let result = tokio::time::timeout(
        Duration::from_secs(120),
        peer.request(
            methods::PROPOSE,
            &json!({ "idea": "two commands, one rule", "projectRoot": root_str }),
        ),
    )
    .await
    .expect("propose returned")
    .expect("propose ok");
    assert_eq!(result["status"], "completed", "{result:#}");

    let asked = asked.lock().unwrap().clone();
    assert_eq!(
        asked.len(),
        1,
        "only the chained command reached the human: {asked:#?}"
    );
    assert_eq!(asked[0]["toolCallId"], "toolu_chained");
    assert_eq!(
        asked[0]["rawInput"]["command"], "echo hola && rm -rf ~",
        "and the human saw the command the rule read: {:#}",
        asked[0]
    );

    // The log says who decided each: the rule for one, the human for the other.
    let list = peer
        .request(methods::SESSION_LIST, &json!({ "projectRoot": root_str }))
        .await
        .expect("session/list ok");
    let id = list["sessions"][0]["sessionId"]
        .as_str()
        .expect("a session id")
        .to_string();
    let log = peer
        .request(
            methods::SESSION_LOG,
            &json!({ "projectRoot": root_str, "sessionId": id, "limit": 1000 }),
        )
        .await
        .expect("session/log ok");
    let decided: Vec<Value> = log["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.as_str()?).ok())
        .filter(|event| event["type"] == "permission_decided")
        .map(|event| event["payload"].clone())
        .collect();
    let by_rule: Vec<&Value> = decided
        .iter()
        .filter(|payload| payload["decidedBy"] == "rule")
        .collect();
    assert_eq!(
        by_rule.len(),
        1,
        "one request decided by the rule: {decided:#?}"
    );
    assert_eq!(by_rule[0]["denied"], false);
    assert_eq!(by_rule[0]["rule"]["commandPrefix"], "echo");
    assert!(
        decided
            .iter()
            .any(|payload| payload["decidedBy"] == "client" && payload["denied"] == true),
        "and the chained one by the human, who refused it: {decided:#?}"
    );

    human.abort();
    peer.close();
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}
