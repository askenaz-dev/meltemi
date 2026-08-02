// SPDX-License-Identifier: Apache-2.0

//! Verification of the `multiproyecto-suscripciones` scenarios that the unit and
//! e2e suites do not already cover, each named by its exact scenario so the
//! verification linker recognizes it.
//!
//! Two kinds of check live here. Behavioural ones drive the real code over
//! temporary fixtures. Surface ones read the repository and assert the wiring a
//! scenario claims — the convention this project already uses for the desktop
//! client (`desktop/tests/surface.rs`), because a Svelte view cannot be driven
//! from a Rust test. Each surface check asserts a specific symbol or string, not
//! the mere existence of a file.

use std::path::{Path, PathBuf};

use meltemi_proto::{FleetResolutionSource, SessionEventKind, TurnStatus};
use meltemid::session_index::{self, SessionRecord};
use meltemid::session_log::SessionLog;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

/// A fixture directory, spelled the way the registry will spell it — the
/// registry canonicalizes every root it records, and a temporary directory is
/// exactly where that differs from what we typed: a Windows CI runner hands out
/// the 8.3 short form and macOS reaches `/var` through a symlink into
/// `/private/var`, while a developer's own temp directory resolves to itself.
fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mel-scen-multi-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.canonicalize().map(strip_verbatim).unwrap_or(dir)
}

/// Windows answers `canonicalize` in the `\\?\` extended-length form, which the
/// registry strips back off for a plain drive path; a fixture that kept it would
/// compare unequal for a reason that has nothing to do with the test.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let shown = path.to_string_lossy();
        if let Some(plain) = shown.strip_prefix(r"\\?\")
            && !plain.starts_with("UNC\\")
        {
            return PathBuf::from(plain);
        }
    }
    path
}

// ---- the project registry ---------------------------------------------------

// Scenario: Registro reconstruido desde el índice de sesiones
#[test]
fn the_registry_rebuilds_itself_from_the_session_index() {
    let data = temp("rebuild");
    let project = temp("repo-rebuild");
    let key = meltemid::paths::project_key(&project);
    // A session index exists, but the project registry file does not: the logs
    // and the index remain the source of truth.
    session_index::append(
        &data,
        &key,
        &SessionRecord {
            session_id: "s1".into(),
            agent_command: vec!["mock-agent".into()],
            project_root: project.display().to_string(),
            level: 1,
            started_at: "2026-07-20T10:00:00Z".into(),
            ended_at: Some("2026-07-20T10:01:00Z".into()),
            final_status: Some(TurnStatus::Completed),
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: None,
            profile: None,
            source: None,
        },
    )
    .expect("index record");

    let rebuilt = meltemid::projects::rebuild_from_sessions(&data);
    assert_eq!(rebuilt.len(), 1, "the project is recovered: {rebuilt:#?}");
    assert_eq!(rebuilt[0].project_key, key);
    assert_eq!(rebuilt[0].root, project.display().to_string());
    let _ = std::fs::remove_dir_all(&data);
    let _ = std::fs::remove_dir_all(&project);
}

// Scenario: Sesión sin perfil se lista sin campos inventados
#[test]
fn a_session_without_a_profile_reports_no_profile_at_all() {
    let data = temp("no-profile");
    let key = "k";
    session_index::append(
        &data,
        key,
        &SessionRecord {
            session_id: "plain".into(),
            agent_command: vec!["mock-agent".into()],
            project_root: "/repo".into(),
            level: 1,
            started_at: "2026-07-20T10:00:00Z".into(),
            ended_at: None,
            final_status: None,
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: None,
            profile: None,
            source: None,
        },
    )
    .expect("index record");

    let records = session_index::records_for_project(&data, key);
    assert_eq!(records.len(), 1);
    assert!(records[0].profile.is_none(), "no subscription is invented");
    assert!(records[0].agent_id.is_none(), "no catalog id is invented");
    // And the serialized record omits them entirely rather than nulling them.
    let json = serde_json::to_string(&records[0]).expect("serialize");
    assert!(
        !json.contains("profile") && !json.contains("agentId"),
        "absent fields are omitted, not emitted empty: {json}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

#[test]
fn the_registry_orders_by_recency_and_keeps_the_first_sighting() {
    // Underpins "Dos proyectos listados por recencia" at the registry level.
    let data = temp("recency");
    let older = temp("repo-older");
    let newer = temp("repo-newer");
    meltemid::projects::touch(&data, &older);
    meltemid::projects::touch(&data, &newer);
    // Using the older project again moves it to the front.
    meltemid::projects::touch(&data, &older);

    let listed = meltemid::projects::list(&data);
    assert_eq!(listed.len(), 2, "no duplicates: {listed:#?}");
    assert_eq!(
        listed[0].root,
        older.display().to_string(),
        "most recently used first"
    );
    // The first sighting is the EARLIEST one ever recorded for that project,
    // not simply some timestamp before the last: read the raw lines and compare.
    let raw = std::fs::read_to_string(data.join("projects").join("index.jsonl")).expect("index");
    let older_key = meltemid::paths::project_key(&older);
    let first_lines: Vec<String> = raw
        .lines()
        .filter(|line| line.contains(&older_key))
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("line parses")["firstSeenAt"]
                .as_str()
                .expect("firstSeenAt")
                .to_string()
        })
        .collect();
    assert!(
        first_lines.len() >= 2,
        "the project was touched more than once: {first_lines:?}"
    );
    assert!(
        first_lines.iter().all(|seen| *seen == first_lines[0]),
        "every later touch repeats the original first sighting: {first_lines:?}"
    );
    assert_eq!(
        listed[0].first_seen_at, first_lines[0],
        "and the fold reports that same first sighting"
    );
    for dir in [&data, &older, &newer] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

// ---- the guide --------------------------------------------------------------

// Scenario: Ejemplo canónico de dos cuentas del mismo agente
#[test]
fn the_guide_shows_two_accounts_of_the_same_agent() {
    let guide = read("docs/agentes.md");
    let section = guide
        .split("## Multiple accounts and subscriptions")
        .nth(1)
        .expect("the guide has a multi-subscription section");
    // Two profiles of the SAME underlying agent, named differently.
    let profiles: Vec<&str> = section
        .lines()
        .filter(|line| line.trim_start().starts_with("name = "))
        .collect();
    assert!(
        profiles.len() >= 2,
        "the canonical example declares two profiles: {profiles:?}"
    );
    let agents: Vec<&str> = section
        .lines()
        .filter(|line| line.trim_start().starts_with("agent = "))
        .collect();
    assert!(
        agents.len() >= 2 && agents[0] == agents[1],
        "both profiles must launch the same agent: {agents:?}"
    );
    assert!(
        section.contains("${"),
        "the example redirects the auth context through an environment reference"
    );
}

// Scenario: La guía no pide credenciales
#[test]
fn the_guide_never_asks_for_a_credential() {
    let guide = read("docs/agentes.md");
    let lower = guide.to_lowercase();
    // The guide may explain that credentials are never touched; it must never
    // instruct the reader to paste one into Meltemi's configuration.
    for forbidden in [
        "api_key =",
        "api-key =",
        "apikey =",
        "token =",
        "password =",
        "secret =",
        "sk-",
    ] {
        assert!(
            !lower.contains(forbidden),
            "the guide must not ask for a credential ({forbidden})"
        );
    }
    assert!(
        lower.contains("never reads, stores or forwards credentials")
            || lower.contains("never read, store or forward"),
        "the guide states the fair-play rule explicitly"
    );
}

// ---- the desktop surface (source-level wiring) -------------------------------

// Scenario: Árbol con dos proyectos y sus sesiones
#[test]
fn the_sidebar_renders_a_project_tree_aggregated_in_the_client() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("groupSessions($projects, $allSessions)"),
        "the tree joins the project registry with the global session list"
    );
    assert!(
        sidebar.contains("role=\"tree\"") && sidebar.contains("role=\"treeitem\""),
        "the tree is announced as a tree to assistive technology"
    );
    assert!(
        sidebar.contains("onOpenSession(session.sessionId)"),
        "a leaf opens its session"
    );
    // The aggregation itself is unit-tested in the frontend suite, which CI runs.
    let tree_tests = read("desktop/ui/tests/tree.test.ts");
    assert!(
        tree_tests.contains("sessions group under their project, most recent first"),
        "the grouping has an executed unit test"
    );
}

// Scenario: Dos suscripciones del mismo agente distinguibles
#[test]
fn the_tree_distinguishes_two_subscriptions_of_the_same_agent() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("agentLabelOf(session)") && sidebar.contains("session.profile"),
        "each leaf carries the agent identity and its subscription"
    );
    assert!(
        sidebar.contains("<Avatar id={agentLabelOf(session)}"),
        "the same agent keeps one visual identity across subscriptions"
    );
    let tree_tests = read("desktop/ui/tests/tree.test.ts");
    assert!(
        tree_tests.contains("two subscriptions of the same agent stay distinguishable"),
        "the case has an executed unit test"
    );
}

// Scenario: Proyecto ausente en disco marcado en el árbol
#[test]
fn an_absent_root_is_marked_in_the_tree_and_in_the_switcher() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("{#if !group.exists}") && sidebar.contains("projects.absent"),
        "the tree marks a root that no longer exists"
    );
    let switcher = read("desktop/ui/src/lib/components/ProjectSwitcher.svelte");
    assert!(
        switcher.contains("projects.absent.remedy"),
        "the switcher states the remedy next to the mark"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    assert!(
        catalog.contains("\"projects.absent\"") && catalog.contains("\"projects.absent.remedy\""),
        "both strings live in the ES/EN catalog"
    );
}

// Scenario: El árbol no salta bajo el cursor
#[test]
fn the_tree_never_animates_its_layout() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    // No layout animation: neither Svelte transitions nor CSS animation on the
    // rows the pointer is over.
    for forbidden in [
        "transition:",
        "animate:",
        "flip",
        "@keyframes",
        "animation:",
    ] {
        assert!(
            !sidebar.contains(forbidden),
            "the sidebar tree must not animate its layout ({forbidden})"
        );
    }
}

// Scenario: Conmutar el proyecto activo reenfoca las vistas
#[test]
fn switching_the_project_renarrows_the_views_and_refetches() {
    let stores = read("desktop/ui/src/lib/stores.ts");
    let switch = stores
        .split("export function switchProject")
        .nth(1)
        .expect("switchProject exists");
    let body = switch.split("\n}").next().expect("function body");
    assert!(
        body.contains("activeProject.set(root)")
            && body.contains("setActiveProject(root)")
            && body.contains("scopedTo(root, get(allSessions))")
            && body.contains("refreshSessions()"),
        "switching sets the scope, persists it, renarrows what is shown and refetches: {body}"
    );
}

// Scenario: El ámbito activo sobrevive al reinicio
#[test]
fn the_active_project_is_persisted_in_the_data_directory() {
    let uistate = read("desktop/src/uistate.rs");
    assert!(
        uistate.contains("active_project"),
        "the persisted UI state carries the active project"
    );
    let app = read("desktop/ui/src/App.svelte");
    assert!(
        app.contains("initProjectScope(state.activeProject)"),
        "the surface resolves its scope from the persisted state on start"
    );
    let stores = read("desktop/ui/src/lib/stores.ts");
    assert!(
        stores.contains("const root = persisted ?? cwd"),
        "the persisted project wins over the working directory"
    );
}

// Scenario: Sin proyecto activo las vistas globales siguen vivas
#[test]
fn without_an_active_project_the_global_views_still_answer() {
    let stores = read("desktop/ui/src/lib/stores.ts");
    let refresh = stores
        .split("export async function refreshSessions")
        .nth(1)
        .expect("refreshSessions exists");
    let body = refresh.split("\n}").next().expect("function body");
    assert!(
        body.contains("request<{ sessions: SessionInfo[] }>(\"session/list\", {})"),
        "the session query is global, so nothing depends on a scope being set"
    );
    assert!(
        body.contains("root ? scopedTo(root, result.sessions) : result.sessions"),
        "with no scope the surface shows every session instead of nothing: {body}"
    );
}

// Scenario: Lanzar en otro proyecto sin conmutar antes
#[test]
fn the_composer_targets_a_project_without_switching_the_scope() {
    // The modal launcher is gone; the composer is where a launch is aimed.
    let composer = read("desktop/ui/src/lib/views/Home.svelte");
    assert!(
        composer.contains("const root = $derived(picked ?? $activeProject ?? \"\")"),
        "the composer follows the active project until the user names another"
    );
    assert!(
        composer.contains("picked = project.root;"),
        "the target project is selectable in the composer"
    );
    assert!(
        !composer.contains("switchProject("),
        "launching elsewhere must not move the surface's scope"
    );
    // Every mode sends the composer's own root, not the store's.
    let params = composer
        .split("function paramsFor")
        .nth(1)
        .expect("the composer builds its params in one place")
        .split(
            "
  }",
        )
        .next()
        .expect("body");
    assert_eq!(
        params.matches("projectRoot: root").count(),
        3,
        "all three modes launch against the selected project: {params}"
    );
}

// Scenario: Suscripción elegible por nombre en el lanzador
#[test]
fn the_composer_offers_subscriptions_by_name() {
    let composer = read("desktop/ui/src/lib/views/Home.svelte");
    assert!(
        composer.contains("entry.detected || entry.source === \"profile\""),
        "launch profiles are offered alongside detected agents"
    );
    assert!(
        composer.contains("title={$t(\"sessions.subscription\")}>{entry.profile}"),
        "a profile is shown by its subscription name"
    );
    assert!(
        composer.contains("entry.underlyingAgent ?? entry.id"),
        "the avatar keys off the underlying agent, so identity survives the profile"
    );
}

// ---- the log the resolution writes ------------------------------------------

#[test]
fn the_resolution_event_carries_the_subscription_name_and_no_environment() {
    // Underpins "El listado nunca lleva el contexto de autenticación" at the log
    // level: what is written is a name, never the overlay it selected.
    let data = temp("resolution-log");
    let mut log = SessionLog::create(&data, "k", "s1").expect("log");
    log.append(SessionEventKind::AgentResolved {
        binary: "mock-agent".into(),
        source: FleetResolutionSource::Profile,
        profile: Some("work".into()),
        agent_id: Some("mock-agent".into()),
        level: 1,
    })
    .expect("append");
    let written = std::fs::read_to_string(log.path()).expect("read log");
    assert!(written.contains("\"profile\":\"work\""));
    assert!(
        !written.to_uppercase().contains("CLAUDE_CONFIG_DIR")
            && !written.contains("env")
            && !written.contains("MELTEMI_MOCK_MARKER"),
        "the event carries the profile NAME only: {written}"
    );
    let _ = std::fs::remove_dir_all(&data);
}

// Scenario: Ámbito de proyecto conmutado desde la paleta
#[test]
fn the_palette_scope_resolves_to_a_real_project_root() {
    // The terminal surface's scope is not a display filter: the typed text is
    // resolved against the known projects into a root, and every scoped call is
    // then made against that root instead of the working directory.
    let shell = read("tui/src/shell/mod.rs");
    let effect = shell
        .split("Some(Effect::RefreshProjects) =>")
        .nth(1)
        .expect("the shell handles the scope effect")
        .split(
            "
        }",
        )
        .next()
        .expect("arm body");
    assert!(
        effect.contains("live.projects") && effect.contains("Command::SetScope"),
        "the typed text is resolved against the known projects: {effect}"
    );
    assert!(
        effect.contains("project.root.to_lowercase().contains(&needle)"),
        "by matching the project's own root"
    );

    let conn = read("tui/src/shell/conn.rs");
    assert!(
        conn.contains("Some(Command::SetScope(root)) => {"),
        "the connection actor holds the scope"
    );
    let fleet = conn
        .split("async fn refresh_fleet")
        .nth(1)
        .expect("the fleet fetcher")
        .split(
            "
}",
        )
        .next()
        .expect("body");
    assert!(
        fleet.contains("scope.map(str::to_string).or_else"),
        "a scoped call uses the scope and falls back to the working directory: {fleet}"
    );
    let context = conn
        .split("async fn project_context")
        .nth(1)
        .expect("the context fetcher")
        .split(
            "
}",
        )
        .next()
        .expect("body");
    assert!(
        context.contains("Some(root) => root.to_string()"),
        "and so does the projection: {context}"
    );
}
