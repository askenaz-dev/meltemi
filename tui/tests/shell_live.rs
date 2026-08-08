// SPDX-License-Identifier: Apache-2.0

//! End-to-end test of the shell's live wiring (task 6.3): the connection actor
//! drives an ephemeral in-process daemon over the real transport and reports a
//! live connection snapshot and the session list. Runs against a temporary
//! endpoint, never the real user daemon.

mod common;

use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use meltemi::shell::conn::{Command, connection_actor};
use meltemi::shell::live::Update;
use meltemi::shell::messages::Lang;
use meltemi::shell::render::ConnState;

use common::spawn_daemon;

#[tokio::test]
async fn connection_actor_reports_connected_and_sessions() {
    let (endpoint, daemon) = spawn_daemon("shell-live").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    let mut connected = false;
    let mut got_sessions = false;
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            match update {
                Update::Conn(ConnState::Connected { .. }) => connected = true,
                Update::Sessions(_) => got_sessions = true,
                _ => {}
            }
            if connected && got_sessions {
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("actor reported Connected + Sessions within 10s");
    assert!(connected, "the actor must report a live Connected snapshot");
    assert!(got_sessions, "the actor must report the session list");

    // Dropping the command sender ends the actor cleanly.
    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
}

// Scenario: Alta y baja de proyecto tecleando la ruta
// Scenario: Baja desde la TUI no toca el disco
// Scenario: Alta sin diálogo nativo en la superficie de terminal
#[tokio::test]
async fn a_typed_path_registers_and_forgets_without_restarting_the_shell() {
    let (endpoint, daemon) = spawn_daemon("shell-registry").await;

    let fixture = std::env::temp_dir().join(format!("meltemi-shell-reg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(&fixture).unwrap();
    let path = fixture.display().to_string();

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    cmd_tx
        .send(Command::RegisterProject(path.clone()))
        .expect("the actor is listening");
    // The registry answer arrives on its own: the view updates without the
    // shell being restarted.
    let leaf = fixture
        .file_name()
        .and_then(|name| name.to_str())
        .expect("the fixture has a name")
        .to_string();
    let listed = wait_for_projects(&mut upd_rx, |roots| {
        roots.iter().any(|root| root.ends_with(&leaf))
    })
    .await;
    assert!(!listed.is_empty(), "the registered project is listed");
    assert!(
        !fixture.join(".meltemi").exists(),
        "nothing was created inside the root"
    );

    cmd_tx
        .send(Command::ForgetProject(path.clone()))
        .expect("the actor is listening");
    let after = wait_for_projects(&mut upd_rx, |roots| roots.is_empty()).await;
    assert!(after.is_empty(), "forgetting drops it from the listing");
    // …and the directory it named is exactly where it was.
    assert!(fixture.is_dir(), "forgetting touches no disk");

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}

/// Waits for a project listing that satisfies `wanted`, returning its roots.
async fn wait_for_projects(
    updates: &mut tokio::sync::mpsc::UnboundedReceiver<Update>,
    wanted: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    let mut seen = Vec::new();
    let collect = async {
        while let Some(update) = updates.recv().await {
            if let Update::Projects(rows) = update {
                let roots: Vec<String> = rows.into_iter().map(|row| row.root).collect();
                if wanted(&roots) {
                    seen = roots;
                    return;
                }
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("the registry answered within 10s");
    seen
}

// Scenario: Sesión no dirigible lo dice con remedio
#[tokio::test]
async fn a_refused_direction_comes_back_with_the_daemons_diagnosis_and_remedy() {
    // The instruction reaches the daemon and the refusal reaches the user: no
    // session was found, and the answer says what to do about it instead of
    // dropping the text the user typed.
    let (endpoint, daemon) = spawn_daemon("shell-direct").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    cmd_tx
        .send(Command::Direct {
            session_id: "11111111-2222-3333-4444-555555555555".into(),
            project_root: std::env::temp_dir().display().to_string(),
            instruction: "Arregla el build".into(),
        })
        .expect("the actor is listening");

    let mut refusal = None;
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            if let Update::Notice(text) = update
                && text.contains("no admite")
            {
                refusal = Some(text);
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("the refusal reached the surface within 10s");
    let refusal = refusal.expect("a refusal notice");
    assert!(
        refusal.contains("meltemi sessions"),
        "the daemon's remedy travels with the diagnosis: {refusal}"
    );

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
}

/// A git fixture with one change, one task, and the mock as the agent —
/// slowed down so a dispatch is observably long rather than instantaneous.
fn race_fixture(tag: &str, turn_delay_ms: u32) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-shell-race-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi/changes/dark-mode")).unwrap();
    let mock = common::mock_agent_bin()
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(
        root.join(".meltemi/config.toml"),
        format!("[agent]\ncommand = ['{mock}', '--turn-delay-ms', '{turn_delay_ms}']\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/permissions.toml"),
        "[[rule]]\neffect = \"allow\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".meltemi/changes/dark-mode/tasks.md"),
        "## 1. Build\n\n- [ ] 1.1 Add the toggle\n",
    )
    .unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "t@t.co"],
        vec!["config", "user.name", "t"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["config", "core.autocrlf", "false"],
        vec!["add", "-A"],
        vec!["commit", "-qm", "init"],
    ] {
        let out = std::process::Command::new("git")
            .args(&args)
            .current_dir(&root)
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?}");
    }
    root
}

// Scenario: El despacho no congela el shell
#[tokio::test]
async fn a_dispatch_runs_a_whole_turn_without_stopping_the_shell() {
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let root = race_fixture("dispatch", 2500);
    let (endpoint, daemon) = spawn_daemon("shell-race").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    cmd_tx
        .send(Command::SetScope(Some(root.display().to_string())))
        .unwrap();
    // The turn holds the daemon for ~2.5s. If the actor awaited it inline, this
    // is exactly the window in which the shell would go deaf.
    cmd_tx
        .send(Command::RaceDispatch {
            change: "dark-mode".into(),
            task: "1.1".into(),
            agent: "mock".into(),
        })
        .unwrap();

    let mut kept_breathing = false;
    let mut board_after: Option<usize> = None;
    let mut notice = String::new();
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            match update {
                // Status/sessions keep arriving WHILE the turn runs: the tick
                // is what proves the loop was never blocked.
                Update::Sessions(_) | Update::Conn(_) if board_after.is_none() => {
                    kept_breathing = true;
                }
                Update::Notice(text) => notice = text,
                // The dispatch finished and the board re-read itself.
                Update::Race(board) => {
                    board_after = Some(board.lanes.len());
                    break;
                }
                _ => {}
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(60), collect)
        .await
        .expect("the dispatch answered and the board re-read within 60s");

    assert!(
        kept_breathing,
        "the shell kept receiving updates while the turn ran"
    );
    assert_eq!(
        board_after,
        Some(1),
        "the board reflects the lane the dispatch created"
    );
    assert!(
        notice.contains("turno") || notice.contains("mock"),
        "the finished turn reports back: {notice}"
    );

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: El rehúso llega con su remedio al shell
#[tokio::test]
async fn a_refused_link_reaches_the_shell_with_the_daemons_remedy() {
    // Linking an agent the catalog does not know: the refusal crosses the
    // wire with its diagnosis AND its remedy, and the shell shows both — a
    // typed name is never dropped into silence.
    let (endpoint, daemon) = spawn_daemon("shell-link").await;

    let (cmd_tx, cmd_rx) = unbounded_channel::<Command>();
    let (upd_tx, mut upd_rx) = unbounded_channel::<Update>();
    let actor = tokio::spawn(connection_actor(endpoint.clone(), Lang::Es, cmd_rx, upd_tx));

    cmd_tx
        .send(Command::LinkSubscription {
            agent: "no-such-provider".into(),
            name: "work".into(),
        })
        .expect("the actor is listening");

    let mut refusal = None;
    let collect = async {
        while let Some(update) = upd_rx.recv().await {
            if let Update::Notice(text) = update
                && text.contains("subscription/link")
            {
                refusal = Some(text);
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), collect)
        .await
        .expect("the refusal reached the surface within 10s");
    let refusal = refusal.expect("a refusal notice");
    assert!(
        refusal.contains("not in the fleet catalog"),
        "the diagnosis travels: {refusal}"
    );
    assert!(
        refusal.contains("meltemi fleet"),
        "and the remedy with it: {refusal}"
    );

    drop(cmd_tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), actor).await;
    daemon.abort();
}
