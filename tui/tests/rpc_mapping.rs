// SPDX-License-Identifier: Apache-2.0

//! End-to-end mapping tests (task 5.3): the CLI's command layer drives an
//! ephemeral in-process daemon over the real transport. These two tests do not
//! mutate process-global environment, so they are safe to run concurrently.
//! The environment-mutating cases live in their own test binaries
//! (`propose_mapping`, `unreachable`).

mod common;

use std::time::Duration;

use meltemi::cli::Command;
use meltemi::run::execute;

use common::spawn_daemon;

#[tokio::test]
async fn status_maps_to_the_status_method() {
    // Scenario: status consulta el estado del daemon.
    // Scenario: Éxito termina en cero
    let (endpoint, handle) = spawn_daemon("status").await;

    let outcome = execute(Command::Status, &endpoint)
        .await
        .expect("status succeeds");
    assert!(
        outcome.json["daemonVersion"].as_str().is_some(),
        "status must report the daemon version, got: {}",
        outcome.json
    );
    assert!(outcome.json["sessions"].is_array());
    assert!(outcome.human.contains("daemon "));

    handle.abort();
}

#[tokio::test]
async fn a_daemon_application_error_maps_to_the_contract_exit_code() {
    // Scenario: Respuesta de error del contrato
    // A daemon application error (here: showing a change that does not exist)
    // maps to exit code 11.
    let (endpoint, handle) = spawn_daemon("contract").await;

    let error = execute(
        Command::Show {
            change: "no-such-change-exists".into(),
        },
        &endpoint,
    )
    .await
    .expect_err("an unknown change is a contract error");
    assert_eq!(
        error.exit,
        meltemi::exit::ExitCode::Contract,
        "a JSON-RPC application error exits with the contract code"
    );

    handle.abort();
}

#[tokio::test]
async fn fleet_maps_to_the_fleet_list_method() {
    // Scenario: fleet consulta el catálogo / Listado humano / para máquinas.
    let (endpoint, handle) = spawn_daemon("fleet").await;

    let outcome = execute(Command::Fleet, &endpoint)
        .await
        .expect("fleet succeeds");

    // The embedded registry answers: a version and well-formed agents. No
    // assertion depends on what is actually installed on this machine.
    assert!(
        outcome.json["registryVersion"].as_str().is_some(),
        "fleet must report the registry version, got: {}",
        outcome.json
    );
    let agents = outcome.json["agents"].as_array().expect("agents array");
    assert!(!agents.is_empty(), "the embedded registry lists agents");
    for agent in agents {
        assert!(agent["id"].as_str().is_some());
        assert!(agent["detected"].is_boolean());
        let level = agent["integrationLevel"].as_i64().expect("level");
        assert!((1..=4).contains(&level), "declared level in range");
    }
    assert!(outcome.human.contains("registry"), "{}", outcome.human);
    assert!(outcome.human.contains("L1"));

    // Under --json the outcome renders as exactly one JSON object on stdout.
    let mut out = Vec::new();
    meltemi::output::render_outcome(&outcome, true, &mut out).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!(printed.trim().lines().count(), 1, "one line, one object");
    let parsed: serde_json::Value = serde_json::from_str(printed.trim()).expect("valid JSON");
    assert_eq!(parsed["registryVersion"], outcome.json["registryVersion"]);

    handle.abort();
}

#[tokio::test]
async fn project_maps_to_context_project_and_emits_one_json_object() {
    // Scenario: CLI project (--json un objeto). Uses a temporary fixture repo
    // as the project root, never this repo.
    let (endpoint, handle) = spawn_daemon("project").await;

    let fixture = std::env::temp_dir().join(format!("meltemi-cli-ctx-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(fixture.join(".meltemi").join("rumbo")).unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("constitution.md"),
        "# C\n\nreglas\n",
    )
    .unwrap();
    std::fs::write(
        fixture.join(".meltemi").join("rumbo").join("product.md"),
        "---\ninclusion: siempre\n---\nRUMBO\n",
    )
    .unwrap();

    let outcome = execute(
        Command::Project {
            project_root: Some(fixture.display().to_string()),
        },
        &endpoint,
    )
    .await
    .expect("project succeeds");

    let targets = outcome.json["targets"].as_array().expect("targets array");
    assert!(!targets.is_empty(), "targets reported: {}", outcome.json);
    assert!(outcome.human.contains("projected context"));

    // Under --json the outcome is exactly one JSON object on stdout.
    let mut out = Vec::new();
    meltemi::output::render_outcome(&outcome, true, &mut out).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!(printed.trim().lines().count(), 1, "one line, one object");
    let parsed: serde_json::Value = serde_json::from_str(printed.trim()).expect("valid JSON");
    assert!(parsed["targets"].is_array());

    handle.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}

#[tokio::test]
async fn sessions_maps_to_session_list_and_emits_one_json_object() {
    // Scenario: CLI sessions (--json un objeto). An empty project lists no
    // sessions, which is enough to exercise the mapping and output discipline.
    let (endpoint, handle) = spawn_daemon("sessions").await;

    let fixture = std::env::temp_dir().join(format!("meltemi-cli-sess-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(&fixture).unwrap();

    let outcome = execute(
        Command::Sessions {
            project_root: Some(fixture.display().to_string()),
        },
        &endpoint,
    )
    .await
    .expect("sessions succeeds");

    assert!(outcome.json["sessions"].is_array(), "{}", outcome.json);
    assert!(outcome.human.contains("session(s)"));

    let mut out = Vec::new();
    meltemi::output::render_outcome(&outcome, true, &mut out).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!(printed.trim().lines().count(), 1, "one line, one object");
    let parsed: serde_json::Value = serde_json::from_str(printed.trim()).expect("valid JSON");
    assert!(parsed["sessions"].is_array());

    handle.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}

#[tokio::test]
async fn stop_maps_to_shutdown_and_stops_the_daemon() {
    // Scenario: stop -> shutdown; the daemon then stops.
    let (endpoint, handle) = spawn_daemon("stop").await;

    let outcome = execute(Command::Stop, &endpoint)
        .await
        .expect("stop succeeds");
    assert_eq!(outcome.json["shutdown"], "requested");

    // The serve loop must return after the shutdown request.
    let stopped = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(stopped.is_ok(), "daemon must stop after `stop`");
}

#[tokio::test]
async fn usage_maps_to_analytics_usage_and_emits_one_json_object() {
    // Scenario: CLI de contabilidad con salida de un objeto
    let (endpoint, handle) = spawn_daemon("usage").await;

    // A project with no records: the honest empty answer, disclosure included.
    let fixture = std::env::temp_dir().join(format!("meltemi-cli-usage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    std::fs::create_dir_all(&fixture).unwrap();

    let outcome = execute(
        Command::Usage {
            project_root: Some(fixture.display().to_string()),
            granularity: Some("month".into()),
            since: None,
            until: None,
        },
        &endpoint,
    )
    .await
    .expect("usage succeeds");

    assert!(outcome.json["cells"].is_array(), "got: {}", outcome.json);
    assert_eq!(outcome.json["cells"].as_array().unwrap().len(), 0);
    assert!(
        outcome.json["totals"]["tokens"].is_null(),
        "no measurement means no token figure at all, not a zero"
    );
    let disclosure = outcome.json["disclosure"]
        .as_array()
        .expect("disclosure array");
    assert!(
        disclosure
            .iter()
            .any(|k| k == "nothing-leaves-this-machine"),
        "the disclosure travels with the numbers: {disclosure:?}"
    );
    // The human rendering states the absence and shows the disclosure.
    assert!(outcome.human.contains("no records"), "{}", outcome.human);
    assert!(outcome.human.contains("nothing is estimated"));

    // Under --json the outcome renders as exactly one JSON object on stdout.
    let mut out = Vec::new();
    meltemi::output::render_outcome(&outcome, true, &mut out).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!(printed.trim().lines().count(), 1, "one line, one object");
    let parsed: serde_json::Value = serde_json::from_str(printed.trim()).expect("valid JSON");
    assert!(parsed["disclosure"].is_array());

    let _ = std::fs::remove_dir_all(&fixture);
    handle.abort();
}

#[tokio::test]
async fn an_invalid_usage_query_exits_with_the_contract_code() {
    // Scenario: Parámetro inválido rehúsa (through the scriptable surface).
    let (endpoint, handle) = spawn_daemon("usage-invalid").await;

    let error = execute(
        Command::Usage {
            project_root: None,
            granularity: None,
            since: Some("2026-07-10T00:00:00Z".into()),
            until: Some("2026-07-01T00:00:00Z".into()),
        },
        &endpoint,
    )
    .await
    .expect_err("an inverted range is refused");
    assert_eq!(error.exit, meltemi::exit::ExitCode::Contract);

    handle.abort();
}
