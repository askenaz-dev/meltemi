// SPDX-License-Identifier: Apache-2.0

//! The harness in the projection, and the line it must not cross
//! (harness-global-y-por-agente design D6).
//!
//! Runs against temporary fixture repositories, never this one.

use std::path::{Path, PathBuf};

fn fixture(tag: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("meltemi-harness-proj-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".meltemi")).expect("fixture root");
    std::fs::write(
        root.join(".meltemi").join("constitution.md"),
        "# Constitución de prueba\n\nUn principio.\n",
    )
    .expect("constitution");
    root
}

fn write_rule(scope_root: &Path, name: &str, front: &str, body: &str) {
    let dir = scope_root.join("rules").join(name);
    std::fs::create_dir_all(&dir).expect("rule dir");
    std::fs::write(
        dir.join("RULE.md"),
        format!("---\n{front}\n---\n\n# {name}\n\n{body}\n"),
    )
    .expect("rule file");
}

/// The tests below aim a write at a temporary directory by setting the agent's
/// own documented home variable — which is process-global, and this binary runs
/// its tests in parallel. Without this they take turns clearing each other's
/// variable, and the failure looks like a bug in the code under test rather
/// than in the test harness. It cost one round of exactly that to find out.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Takes the environment lock, surviving a panic in a test that held it before.
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn agents() -> Vec<String> {
    vec!["claude-code".to_string(), "codex-cli".to_string()]
}

// Scenario: Una regla del proyecto llega a los destinos del repositorio
#[test]
fn a_project_rule_reaches_the_repository_targets_with_what_it_applies_to() {
    let root = fixture("project-rule");
    write_rule(
        &root.join(".meltemi/harness"),
        "no-any-cast",
        "name: no-any-cast\nseverity: error\nscope: [\"**/*.{ts,tsx}\"]\ndescription: \"No `as any`.\"",
        "Do not commit `as any`.",
    );

    let written = meltemid::context::project_and_write(&root, &agents()).expect("projection");
    assert!(!written.is_empty(), "the projection wrote its targets");

    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md");
    assert!(
        agents_md.contains("no-any-cast"),
        "the rule reached the managed block:\n{agents_md}"
    );
    assert!(
        agents_md.contains("Do not commit `as any`."),
        "and so did its body:\n{agents_md}"
    );
    // The scope travels as prose, and the glob survives the comma inside its
    // braces — the defect the real-format fixture found.
    assert!(
        agents_md.contains("`**/*.{ts,tsx}`"),
        "the scope is stated, whole:\n{agents_md}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: Proyectar con harness global deja el repositorio intacto
#[test]
fn projecting_with_a_global_harness_present_leaves_the_repository_untouched() {
    let root = fixture("global-stays-out");
    // A user harness with something unmistakable in it, in BOTH of its layers.
    let user_harness = root.join("elsewhere/config/harness");
    write_rule(
        &user_harness,
        "personal-preference",
        "name: personal-preference",
        "PERSONAL-CONTENT-THAT-MUST-NOT-BE-COMMITTED",
    );
    write_rule(
        &user_harness.join("per-agent/claude-code"),
        "personal-per-agent",
        "name: personal-per-agent",
        "ALSO-PERSONAL-AND-PER-AGENT",
    );
    // And a project rule, so the projection genuinely runs the rules path.
    write_rule(
        &root.join(".meltemi/harness"),
        "team-rule",
        "name: team-rule",
        "SHARED-WITH-THE-TEAM",
    );

    meltemid::context::project_and_write(&root, &agents()).expect("projection");

    // Every target the projection writes, checked: the personal content is in
    // none of them, and the shared one is.
    for target in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        let path = root.join(target);
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read target");
        assert!(
            !text.contains("PERSONAL-CONTENT-THAT-MUST-NOT-BE-COMMITTED"),
            "{target} carries the user's own harness"
        );
        assert!(
            !text.contains("ALSO-PERSONAL-AND-PER-AGENT"),
            "{target} carries the user's per-agent harness"
        );
        assert!(
            !text.contains("personal-preference"),
            "{target} names a user-scope rule"
        );
    }
    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md");
    assert!(
        agents_md.contains("SHARED-WITH-THE-TEAM"),
        "the project's own rule did travel, so this test is not passing by \
         projecting nothing:\n{agents_md}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_rule_that_cannot_be_read_is_left_out_of_the_projection() {
    let root = fixture("unusable");
    write_rule(
        &root.join(".meltemi/harness"),
        "misnamed",
        "name: something-else",
        "BODY-OF-A-RULE-THAT-CONTRADICTS-ITS-DIRECTORY",
    );

    meltemid::context::project_and_write(&root, &agents()).expect("projection");
    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md");
    assert!(
        !agents_md.contains("BODY-OF-A-RULE-THAT-CONTRADICTS-ITS-DIRECTORY"),
        "a rule whose identity is in dispute is not projected:\n{agents_md}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_per_agent_rule_of_the_project_is_projected_and_an_unknown_agents_is_not() {
    let root = fixture("per-agent");
    write_rule(
        &root.join(".meltemi/harness/per-agent/claude-code"),
        "for-claude",
        "name: for-claude",
        "RULE-FOR-A-KNOWN-AGENT",
    );
    write_rule(
        &root.join(".meltemi/harness/per-agent/some-agent-9000"),
        "for-nobody",
        "name: for-nobody",
        "RULE-FOR-AN-AGENT-THE-CATALOG-DOES-NOT-KNOW",
    );

    meltemid::context::project_and_write(&root, &agents()).expect("projection");
    let agents_md = std::fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md");
    assert!(agents_md.contains("RULE-FOR-A-KNOWN-AGENT"), "{agents_md}");
    assert!(
        !agents_md.contains("RULE-FOR-AN-AGENT-THE-CATALOG-DOES-NOT-KNOW"),
        "writing for an agent the core cannot name is writing blind:\n{agents_md}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// Scenario: La primera escritura en config ajena se consiente
// Scenario: Un agente sin destino verificado no recibe escritura
#[test]
fn the_first_write_into_an_agents_own_file_waits_for_consent_and_then_records_it() {
    let _env = env_lock();
    // A user config directory with a global rule, and a fake home so nothing
    // of the real machine is touched.
    let root = fixture("consent");
    let config = root.join("config");
    let home = root.join("home");
    write_rule(
        &config.join("harness"),
        "my-preference",
        "name: my-preference",
        "MY-OWN-GLOBAL-RULE",
    );
    // CODEX_HOME is documented, which is what lets this test aim the write at
    // a temporary directory instead of the developer's own configuration.
    let codex_home = home.join("codex");
    std::fs::create_dir_all(&codex_home).expect("codex home");
    unsafe { std::env::set_var("CODEX_HOME", &codex_home) };

    let known = vec!["codex-cli".to_string()];

    // First pass: nothing consented, so nothing is written and the pending
    // destination is named.
    let first = meltemid::user_scope::project_user_scope(&config, &known).expect("first pass");
    let codex = first
        .iter()
        .find(|written| written.agent == "codex-cli")
        .expect("the verified destination is reported");
    assert_eq!(
        codex.state,
        meltemid::user_scope::UserState::AwaitingConsent
    );
    assert!(
        !codex.path.is_file(),
        "nothing was written into someone else's configuration: {}",
        codex.path.display()
    );

    // Consent, then project again.
    let mut asked = std::collections::BTreeSet::new();
    asked.insert("codex-cli".to_string());
    meltemid::user_scope::record_consent(&config, &asked).expect("record consent");
    let second = meltemid::user_scope::project_user_scope(&config, &known).expect("second pass");
    let codex = second
        .iter()
        .find(|written| written.agent == "codex-cli")
        .expect("still reported");
    assert_eq!(codex.state, meltemid::user_scope::UserState::Written);
    let text = std::fs::read_to_string(&codex.path).expect("the file now exists");
    assert!(text.contains("MY-OWN-GLOBAL-RULE"), "{text}");

    // Idempotent: a third pass changes nothing.
    let third = meltemid::user_scope::project_user_scope(&config, &known).expect("third pass");
    assert_eq!(
        third
            .iter()
            .find(|w| w.agent == "codex-cli")
            .expect("reported")
            .state,
        meltemid::user_scope::UserState::Unchanged
    );

    unsafe { std::env::remove_var("CODEX_HOME") };
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_the_agent_itself_ignores_is_reported_and_not_written() {
    let _env = env_lock();
    // Codex documents that it reads AGENTS.override.md INSTEAD of AGENTS.md
    // when the override exists. Writing anyway would fill a file nobody reads
    // and look like it worked.
    let root = fixture("overridden");
    let config = root.join("config");
    let codex_home = root.join("home/codex");
    std::fs::create_dir_all(&codex_home).expect("codex home");
    std::fs::write(codex_home.join("AGENTS.override.md"), "mine\n").expect("override");
    write_rule(
        &config.join("harness"),
        "my-preference",
        "name: my-preference",
        "MY-OWN-GLOBAL-RULE",
    );
    let mut asked = std::collections::BTreeSet::new();
    asked.insert("codex-cli".to_string());
    meltemid::user_scope::record_consent(&config, &asked).expect("consent");
    unsafe { std::env::set_var("CODEX_HOME", &codex_home) };

    let written = meltemid::user_scope::project_user_scope(&config, &["codex-cli".to_string()])
        .expect("projection");
    let codex = written
        .iter()
        .find(|w| w.agent == "codex-cli")
        .expect("reported");
    assert_eq!(
        codex.state,
        meltemid::user_scope::UserState::IgnoredByAgent {
            overridden_by: "AGENTS.override.md".to_string()
        },
        "consented is not the same as effective"
    );
    assert!(
        !codex_home.join("AGENTS.md").is_file(),
        "the file the agent would not read was left uncreated"
    );

    unsafe { std::env::remove_var("CODEX_HOME") };
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn nothing_of_a_repository_reaches_a_users_own_file() {
    let _env = env_lock();
    // The mirror of the repository guard, and structural for the same reason:
    // the user projection is handed no project root at all.
    let root = fixture("mirror");
    let config = root.join("config");
    let codex_home = root.join("home/codex");
    std::fs::create_dir_all(&codex_home).expect("codex home");
    write_rule(
        &config.join("harness"),
        "mine",
        "name: mine",
        "MY-OWN-GLOBAL-RULE",
    );
    // A project rule sitting right there, which must not travel.
    write_rule(
        &root.join(".meltemi/harness"),
        "team-rule",
        "name: team-rule",
        "TEAM-CONTENT-THAT-MUST-NOT-LEAVE-THE-REPO",
    );
    let mut asked = std::collections::BTreeSet::new();
    asked.insert("codex-cli".to_string());
    meltemid::user_scope::record_consent(&config, &asked).expect("consent");
    unsafe { std::env::set_var("CODEX_HOME", &codex_home) };

    meltemid::user_scope::project_user_scope(&config, &["codex-cli".to_string()])
        .expect("projection");
    let text = std::fs::read_to_string(codex_home.join("AGENTS.md")).expect("written");
    assert!(text.contains("MY-OWN-GLOBAL-RULE"), "{text}");
    assert!(
        !text.contains("TEAM-CONTENT-THAT-MUST-NOT-LEAVE-THE-REPO"),
        "a project rule reached a file that applies to every repository:\n{text}"
    );

    unsafe { std::env::remove_var("CODEX_HOME") };
    let _ = std::fs::remove_dir_all(&root);
}
