// SPDX-License-Identifier: Apache-2.0

//! The scripted headless-session wire behaves as the adapters will need it to
//! (adaptadores-propios-acp task 1.3).
//!
//! These are fixture tests: they pilot the fixture the way the adapter will,
//! over a real process and real pipes, so that a broken fixture fails here
//! instead of in the middle of an adapter's test, where it would read as an
//! adapter bug.

use std::io::Write;
use std::process::{Command, Stdio};

/// Runs the wire with the given arguments, sends the input, and returns
/// `(exit code, stdout, stderr)`.
fn run(args: &[&str], input: &str) -> (Option<i32>, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mock-claude-wire"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the scripted wire is built next to this test");
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        stdin.write_all(input.as_bytes()).expect("write input");
        // Closing the input is how a real adapter ends the session.
    }
    let done = child.wait_with_output().expect("the wire ends on its own");
    (
        done.status.code(),
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

/// The JSON values of every emitted line.
fn events(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str(l)
                .unwrap_or_else(|e| panic!("emitted line is not JSON ({e}): {l}"))
        })
        .collect()
}

#[test]
fn the_signed_in_wire_plays_a_whole_turn_for_one_user_message() {
    let (code, stdout, stderr) = run(
        &[],
        "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"do it\"}}\n",
    );
    assert_eq!(code, Some(0), "stderr: {stderr}");

    let events = events(&stdout);
    let init = &events[0];
    assert_eq!(init["type"], "system");
    assert_eq!(init["subtype"], "init");
    assert!(
        init["capabilities"].is_array(),
        "the initial event carries the feature-detection array the handshake reads"
    );
    assert_eq!(
        init["apiKeySource"], "none",
        "this wire runs under the signed-in session, with no API key anywhere"
    );

    let kinds: Vec<&str> = events.iter().filter_map(|e| e["type"].as_str()).collect();
    assert!(
        kinds.contains(&"stream_event"),
        "token-level deltas are emitted: {kinds:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e["message"]["content"][0]["type"] == "tool_use"),
        "a tool call is emitted, so the permission path has something to carry"
    );
    assert_eq!(
        events.last().expect("at least one event")["type"],
        "result",
        "the turn is closed by the final result"
    );
}

#[test]
fn each_user_message_replays_the_turn_and_the_preamble_is_emitted_once() {
    let message = "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"again\"}}\n";
    let (code, stdout, stderr) = run(&[], &format!("{message}{message}"));
    assert_eq!(code, Some(0), "stderr: {stderr}");

    let events = events(&stdout);
    let inits = events.iter().filter(|e| e["subtype"] == "init").count();
    let results = events.iter().filter(|e| e["type"] == "result").count();
    assert_eq!(inits, 1, "the handshake happens once per process");
    assert_eq!(results, 2, "each message gets its own turn");
}

#[test]
fn the_api_key_variant_announces_the_surface_that_demands_a_key() {
    let (code, stdout, stderr) = run(&["--api-key-mode"], "");
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let events = events(&stdout);
    assert_eq!(
        events[0]["apiKeySource"], "ANTHROPIC_API_KEY",
        "the fixture for the guard announces the API-key surface at the handshake"
    );
    assert_eq!(
        events[0]["capabilities"].as_array().map(Vec::len),
        Some(0),
        "and offers none of the capabilities the signed-in surface has"
    );
}

#[test]
fn the_wire_answers_what_version_it_is() {
    // The session's events carry a feature array and no number, so the version
    // a session records is the one this flag answers — and it says "mock", so
    // that a log holding it can never be mistaken for a real CLI's.
    let (code, stdout, stderr) = run(&["--version"], "");
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stdout.contains("mock"), "stdout: {stdout}");
    assert!(
        stdout.split_whitespace().next().is_some_and(|token| token
            .split(['-', '+'])
            .next()
            .is_some_and(|core| core.split('.').count() == 3)),
        "the answer starts with a version an adapter can read: {stdout}"
    );
}

#[test]
fn input_that_is_not_the_dialect_fails_loudly() {
    // A fixture that tolerated a malformed turn would let a broken adapter
    // pass its own tests.
    let (code, _stdout, stderr) = run(&[], "not json\n");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("not JSON"), "stderr: {stderr}");

    let (code, _stdout, stderr) = run(&[], "{\"type\":\"assistant\"}\n");
    assert_eq!(code, Some(2));
    assert!(stderr.contains("assistant"), "stderr: {stderr}");
}

#[test]
fn a_script_from_a_file_replaces_the_embedded_one() {
    // The fixtures are data: a later test freezes the wire it needs without a
    // new binary and without new code.
    let path =
        std::env::temp_dir().join(format!("meltemi-wire-script-{}.ndjson", std::process::id()));
    std::fs::write(
        &path,
        "# a wire that says only this\n{\"type\":\"system\",\"subtype\":\"init\",\"capabilities\":[\"custom\"]}\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(&["--script", &path.display().to_string()], "");
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let events = events(&stdout);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["capabilities"][0], "custom");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_hook_is_run_through_a_shell_with_its_command_line_intact() {
    // The adapter writes a hook command with quoted paths, because the paths a
    // real install uses have spaces in them. This is where that quoting is
    // tested rather than assumed: the command goes through the same shell a CLI
    // uses, and what it printed has to come back.
    let dir = std::env::temp_dir().join(format!("meltemi hook test {}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let settings = dir.join("settings.json");
    let command = format!("\"{}\" --version", env!("CARGO_BIN_EXE_mock-claude-wire"));
    std::fs::write(
        &settings,
        serde_json::json!({
            "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [
                {"type": "command", "command": command}
            ]}]},
        })
        .to_string(),
    )
    .unwrap();

    let handed = mock_provider::claude_permissions::Handed {
        settings: Some(settings),
        ..mock_provider::claude_permissions::Handed::default()
    };
    let decision = handed.ask_hook("Write", "toolu_1", &serde_json::json!({}));
    assert!(
        decision.reason.contains("mock"),
        "the hook ran and what it said came back: {decision:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
