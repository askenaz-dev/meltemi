// SPDX-License-Identifier: Apache-2.0

//! The identity of the user's own mesh, end to end (identidad-propia).
//!
//! These drive a real daemon over a real transport and point `[identity] command`
//! at `mock-mesh`, the simulated client. No network, no account and no real mesh
//! client, which is what the constitution demands (§5) and what the living
//! scenario «La malla del usuario no es una dependencia de Meltemi» keeps true.
//!
//! The simulated client **refuses any argv that is not `status --json`**, so the
//! linked cases below are also the assertion behind «Ningún token entra ni sale
//! del daemon»: a daemon that ever invoked `up`, `login` or `logout` would fail
//! these tests instead of quietly signing somebody in.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, error_codes, methods};
use meltemid::server::{DaemonState, serve_until_shutdown};
use meltemid::transport::{Listener, connect};

/// The simulated client, copied under a name that selects `scenario`.
///
/// A copy per scenario because Rust's tests share one process and therefore one
/// environment: two tests wanting two scenarios through a variable would race.
fn mock_mesh(dir: &Path, scenario: &str) -> PathBuf {
    let mut built = std::env::current_exe().expect("current exe");
    built.pop();
    if built.ends_with("deps") {
        built.pop();
    }
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let built = built.join(format!("mock-mesh{exe}"));
    assert!(
        built.is_file(),
        "{} is missing: run `cargo build --workspace --bins` first",
        built.display()
    );
    let copy = dir.join(format!("mock-mesh-{scenario}{exe}"));
    std::fs::copy(&built, &copy).expect("copy the simulated client");
    copy
}

fn test_endpoint(tag: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\meltemid-identidad-{}-{tag}", std::process::id())
    }
    #[cfg(unix)]
    {
        std::env::temp_dir()
            .join(format!(
                "meltemid-identidad-{}-{tag}.sock",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }
}

/// A daemon whose USER configuration is `config`, and a client talking to it.
///
/// User scope and no project root anywhere: identity is of the person and the
/// machine, and a project must not decide it (design D5).
struct Fixture {
    peer: Peer,
    shutdown: mpsc::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
    dir: PathBuf,
}

impl Fixture {
    async fn start(tag: &str, config: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("meltemi-identidad-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let config_dir = dir.join("config");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::write(config_dir.join("config.toml"), config).expect("config file");

        let endpoint = test_endpoint(tag);
        let listener = Listener::bind(&endpoint).await.expect("bind");
        let (shutdown, shutdown_rx) = mpsc::channel(1);
        let state = DaemonState::new(dir.join("data"), config_dir, shutdown.clone());
        let handle = tokio::spawn(serve_until_shutdown(listener, state, shutdown_rx));

        let stream = connect(&endpoint).await.expect("connect");
        let (peer, _incoming) = Peer::start(stream);
        peer.request(
            methods::INITIALIZE,
            &InitializeParams {
                protocol_version: PROTOCOL_VERSION,
                client: PeerInfo {
                    name: "e2e-identidad".into(),
                    version: "0".into(),
                },
            },
        )
        .await
        .expect("initialize");
        Self {
            peer,
            shutdown,
            handle,
            dir,
        }
    }

    async fn status(&self) -> Value {
        self.peer
            .request(methods::IDENTITY_STATUS, &json!({}))
            .await
            .expect("identity/status answers, refusal or not")
    }

    async fn stop(self) {
        let _ = self.shutdown.send(()).await;
        let _ = tokio::time::timeout(Duration::from_secs(5), self.handle).await;
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A user configuration naming `client` as the mesh client.
fn config_for(client: &Path) -> String {
    // The path is TOML-escaped by hand: a Windows path has backslashes, and a
    // raw one would be read as escapes.
    format!(
        "[identity]\nmesh = \"tailscale\"\nlogin-server = \"https://headscale.example\"\ncommand = {}\n",
        json!(client.to_string_lossy())
    )
}

// Scenario: Quién soy, según la malla
// Scenario: Las máquinas son los pares de la malla
// Scenario: Ningún token entra ni sale del daemon
#[tokio::test]
async fn a_linked_mesh_answers_who_i_am_and_which_machines_are_mine() {
    let dir = std::env::temp_dir().join(format!("meltemi-mesh-linked-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let client = mock_mesh(&dir, "linked");
    let fixture = Fixture::start("linked", &config_for(&client)).await;

    let answer = fixture.status().await;
    assert_eq!(answer["available"], json!(true), "{answer:#}");
    assert_eq!(answer["user"], json!("guillmar@mock.example"));
    // The configured login server answers "whose mesh is this".
    assert_eq!(answer["provider"], json!("https://headscale.example"));
    assert_eq!(answer["this"], json!("mi-pc"));

    let machines = answer["machines"].as_array().expect("machines");
    let names: Vec<&str> = machines
        .iter()
        .map(|m| m["name"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        names,
        vec!["mi-pc", "el-portatil", "la-torre"],
        "three machines of mine, this one first: {answer:#}"
    );
    // A machine somebody else shared in is not one of mine, and the client's own
    // `status` hides those too.
    assert!(!names.contains(&"equipo-de-otra-persona"), "{answer:#}");
    assert_eq!(machines[0]["thisMachine"], json!(true));
    assert_eq!(machines[2]["presence"], json!("offline"));
    assert_eq!(machines[2]["lastSeen"], json!("2026-09-30T18:21:05Z"));
    // The one that is here carries no last-seen, because the client ships the
    // zero time for it rather than omitting the field.
    assert!(machines[0]["lastSeen"].is_null(), "{answer:#}");

    // Nothing outside the model survives: the client's output contains public
    // keys and addresses, and none of them are here.
    let whole = answer.to_string();
    for leaked in ["nodekey", "100.64.0", "TailscaleIPs", "PublicKey"] {
        assert!(!whole.contains(leaked), "`{leaked}` leaked: {whole}");
    }

    fixture.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
}

// Scenario: Cliente presente, equipo sin enlazar
#[tokio::test]
async fn a_client_that_is_there_without_a_session_says_so_and_offers_the_gesture() {
    let dir = std::env::temp_dir().join(format!("meltemi-mesh-unlinked-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let client = mock_mesh(&dir, "unlinked");
    let fixture = Fixture::start("unlinked", &config_for(&client)).await;

    let answer = fixture.status().await;
    assert_eq!(answer["available"], json!(false), "{answer:#}");
    assert_eq!(
        answer["reason"],
        json!("not_linked"),
        "distinct from a client that is not there: {answer:#}"
    );
    let remedy = answer["remedy"].as_str().expect("a remedy");
    assert!(remedy.contains("linking gesture"), "{remedy}");

    // And the gesture is there to be run, composed for this machine.
    let gesture = fixture
        .peer
        .request(methods::IDENTITY_LINK, &json!({}))
        .await
        .expect("identity/link");
    assert!(
        gesture["command"]
            .as_str()
            .expect("a command")
            .contains("--login-server https://headscale.example"),
        "{gesture:#}"
    );
    assert_eq!(gesture["opensBrowser"], json!(true));

    fixture.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
}

// Scenario: Sin cliente de malla, rehúso con remedio
#[tokio::test]
async fn without_a_table_identity_is_off_and_nothing_else_is_affected() {
    let fixture = Fixture::start("off", "[agent]\n").await;

    let answer = fixture.status().await;
    assert_eq!(answer["available"], json!(false));
    assert_eq!(answer["reason"], json!("not_configured"));
    let remedy = answer["remedy"].as_str().expect("a remedy");
    assert!(remedy.contains("[identity]"), "{remedy}");

    // No other capability of the daemon is affected by identity being off.
    let status = fixture
        .peer
        .request(methods::STATUS, &json!({}))
        .await
        .expect("status still answers");
    assert!(status["daemonVersion"].is_string(), "{status:#}");

    // And linking refuses with the contract's own code, not a generic error.
    let refused = fixture
        .peer
        .request(methods::IDENTITY_LINK, &json!({}))
        .await
        .expect_err("nothing to link to");
    assert_eq!(refused.code, error_codes::IDENTITY_UNAVAILABLE);
    assert_eq!(
        refused.data.as_ref().map(|data| data["kind"].clone()),
        Some(json!("identity_unavailable")),
        "{refused:?}"
    );

    fixture.stop().await;
}

#[tokio::test]
async fn a_client_that_fails_is_reported_with_what_it_said() {
    let dir = std::env::temp_dir().join(format!("meltemi-mesh-fail-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let client = mock_mesh(&dir, "fail");
    let fixture = Fixture::start("fail", &config_for(&client)).await;

    let answer = fixture.status().await;
    assert_eq!(answer["available"], json!(false), "{answer:#}");
    assert_eq!(answer["reason"], json!("client_missing"));
    let remedy = answer["remedy"].as_str().expect("a remedy");
    assert!(
        remedy.contains("failed to connect to local tailscaled"),
        "the client's own complaint travels: {remedy}"
    );

    fixture.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn an_answer_that_is_not_the_json_we_know_is_refused() {
    let dir = std::env::temp_dir().join(format!("meltemi-mesh-garbage-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let client = mock_mesh(&dir, "garbage");
    let fixture = Fixture::start("garbage", &config_for(&client)).await;

    let answer = fixture.status().await;
    assert_eq!(answer["available"], json!(false), "{answer:#}");
    assert_eq!(answer["reason"], json!("client_missing"));
    assert!(
        answer["remedy"]
            .as_str()
            .expect("a remedy")
            .contains("[identity] command"),
        "{answer:#}"
    );

    fixture.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_client_that_hangs_runs_out_of_its_deadline() {
    // The budget is a constant somebody could delete; this is what makes it a
    // behaviour. Without it a wedged client would wedge the request and a surface
    // would spin forever.
    let dir = std::env::temp_dir().join(format!("meltemi-mesh-slow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let client = mock_mesh(&dir, "slow");
    let fixture = Fixture::start("slow", &config_for(&client)).await;

    let started = std::time::Instant::now();
    let answer = fixture.status().await;
    let took = started.elapsed();

    assert_eq!(answer["available"], json!(false), "{answer:#}");
    assert_eq!(answer["reason"], json!("client_missing"));
    assert!(
        answer["remedy"]
            .as_str()
            .expect("a remedy")
            .contains("did not answer within"),
        "{answer:#}"
    );
    assert!(
        took < meltemid::identity::BUDGET + Duration::from_secs(5),
        "the deadline bit rather than the test's patience: {took:?}"
    );

    fixture.stop().await;
    let _ = std::fs::remove_dir_all(&dir);
}

// Scenario: El gesto de vínculo se imprime, no se ejecuta
#[tokio::test]
async fn the_gesture_is_composed_for_a_client_that_could_not_possibly_be_run() {
    // The proof that nothing is launched, written so it cannot pass by accident:
    // the configured client DOES NOT EXIST. A daemon that ran it would fail; one
    // that composes text answers with the gesture naming that very path.
    let missing = std::env::temp_dir().join("meltemi-no-such-mesh-client-ever");
    assert!(
        !missing.exists(),
        "the fixture needs a path with nothing at it"
    );
    let fixture = Fixture::start("ghost", &config_for(&missing)).await;

    let gesture = fixture
        .peer
        .request(
            methods::IDENTITY_LINK,
            &json!({ "hostname": "el portatil" }),
        )
        .await
        .expect("a gesture, composed and not run");

    let command = gesture["command"].as_str().expect("a command");
    assert!(command.contains("up"), "{gesture:#}");
    assert!(command.ends_with("--hostname el portatil"), "{gesture:#}");
    // Each shell quoted its own way, and the argument vector for a surface that
    // runs it — never a re-split of the printed line.
    assert!(
        gesture["posix"]
            .as_str()
            .expect("posix")
            .ends_with("'el portatil'"),
        "{gesture:#}"
    );
    let args = gesture["args"].as_array().expect("an argv");
    assert_eq!(args.last(), Some(&json!("el portatil")), "{gesture:#}");
    assert!(args.len() >= 2, "{gesture:#}");
    // And no secret of any kind was composed into it.
    let whole = gesture.to_string().to_lowercase();
    for secret in ["authkey", "tskey-", "token", "password"] {
        assert!(!whole.contains(secret), "`{secret}` in {whole}");
    }

    // Reading status with the same configuration says the client is not there,
    // which is the honest answer and not a crash.
    let answer = fixture.status().await;
    assert_eq!(answer["reason"], json!("client_missing"), "{answer:#}");

    fixture.stop().await;
}
