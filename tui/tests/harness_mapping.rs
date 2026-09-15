// SPDX-License-Identifier: Apache-2.0

//! End-to-end mapping of the `harness` verb. Isolated in its own test binary
//! because the verb takes no project root — it reads the working directory, the
//! same way `git` does — so exercising it means changing the process's current
//! directory. Being the only test here keeps that mutation free of concurrent
//! readers, the same reason `propose_mapping` stands alone.
//!
//! The fixture is a temporary repository, never this one.

mod common;

use meltemi::cli::Command;
use meltemi::run::execute;

use common::spawn_daemon;

/// Writes `<root>/<dirs...>/RULE.md` with the given front matter and body.
fn write_rule(root: &std::path::Path, relative: &[&str], front: &str, body: &str) {
    let mut dir = root.to_path_buf();
    for part in relative {
        dir.push(part);
    }
    std::fs::create_dir_all(&dir).expect("rule directory");
    std::fs::write(dir.join("RULE.md"), format!("---\n{front}---\n\n{body}\n")).expect("RULE.md");
}

// Scenario: El verbo muestra de qué capa viene cada pieza
#[tokio::test]
async fn the_cli_verb_shows_the_layer_each_piece_came_from_and_what_does_not_apply() {
    let fixture =
        std::env::temp_dir().join(format!("meltemi-cli-harness-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture);
    let harness = fixture.join(".meltemi").join("harness");

    // The same rule in two layers: the more specific one governs, and the one
    // it covered must still be visible — "why is mine not the one applying" is
    // the question this verb exists to answer.
    write_rule(
        &harness,
        &["rules", "no-console-log"],
        "name: no-console-log\nseverity: warn\n",
        "the project's own version",
    );
    write_rule(
        &harness,
        &["per-agent", "claude-code", "rules", "no-console-log"],
        "name: no-console-log\nseverity: error\n",
        "the version for one agent",
    );
    // Read, but unusable: the directory and the front matter name different
    // things, so it is listed with its diagnostic instead of vanishing.
    write_rule(
        &harness,
        &["rules", "no-any-cast"],
        "name: no-any-cast-v2\n",
        "a rule whose block disagrees with its directory",
    );
    // A per-agent directory the fleet catalog cannot name: reported, not read.
    write_rule(
        &harness,
        &["per-agent", "claudius", "rules", "no-todo-comments"],
        "name: no-todo-comments\n",
        "for an agent this build does not know",
    );

    let (endpoint, handle) = spawn_daemon("harness").await;

    // The verb reads the working directory. Restored before the assertions run
    // so a failure cannot leave the process pointing at a temporary directory.
    // SAFETY: this test binary runs this single test; nothing reads the
    // process's current directory concurrently.
    let previous = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(&fixture).expect("enter the fixture");
    let outcome = execute(Command::Harness { agent: None }, &endpoint, false).await;
    std::env::set_current_dir(&previous).expect("leave the fixture");
    let outcome = outcome.expect("harness succeeds");

    let human = &outcome.human;
    // Every piece with its layer of origin.
    assert!(
        human.contains("no-console-log [project_agent · claude-code]"),
        "the governing rule with its layer and agent: {human}"
    );
    // What it covered, with the layer it came from.
    assert!(
        human.contains("covered: [project]"),
        "the covered layer: {human}"
    );
    // What cannot be projected, with its reason.
    assert!(
        human.contains("no-any-cast [project] — not applied"),
        "the unusable rule: {human}"
    );
    assert!(
        human.contains("the directory is the identity resolution uses"),
        "the diagnostic that says why: {human}"
    );
    // An identifier the catalog does not know is declared, never read silently.
    assert!(
        human.contains("unknown agent `claudius`"),
        "the unknown per-agent directory: {human}"
    );

    // Output discipline: structured under `--json`, exactly one object.
    let mut out = Vec::new();
    meltemi::output::render_outcome(&outcome, meltemi::format::Format::Json, &mut out).unwrap();
    let printed = String::from_utf8(out).unwrap();
    assert_eq!(printed.trim().lines().count(), 1, "one line, one object");
    let parsed: serde_json::Value = serde_json::from_str(printed.trim()).expect("valid JSON");
    let rules = parsed["rules"].as_array().expect("rules array");
    let governing = rules
        .iter()
        .find(|r| r["name"] == "no-console-log")
        .expect("the governing rule travels");
    assert_eq!(governing["origin"]["layer"], "project_agent");
    assert_eq!(governing["origin"]["agent"], "claude-code");
    assert_eq!(governing["shadowed"][0]["layer"], "project");

    handle.abort();
    let _ = std::fs::remove_dir_all(&fixture);
}
