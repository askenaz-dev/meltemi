// SPDX-License-Identifier: Apache-2.0

//! Verification of the desktop-shell scenarios of `gui-tauri-paridad` and
//! `gui-clase-mundial`.
//!
//! A Svelte view cannot be driven from a Rust test, so — following the
//! convention `surface.rs` already established for this surface — each check
//! reads the source and asserts the specific wiring its scenario claims: a
//! symbol, a string, a store call, an attribute. Where the frontend has an
//! executed unit test for the behaviour (`desktop/ui/tests/*.test.ts`, run by
//! CI), the check also asserts that the case is named there, so the link is to
//! something that runs and not merely to something that exists.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

fn app() -> String {
    read("desktop/ui/src/App.svelte")
}

/// The markup of the routed view region, i.e. what a view can influence.
fn main_region(app: &str) -> &str {
    let start = app.find("<main>").expect("the shell has a main region");
    let end = app.find("</main>").expect("main closes");
    &app[start..end]
}

// ---- shell architecture ------------------------------------------------------

// Scenario: Tres zonas presentes
// Scenario: Aterrizaje con daemon y sesiones
#[test]
fn the_shell_has_three_zones_and_lands_on_the_composer() {
    let app = app();
    // Sidebar, top bar and status bar are mounted once, OUTSIDE the routed view,
    // so no view can remove or displace them.
    for component in ["<Sidebar", "<TopBar", "<StatusBar"] {
        assert_eq!(
            app.matches(component).count(),
            1,
            "{component} is mounted exactly once"
        );
        assert!(
            !main_region(&app).contains(component),
            "{component} must live outside the routed view"
        );
    }
    assert!(
        app.contains("let view: ViewId = $state(\"home\")"),
        "a first run lands on the conversational composer"
    );
    // And the composer is remembered like any other view, so restoring the last
    // view on open keeps holding rather than being excepted for the landing.
    assert!(
        app.contains("[\"home\", ...KEYED_VIEWS,"),
        "the composer is restorable, not a view the shell forgets"
    );
    // The chrome carries the three signals the scenario names.
    let status = read("desktop/ui/src/lib/components/StatusBar.svelte");
    assert!(
        status.contains("$conn.state === \"connected\"") && status.contains("conn.unreachable"),
        "the status bar states the daemon's connection either way"
    );
    let top = read("desktop/ui/src/lib/components/TopBar.svelte");
    assert!(
        top.contains("$pending.length") && top.contains("permissions.waitingWord"),
        "the top bar carries the permission counter with its word"
    );
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("nav.noProject") && sidebar.contains("$activeProject"),
        "the sidebar names the project scope, or says there is none"
    );
}

#[test]
fn the_chrome_seeds_the_project_registry_on_connect() {
    // The sidebar tree groups sessions under the KNOWN projects, so the registry
    // must be fetched as part of connecting. Without it every session becomes its
    // own inferred node and a worktree session reads as a separate project.
    let app = app();
    let seed = app
        .split("const seed = conn.subscribe")
        .nth(1)
        .expect("the shell seeds state on connect")
        // The closing brace of the subscribe, at its own indentation: `});`
        // alone also matches the `.catch(() => {});` inside the block.
        .split(
            "
    });",
        )
        .next()
        .expect("seed body");
    for fetch in ["refreshPending()", "refreshProjects()", "refreshSessions()"] {
        assert!(seed.contains(fetch), "connecting must seed {fetch}: {seed}");
    }
}

// Scenario: Contadores vivos en el sidebar
#[test]
fn the_sidebar_counters_come_from_live_state() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    let counter = sidebar
        .split("function counterFor")
        .nth(1)
        .expect("the sidebar computes its counters");
    let body = counter.split("\n  }").next().expect("body");
    assert!(
        body.contains("liveSessions") && body.contains("$pending.length"),
        "sessions and pending permissions feed their own counters: {body}"
    );
    assert!(
        sidebar.contains("session.state === \"active\"")
            && sidebar.contains("session.state === \"waiting_permission\""),
        "\"live\" is a state of the session, not a guess"
    );
    // A permission counter is a warning; a session counter is not.
    assert!(
        body.contains("warn: true") && body.contains("warn: false"),
        "the two counters are distinguishable beyond their number"
    );
}

// Scenario: La barra de estado dice la verdad de conexión
#[test]
fn the_status_bar_tells_the_truth_about_the_connection() {
    let status = read("desktop/ui/src/lib/components/StatusBar.svelte");
    for state in ["connected", "connecting", "unreachable"] {
        assert!(
            status.contains(state),
            "the status bar has a rendering for `{state}`"
        );
    }
    assert!(
        status.contains("$conn.endpoint"),
        "it names the endpoint, so a wrong socket is diagnosable"
    );
    // Symbol plus word, never colour alone.
    assert!(
        status.contains("aria-hidden=\"true\"") && status.contains("$t("),
        "the state is a glyph and a localized word"
    );
}

// ---- density and depth -------------------------------------------------------

// Scenario: Fila con jerarquía y selección visible
// Scenario: Categorías como pills
#[test]
fn rows_have_hierarchy_and_categories_render_as_pills() {
    let css = read("desktop/ui/src/app.css");
    let cell = css
        .split("table.dense td {")
        .nth(1)
        .expect("dense cells")
        .split('}')
        .next()
        .expect("body");
    assert!(
        cell.contains("height: var(--row-h)") && cell.contains("var(--cell-pad)"),
        "the dense row is 32/8 by token: {cell}"
    );
    // Selection is visible without relying on colour: a left border marks it.
    assert!(
        css.contains("table.dense tbody tr") && css.contains("border-left"),
        "a row's selection is a shape, not only a tint"
    );
    let pill = css
        .split(".pill {")
        .nth(1)
        .expect(".pill")
        .split('}')
        .next()
        .expect("body");
    assert!(
        pill.contains("border-radius: var(--radius-control)"),
        "a pill takes the 4 px control radius, never a lozenge: {pill}"
    );
    // Categories in the palette are pills, not headings.
    let palette = read("desktop/ui/src/lib/components/Palette.svelte");
    assert!(
        palette.contains("class=\"pill") || palette.contains("class=\"group"),
        "the palette groups by category visibly"
    );
}

// Scenario: La bandeja no se mueve bajo el cursor
// Scenario: Orden de prioridad de señales
#[test]
fn the_tray_never_moves_and_the_signal_order_is_explicit() {
    let app = app();
    // The order is computed, not implied by markup.
    assert!(
        app.contains("const topSignal = $derived.by<\"daemon\" | \"permission\" | \"none\">"),
        "the shell computes which signal outranks the others"
    );
    let signal = app
        .split("const topSignal")
        .nth(1)
        .expect("topSignal")
        .split("});")
        .next()
        .expect("body");
    let daemon_at = signal.find("return \"daemon\"").expect("daemon leg");
    let permission_at = signal
        .find("return \"permission\"")
        .expect("permission leg");
    assert!(
        daemon_at < permission_at,
        "an unreachable daemon outranks a pending permission: {signal}"
    );
    // And the DOM order matches it: the banner precedes the tray.
    let banner_at = app
        .find("$conn.state === \"unreachable\"")
        .expect("the daemon banner");
    let topbar_at = app.find("<TopBar").expect("the top bar");
    assert!(
        banner_at < topbar_at,
        "the banner is met before the tray, by both eye and screen reader"
    );
    // The tray only announces itself while it IS the top signal.
    assert!(
        app.contains("urgent={topSignal === \"permission\"}"),
        "the tray's urgency follows the computed order"
    );
    let top = read("desktop/ui/src/lib/components/TopBar.svelte");
    assert!(
        top.contains("aria-live={urgent ? \"polite\" : \"off\"}"),
        "with the daemon down the tray does not also shout"
    );
    // Nothing in the tray animates its layout.
    let permissions = read("desktop/ui/src/lib/views/Permissions.svelte");
    // Declaring `transition: none` is the opposite of animating; what must not
    // appear is an actual animation of the layout.
    for forbidden in ["transition:in", "animate:", "@keyframes", "animation-name"] {
        assert!(
            !permissions.contains(forbidden),
            "the tray must not animate its layout ({forbidden})"
        );
    }
    for line in permissions
        .lines()
        .filter(|l| l.trim_start().starts_with("transition:"))
    {
        assert!(
            line.contains("none"),
            "the tray declares no transition of its own: {line}"
        );
    }
}

// Scenario: Avatar estable por id
#[test]
fn an_avatar_is_stable_for_an_id() {
    let agents = read("desktop/ui/src/lib/agents.ts");
    assert!(
        agents.contains("fnv1a") || agents.contains("hash"),
        "the tone comes from a hash of the id, so it never moves"
    );
    let tests = read("desktop/ui/tests/agents.test.ts");
    assert!(
        tests.contains("stable") || tests.contains("same id"),
        "the stability has an executed unit test"
    );
    let avatar = read("desktop/ui/src/lib/components/Avatar.svelte");
    assert!(
        avatar.contains("toneFor(id)") && avatar.contains("initialsFor("),
        "the avatar is derived from the id, never random"
    );
}

// ---- the drawer --------------------------------------------------------------

// Scenario: Drawer de agente con acciones
// Scenario: Esc cierra el panel primero
#[test]
fn the_drawer_keeps_the_list_and_escape_closes_it_first() {
    let fleet = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        fleet.contains("<Drawer"),
        "the detail is a drawer, so the list stays on screen"
    );
    let drawer = read("desktop/ui/src/lib/components/Drawer.svelte");
    assert!(
        drawer.contains("event.key === \"Escape\"") && drawer.contains("onClose"),
        "Escape closes the drawer"
    );
    assert!(
        drawer.contains("stopPropagation"),
        "and it closes the drawer FIRST: the key does not also reach the shell"
    );
    // The drawer carries the remedy actions the fleet needs.
    assert!(
        fleet.contains("remedyCommand") && fleet.contains("legalNote"),
        "the drawer offers the per-layer remedy and states the legal note"
    );
}

// ---- settings ----------------------------------------------------------------

// Scenario: La plantilla de Abrir con se configura y persiste
#[test]
fn the_open_with_template_is_configurable_and_persisted() {
    let settings = read("desktop/ui/src/lib/views/Settings.svelte");
    assert!(
        settings.contains("openWithTemplate"),
        "Settings edits the template"
    );
    assert!(
        settings.contains("setOpenWithTemplate") || settings.contains("saveUiState"),
        "the edit is persisted, not held in memory"
    );
    let uistate = read("desktop/src/uistate.rs");
    assert!(
        uistate.contains("open_with_template"),
        "the persisted UI state carries it"
    );
    // And the opener honors it before any fallback.
    let fsops = read("desktop/src/fsops.rs");
    let open_with = fsops.split("pub fn open_with").nth(1).expect("open_with");
    let template_at = open_with.find("template").expect("template is consulted");
    let env_at = open_with.find("MELTEMI_OPEN_WITH").expect("env fallback");
    assert!(
        template_at < env_at,
        "the configured template wins over the environment fallback"
    );
}

// Scenario: Ver y editar la configuración efectiva
#[test]
fn settings_shows_the_effective_configuration_and_can_edit_it() {
    let settings = read("desktop/ui/src/lib/views/Settings.svelte");
    assert!(
        settings.contains("context/project") || settings.contains("effective"),
        "Settings surfaces the effective configuration"
    );
    assert!(
        settings.contains("onEditFile"),
        "and it offers editing the file that produced it"
    );
}

// Scenario: La promesa de privacidad es visible
#[test]
fn the_privacy_promise_is_visible_in_settings() {
    let settings = read("desktop/ui/src/lib/views/Settings.svelte");
    assert!(
        settings.contains("settings.privacy"),
        "Settings states the privacy posture"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    let privacy: Vec<&str> = catalog
        .lines()
        .filter(|line| line.contains("\"settings.privacy"))
        .collect();
    assert!(
        privacy.len() >= 2,
        "the promise is translated in both catalogs: {privacy:?}"
    );
    let text = privacy.join(" ").to_lowercase();
    assert!(
        text.contains("telemetr") && (text.contains("local") || text.contains("máquina")),
        "it says what it promises: no telemetry, everything local"
    );
}

// ---- identity ----------------------------------------------------------------

// Scenario: Marca presente en el chrome
// Scenario: Alto contraste no borra la marca
#[test]
fn the_mark_is_in_the_chrome_and_survives_forced_colors() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("class=\"mark\""),
        "the chrome carries the brand mark"
    );
    let css = read("desktop/ui/src/app.css");
    let forced = css
        .split("@media (forced-colors: active)")
        .nth(1)
        .expect("the client honors forced colors");
    assert!(
        forced.contains("wordmark") || forced.contains("Meltemi") || forced.contains("mark"),
        "the mark has a forced-colors fallback: {forced}"
    );
}

// Scenario: Estados vacíos sin emoji de plataforma
#[test]
fn empty_states_use_the_line_icons_not_platform_emoji() {
    let empty = read("desktop/ui/src/lib/components/EmptyState.svelte");
    assert!(
        empty.contains("<Icon"),
        "an empty state draws its own line icon"
    );
    // No emoji anywhere in the shell's components: they are platform art.
    let root = repo_root().join("desktop/ui/src");
    let mut stack = vec![root];
    let mut offenders: Vec<String> = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let is_source = path
                .extension()
                .is_some_and(|e| e == "svelte" || e == "ts" || e == "css");
            if !is_source {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for ch in text.chars() {
                let code = ch as u32;
                // Emoji blocks (pictographs, transport, misc symbols) — the
                // geometric glyphs the design system uses are NOT in these.
                let emoji = (0x1F300..=0x1FAFF).contains(&code)
                    || (0x1F000..=0x1F2FF).contains(&code)
                    || code == 0xFE0F;
                if emoji {
                    offenders.push(format!("{}: {ch}", path.display()));
                    break;
                }
            }
        }
    }
    assert!(offenders.is_empty(), "platform emoji found: {offenders:?}");
}

// ---- the palette -------------------------------------------------------------

// Scenario: Subsecuencia encuentra el método
// Scenario: Recientes primero
// Scenario: Capacidad sin vista dedicada alcanzable
#[test]
fn the_palette_ranks_by_subsequence_and_remembers_what_was_used() {
    let fuzzy = read("desktop/ui/src/lib/fuzzy.ts");
    assert!(
        fuzzy.contains("export function fuzzyRank"),
        "the palette ranks with its own subsequence matcher"
    );
    let tests = read("desktop/ui/tests/fuzzy.test.ts");
    assert!(
        tests.contains("subsequence") || tests.contains("ranks"),
        "the matcher has an executed unit test"
    );
    let palette = read("desktop/ui/src/lib/components/Palette.svelte");
    assert!(
        palette.contains("paletteUsage")
            || palette.contains("frecency")
            || palette.contains("recent"),
        "recently used methods are offered first"
    );
    let uistate = read("desktop/src/uistate.rs");
    assert!(
        uistate.contains("palette_usage"),
        "that memory is persisted with the rest of the UI state"
    );
    // Every method is reachable, view or no view.
    let registry = read("desktop/ui/src/lib/registry.ts");
    assert!(
        registry.contains("REGISTRY: RegistryEntry[]"),
        "the palette is backed by the typed registry"
    );
}

// Scenario: Formulario tipado con obligatorios marcados
// Scenario: La frescura del generador es un gate
#[test]
fn typed_forms_mark_required_fields_and_their_freshness_is_a_gate() {
    let forms = read("desktop/ui/src/lib/generated/method-forms.ts");
    assert!(
        forms.contains("\"required\": true"),
        "the generated forms know which fields are required"
    );
    let component = read("desktop/ui/src/lib/components/MethodForm.svelte");
    assert!(
        component.contains("{#if field.required}") && component.contains("class=\"req\""),
        "a required field is marked to the eye"
    );
    assert_eq!(
        component.matches("aria-required={field.required}").count(),
        3,
        "and to assistive technology, on every kind of control"
    );
    let package = read("desktop/ui/package.json");
    assert!(
        package.contains("check:forms"),
        "there is a freshness check for the generator"
    );
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains("check:forms"),
        "and CI runs it, so a stale form fails the build"
    );
    let tests = read("desktop/ui/tests/forms.test.ts");
    assert!(
        tests.contains("required"),
        "the required-field handling has an executed unit test"
    );
}

// ---- the session as the primary action ---------------------------------------

// Scenario: Nueva sesión desde el chrome
// Scenario: Proponer sigue a una tecla
#[test]
fn a_new_session_is_the_primary_action_and_propose_is_one_key_away() {
    let top = read("desktop/ui/src/lib/components/TopBar.svelte");
    assert!(
        top.contains("class=\"primary\"") && top.contains("session.new"),
        "the chrome's primary button starts a session"
    );
    let app = app();
    assert!(
        app.contains("event.key.toLowerCase() === \"n\""),
        "and a key does the same"
    );
    // Both go through the one door, and it opens the composer.
    assert!(
        app.contains("onNewSession={() => openComposer()}") && app.contains("view = \"home\";"),
        "the primary action and its key open the conversational composer"
    );
    // Free is what a new session is, before anything is chosen.
    let home = read("desktop/ui/src/lib/views/Home.svelte");
    assert!(
        home.contains("initialMode = \"free\""),
        "the composer preselects the free mode"
    );
    // Propose is a mode of that same composer, reached from the Project view
    // rather than from the chrome: a tool, not the centre of the shell.
    assert!(
        app.contains("onPropose={() => openComposer(\"propose\")}"),
        "proposing routes to the composer with its mode already chosen"
    );
    assert!(
        !top.contains("propose"),
        "the chrome does not make propose the headline action"
    );
}

// Scenario: Inicializar desde el vacío de Proyecto
// Scenario: Solo Proyecto queda vacío sin `.meltemi/`
#[test]
fn only_the_project_view_degrades_without_a_meltemi_directory() {
    let project = read("desktop/ui/src/lib/views/Project.svelte");
    // The absence is PROBED, not inferred from an empty list: the daemon
    // tolerates a directory with no `.meltemi/`.
    assert!(
        project.contains("isMeltemiProject(root)"),
        "the view asks whether the marker exists"
    );
    let files = read("desktop/ui/src/lib/editor/files.ts");
    let probe = files
        .split("export async function isMeltemiProject")
        .nth(1)
        .expect("the probe exists");
    assert!(
        probe.contains(".meltemi/constitution.md"),
        "it looks for a real marker of an initialized project"
    );
    assert!(
        project.contains("isProject = false") && project.contains("<EmptyState"),
        "without it the view offers the initialization path"
    );
    // And nothing else in the shell depends on the project being initialized.
    let app = app();
    assert!(
        !app.contains("isProject"),
        "the shell does not gate other views on the project marker"
    );
}

// Scenario: Flota vacía ofrece refrescar
#[test]
fn an_empty_fleet_offers_to_look_again() {
    let fleet = read("desktop/ui/src/lib/views/Fleet.svelte");
    assert!(
        fleet.contains("<EmptyState") && fleet.contains("refreshFleet"),
        "with nothing detected the view offers a refresh instead of a dead end"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    assert!(
        catalog.contains("\"fleet.empty."),
        "the empty state's words live in the catalog"
    );
}

// ---- sessions ----------------------------------------------------------------

// Scenario: Filtrar por agente
// Scenario: Tiempo relativo con absoluto accesible
// Scenario: Cancelar desde la fila
#[test]
fn the_session_list_filters_shows_human_time_and_acts_per_row() {
    let view = read("desktop/ui/src/lib/views/Sessions.svelte");
    let rows = view
        .split("const rows = $derived.by")
        .nth(1)
        .expect("the rows are derived")
        .split("return list;")
        .next()
        .expect("body");
    assert!(
        rows.contains("agentLabelOf(session).toLowerCase().includes(needle)"),
        "the filter reaches the agent: {rows}"
    );
    // Relative time with the absolute one reachable, not lost.
    assert!(
        view.contains("relativeTime(session.startedAt, $locale)")
            && view.contains("title={absoluteTime(session.startedAt, $locale)}")
            && view.contains("aria-label={absoluteTime(session.startedAt, $locale)}"),
        "the row shows relative time and exposes the absolute one"
    );
    // Cancelling from the row is gated by a confirmation.
    assert!(
        view.contains("cancelTarget = session.sessionId") && view.contains("<ConfirmDialog"),
        "cancelling from the row asks first"
    );
}

// Scenario: Sesión finalizada sigue accesible
#[test]
fn a_finished_session_stays_in_the_list_marked_as_finished() {
    let view = read("desktop/ui/src/lib/views/Sessions.svelte");
    let rows = view
        .split("const rows = $derived.by")
        .nth(1)
        .expect("rows")
        .split("return list;")
        .next()
        .expect("body");
    assert!(
        rows.contains("if (stateFilter && session.state !== stateFilter) return false;"),
        "the only state comparison is the explicit filter"
    );
    assert!(
        !rows.contains("\"ended\"") && !rows.contains("isLive"),
        "nothing drops a session for having finished: {rows}"
    );
    assert!(
        view.contains("<StatusBadge state={session.state} />"),
        "the row carries its own state mark"
    );
    let badge = read("desktop/ui/src/lib/components/StatusBadge.svelte");
    assert!(
        badge.contains("ended:") && badge.contains("$t((\"state.\" + state)"),
        "the finished mark is glyph plus localized word"
    );
    // The daemon keeps reporting it, which is what the list reads.
    let server = read("core/meltemid/src/server.rs");
    assert!(
        server.contains("None if record.ended_at.is_some() => SessionState::Ended"),
        "the source of the list keeps the finished session"
    );
}

// ---- the transcript ----------------------------------------------------------

// Scenario: Evento con texto expandible
// Scenario: Buscar en el transcript
// Scenario: Tipo desconocido no rompe
#[test]
fn the_transcript_expands_searches_and_survives_an_unknown_event() {
    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    assert!(
        detail.contains("let expanded: Record<number, boolean>")
            && detail.contains("expanded[line.id] && line.full"),
        "a long event keeps its full payload behind an expand"
    );
    assert!(
        detail.contains("class=\"link expand\""),
        "and the expand is an affordance, not a hidden gesture"
    );
    assert!(
        detail.contains("transcript.search"),
        "the transcript is searchable"
    );
    // Search highlights lines of the operator log, so opening it goes there
    // rather than leaving a field that finds what the reading cannot show.
    assert!(
        detail.contains("void switchReading(\"log\");"),
        "searching takes the reader to the reading that can show the hits"
    );
    // An unknown event type renders instead of throwing: the style table is
    // consulted with a fallback, never indexed blindly.
    assert!(
        detail.contains("const EVENT_STYLE: Record<string, { glyph: string; tone: string }>"),
        "known types have a glyph and a tone"
    );
    assert!(
        detail.contains("EVENT_STYLE[") && detail.contains("??"),
        "and an unrecognized type falls back to the neutral style"
    );
}

// Scenario: Caída durante el streaming
// Scenario: Reconexión a un daemon sin la sesión
#[test]
fn a_disconnection_is_loud_and_a_vanished_session_is_stated() {
    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    assert!(
        detail.contains("transcript.cut") || detail.contains("cut"),
        "the transcript marks where it was cut instead of pretending to continue"
    );
    let app = app();
    assert!(
        app.contains("banner.daemonDown") && app.contains("role=\"alert\""),
        "the daemon going away is announced, not hinted"
    );
    // On reconnection a session the daemon no longer knows is stated as gone.
    assert!(
        detail.contains("let gone = $state(false)")
            && detail.contains("sessions.detail.goneAfterReconnect"),
        "a session the daemon no longer knows is reported, not spun forever"
    );
    assert!(
        detail.contains("if (!session) gone = true"),
        "the check is explicit on reconnection"
    );
}

// ---- unsaved work ------------------------------------------------------------

// Scenario: Cerrar pestaña sucia pide decisión
// Scenario: Salir del editor con sucios pide decisión
// Scenario: Cerrar la ventana con sucios pide decisión
#[test]
fn no_edit_is_lost_silently_on_any_exit_path() {
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        editor.contains("closeGuard") && editor.contains("<ConfirmDialog"),
        "closing a dirty tab asks first"
    );
    let app = app();
    assert!(
        app.contains("guard = { kind: \"leave\", go }"),
        "leaving the editor with unsaved work asks first"
    );
    assert!(
        app.contains("listen(\"app:close-requested\"")
            && app.contains("guard = { kind: \"close\" }"),
        "closing the window asks first"
    );
    let host = read("desktop/src/lib.rs");
    assert!(
        host.contains("api.prevent_close()") && host.contains("app:close-requested"),
        "the host holds the close until the surface has decided"
    );
    assert!(
        host.contains("close_confirmed"),
        "and closes only when the surface says so"
    );
    // The dirty set itself has an executed unit test.
    let tests = read("desktop/ui/tests/dirty.test.ts");
    assert!(
        tests.contains("dirty"),
        "the dirty bookkeeping has a unit test"
    );
}

// Scenario: Quick-open por nombre
#[test]
fn quick_open_finds_a_file_by_name() {
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        editor.contains("quickOpen") && editor.contains("fuzzyRank"),
        "quick-open ranks files with the same matcher as the palette"
    );
    assert!(
        editor.contains("Ctrl P") || editor.contains("\"p\""),
        "it has a keyboard shortcut"
    );
}

// ---- persistence -------------------------------------------------------------

// Scenario: El tema sobrevive al reinicio
// Scenario: Override de idioma
#[test]
fn theme_and_locale_survive_a_restart_and_the_override_wins() {
    let uistate = read("desktop/src/uistate.rs");
    assert!(
        uistate.contains("theme") && uistate.contains("locale"),
        "both are persisted in the data directory"
    );
    let app = app();
    assert!(
        app.contains("if (state.locale) setLocale(state.locale)"),
        "the persisted locale is applied on boot"
    );
    let css = read("desktop/ui/src/app.css");
    assert!(
        css.contains(":root[data-theme=\"light\"]") && css.contains(":root[data-theme=\"dark\"]"),
        "the explicit theme override wins over the OS in both directions"
    );
    // No user-visible string bypasses the catalog — including the OS title the
    // host sets, which the surface now supplies.
    let host = read("desktop/src/lib.rs");
    assert!(
        !host.contains("permiso)") && !host.contains("permisos)"),
        "the host no longer writes Spanish prose of its own"
    );
    assert!(
        host.contains("title: Option<String>"),
        "the host receives the localized title from the surface"
    );
    let stores = read("desktop/ui/src/lib/stores.ts");
    assert!(
        stores.contains("window.title.pending"),
        "and the surface takes it from the catalog"
    );
}

// Scenario: El atajo de la paleta es visible
#[test]
fn the_palette_shortcut_is_visible_in_the_chrome() {
    let top = read("desktop/ui/src/lib/components/TopBar.svelte");
    assert!(
        top.contains("<kbd>Ctrl K</kbd>"),
        "the chrome shows how to open the palette"
    );
}

// ---- attention ---------------------------------------------------------------

// Scenario: Permiso sin foco reclama atención
// Scenario: El foco limpia la señal
// Scenario: Atender un permiso pendiente
// Scenario: Vencimiento anunciado
#[test]
fn a_permission_claims_attention_and_the_tray_can_be_attended() {
    let stores = read("desktop/ui/src/lib/stores.ts");
    assert!(
        stores.contains("invoke(\"request_attention\""),
        "a permission asks the OS for attention"
    );
    let host = read("desktop/src/lib.rs");
    let attention = host
        .split("fn request_attention")
        .nth(1)
        .expect("the host command exists");
    let body = attention.split("\n}").next().expect("body");
    assert!(
        body.contains("is_focused") && body.contains("pending > 0 && !focused"),
        "attention is asked only while the window is NOT focused: {body}"
    );
    assert!(
        body.contains("None") && body.contains("request_user_attention"),
        "and it is cleared when there is nothing pending"
    );
    // Attending the tray focuses a request however the user got there.
    let app = app();
    assert!(
        app.contains("if (target === \"permissions\") focusPendingRequest();"),
        "every path into the tray focuses a pending request"
    );
    assert!(
        app.contains("[data-autofocus]"),
        "the tray marks what should take the focus"
    );
    let permissions = read("desktop/ui/src/lib/views/Permissions.svelte");
    assert!(
        permissions.contains("data-autofocus"),
        "and the view provides that anchor"
    );
    // The expiry notice names the operation it knew about, never "?".
    let timeout = stores
        .split("case \"permission/timeout\"")
        .nth(1)
        .expect("the timeout is routed");
    let case = timeout
        .split("case \"session/event\"")
        .next()
        .expect("case body");
    assert!(
        case.contains("get(pending).find("),
        "the operation comes from the queue the client already holds: {case}"
    );
    assert!(
        case.contains("permissions.timeout.unknownTool"),
        "and an unknown one is named honestly rather than shown as a question mark"
    );
    assert!(
        !case.contains("params.tool ??"),
        "the notice no longer reads a field the contract never sends"
    );
}

// ---- notices -----------------------------------------------------------------

// Scenario: Overflow de avisos colapsa con historial
// Scenario: Copiar el diagnóstico de conexión
#[test]
fn notices_are_bounded_with_history_and_the_banner_is_actionable() {
    let notices = read("desktop/ui/src/lib/components/Notices.svelte");
    assert!(
        notices.contains("MAX") || notices.contains("slice("),
        "the notice list is bounded"
    );
    assert!(
        notices.contains("history") || notices.contains("notices.more"),
        "the overflow collapses into a history rather than vanishing"
    );
    let app = app();
    assert!(
        app.contains("copyDiagnostics") && app.contains("banner.copyDiagnostics"),
        "the daemon-down banner offers to copy its diagnosis"
    );
    let copy = app
        .split("async function copyDiagnostics")
        .nth(1)
        .expect("copyDiagnostics")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        copy.contains("endpoint") && copy.contains("detail"),
        "the copied text carries the endpoint and the detail: {copy}"
    );
}

// ---- the spec editor and the diff review ------------------------------------

// Scenario: Findings de validación en vivo
#[test]
fn validation_findings_are_about_the_artifact_in_front_of_the_user() {
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    let validate = editor
        .split("async function validate()")
        .nth(1)
        .expect("validate exists")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        validate.contains("changeOfPath(activePath)"),
        "the scope is the change the open file belongs to: {validate}"
    );
    assert!(
        validate.contains("await save(true)"),
        "a dirty buffer is saved first, so the findings describe what the user sees"
    );
    assert!(
        validate.contains("change ? { projectRoot: root, change } : { projectRoot: root }"),
        "and the request carries that scope"
    );
    assert!(
        editor.contains("if (isMethodFile) void validate();"),
        "editing a method artifact validates it without being asked"
    );
}

// Scenario: Guardado trazable de un artefacto
#[test]
fn every_save_goes_through_the_daemon_with_its_trace() {
    let files = read("desktop/ui/src/lib/editor/files.ts");
    assert!(
        files.contains("request<SaveOutcome>(\"worktree/apply-edit\""),
        "the only writer is the daemon's apply-edit"
    );
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        !editor.contains("invoke(\"tree_write") && !editor.contains("writeTextFile"),
        "the surface has no writing path of its own"
    );
    assert!(
        editor.contains("editor.saved") && editor.contains("editor.saved.project"),
        "the surface states where the trace landed"
    );
}

// Scenario: Comparar competidores de una carrera
// Scenario: Edición de un hunk pasa por el daemon
// Scenario: Abrir con el editor del usuario
#[test]
fn the_review_compares_competitors_and_acts_per_hunk_and_per_line() {
    let review = read("desktop/ui/src/lib/views/Review.svelte");
    // Competitors of one task, side by side, over a common base. They used to
    // be tabs — one lane visible at a time, which is not a comparison — and are
    // now lanes laid out beside each other (tablero-de-carrera D3).
    assert!(
        review.contains("class=\"lanes\"") && review.contains("{#each competitors as lane"),
        "every competitor of the race is on screen at once"
    );
    assert!(
        review.contains("review.base") && review.contains("baseRev"),
        "the common base is stated, so the diffs are comparable"
    );
    // Per-hunk unit with its own affordances. The grammar itself lives in the
    // shared module every diff surface reads, so the review and the race board
    // cannot drift into two readings of the same text (tablero-de-carrera D3).
    assert!(
        review.contains("import { fileSections, hunksOf } from \"../diff\"")
            && review.contains("class=\"hunk\""),
        "the diff is grouped into hunks, by the shared parser"
    );
    let diff = read("desktop/ui/src/lib/diff.ts");
    assert!(
        diff.contains("export function fileSections(") && diff.contains("export function hunksOf("),
        "and the shared module is where those two live"
    );
    let diff_test = read("desktop/ui/tests/diff.test.ts");
    assert!(
        diff_test.contains("Los hunks conservan su cabecera y su primera línea nueva"),
        "the grammar has an executed unit test, not merely a home"
    );
    assert!(
        review.contains("review.editHunk"),
        "each hunk offers to be edited"
    );
    assert!(
        review.contains("hunk.startLine ?? undefined"),
        "editing starts at the hunk's own line"
    );
    // Editing goes to the editor, whose only writer is the daemon.
    let app = app();
    assert!(
        app.contains("onEditWorktree={(worktreePath, target, file, line)")
            && app.contains("initialLine: line ?? null"),
        "the shell carries the hunk's line into the editor"
    );
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        editor.contains("void openFile(initialFile, initialLine ?? undefined)"),
        "and the editor opens the file at that line"
    );
    // Per-line "open with", carrying the exact line.
    assert!(
        review.contains("class=\"lineNo\"") && review.contains("review.openLine"),
        "every line offers to open the user's editor"
    );
    assert!(
        review.contains("openExternally(\n                                    activeDiff.path,\n                                    section.file,\n                                    line.newLine,\n                                  )")
            || review.contains("line.newLine,"),
        "and it passes that line, not the head of the file"
    );
}

// Scenario: Vincular desde la ficha del agente
// Scenario: El gesto de login queda a un clic de copiar
// Scenario: La entrada sin variable señala la vía manual
#[test]
fn the_agent_drawer_links_a_subscription_where_the_registry_declares_the_variable() {
    let fleet = read("desktop/ui/src/lib/views/Fleet.svelte");
    // The flow is gated on the registry's own datum, never on a provider name.
    assert!(
        fleet.contains("{#if selected.authContextVar}"),
        "linking is offered exactly where the variable is declared"
    );
    // One field: the name. The daemon composes everything else.
    let link_box = fleet
        .split("{#if selected.authContextVar}")
        .nth(1)
        .expect("link section")
        .split("{:else if")
        .next()
        .expect("section body");
    assert_eq!(
        link_box.matches("<input").count(),
        1,
        "the form asks only the name: {link_box}"
    );
    // The request is the contract's, and the catalog re-reads without reload.
    assert!(
        fleet.contains("request<") && fleet.contains("\"subscription/link\""),
        "the link goes through the contract"
    );
    let link_fn = fleet
        .split("async function linkSubscription()")
        .nth(1)
        .expect("link action")
        .split(
            "
  }",
        )
        .next()
        .expect("body");
    assert!(
        link_fn.contains("await refreshFleet()"),
        "the new row appears without reloading the app: {link_fn}"
    );
    // The gesture lands beside the copy action the fleet already has.
    assert!(
        link_box.contains("{#if gesture}")
            && link_box.contains("copyCommand(gesture!.powershell)")
            && link_box.contains("copyCommand(gesture!.posix)"),
        "the login gesture is one click from copied, in both shells"
    );
    // No declared variable: the manual path is named, not a dead control.
    assert!(
        fleet.contains("fleet.link.manualHint"),
        "an entry without the variable points at the manual path"
    );
    let es = read("desktop/ui/src/lib/messages.ts");
    assert!(
        es.contains("config.toml") && es.contains("[[fleet.profile]]"),
        "the manual hint names the real file and block"
    );
}

// Scenario: Desvincular dice lo que no borra
#[test]
fn unlinking_from_the_drawer_says_the_context_stays() {
    let fleet = read("desktop/ui/src/lib/views/Fleet.svelte");
    // The unlink control lives on profile rows, beside the words that matter.
    assert!(
        fleet.contains("{:else if selected.source === \"profile\"}")
            && fleet.contains("unlinkSubscription()")
            && fleet.contains("fleet.unlink.keeps"),
        "profile rows offer unlink with the it-stays declaration beside it"
    );
    // The declaration exists in both languages, and the done-notice names the
    // directory left behind.
    let messages = read("desktop/ui/src/lib/messages.ts");
    assert!(
        messages.contains("queda intacto") && messages.contains("stays intact"),
        "the it-stays words exist in both locales"
    );
    assert!(
        messages.contains("{dir}"),
        "the unlink notice names the directory left behind"
    );
    // And the row disappears by re-reading the catalog, not by pretending.
    let unlink_fn = fleet
        .split("async function unlinkSubscription()")
        .nth(1)
        .expect("unlink action")
        .split(
            "
  }",
        )
        .next()
        .expect("body");
    assert!(
        unlink_fn.contains("\"subscription/unlink\"") && unlink_fn.contains("await refreshFleet()"),
        "unlink goes through the contract and refreshes: {unlink_fn}"
    );
}

/// Every style-bearing source of the desktop surface, for sweeps that must not
/// miss a file just because nobody remembered to name it.
fn walk_ui_sources() -> Vec<std::path::PathBuf> {
    let mut stack = vec![repo_root().join("desktop/ui/src")];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e == "svelte" || e == "css")
            {
                found.push(path);
            }
        }
    }
    found
}

// Scenario: La tira se recorre entera con el teclado
#[test]
fn the_tab_strip_is_traversable_by_arrow_keys() {
    let strip = read("desktop/ui/src/lib/components/TabStrip.svelte");

    // The full tabs pattern, not a row of buttons that looks like one.
    for attr in [
        "role=\"tablist\"",
        "role=\"tab\"",
        "aria-selected={active}",
        "aria-controls=\"panel-{item.id}\"",
        "id=\"tab-{item.id}\"",
        "aria-orientation=\"horizontal\"",
    ] {
        assert!(strip.contains(attr), "the strip declares {attr}");
    }

    // Roving tabindex: exactly one tab in the tab order at a time. This is what
    // the widened sweep is exempted for, and the exemption is paid for here.
    assert!(
        strip.contains("tabindex={active ? 0 : -1}"),
        "one tab is in the tab order, and the arrows reach the others"
    );

    let keys = strip
        .split("function onKeys")
        .nth(1)
        .expect("the key handler")
        .split("\n  }")
        .next()
        .expect("body");
    for key in ["ArrowRight", "ArrowLeft", "Home", "End", "Delete"] {
        assert!(keys.contains(key), "the strip answers to {key}: {keys}");
    }
    assert!(
        keys.contains("% items.length") && keys.contains("+ items.length"),
        "the arrows wrap at both ends rather than stopping: {keys}"
    );
    assert!(
        keys.contains("preventDefault()") && keys.contains("stopPropagation()"),
        "the strip's keys do not leak to the shell: {keys}"
    );
    assert!(
        keys.contains("if (!items[current].closable) return;"),
        "Delete never closes a tab that has no close control: {keys}"
    );

    // Selection alone is not enough: if focus does not follow, the next arrow
    // press moves from wherever focus actually was.
    let focus = strip
        .split("function focusTab")
        .nth(1)
        .expect("the focus mover")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        focus.contains("onSelect(") && focus.contains(".focus()"),
        "selection and DOM focus move together: {focus}"
    );
}

// Scenario: El árbol de proyectos desplaza sin comerse la columna
// Scenario: La barra sigue el tema sin una segunda declaración
#[test]
fn the_surface_declares_a_thin_scrollbar_from_its_own_tokens() {
    let css = read("desktop/ui/src/app.css");

    assert!(
        css.contains("scrollbar-width: thin"),
        "the surface asks for a bar without stepper buttons"
    );
    assert!(
        css.contains("scrollbar-color: var(--text-faint) transparent"),
        "the bar takes its colour from the token the rest of the chrome uses"
    );

    // Once, inherited, in the block that already sets color-scheme — so every
    // scroller follows and each theme's redefinition of --text-faint carries
    // the colour with no per-theme and no per-region rule.
    assert_eq!(
        css.matches("scrollbar-width").count(),
        1,
        "one declaration for the whole surface, not one per region"
    );
    let root = css
        .split(":root {")
        .nth(1)
        .expect("the root block")
        .split("\n}")
        .next()
        .expect("body");
    assert!(
        root.contains("color-scheme: light dark") && root.contains("scrollbar-width: thin"),
        "the pair rides the inherited cascade beside color-scheme"
    );

    // Engine-specific selectors are refused: they would style one webview and
    // leave the others with whatever they had.
    let mut offenders = Vec::new();
    for path in walk_ui_sources() {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        if text.contains("::-webkit-scrollbar") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "the surface uses standard scrollbar properties, not engine-specific selectors: {offenders:?}"
    );

    // Narrowing the bar must not narrow a row: the tree still scrolls its
    // overflow rather than compressing it.
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    let tree_rule = sidebar
        .split("\n  .tree {")
        .nth(1)
        .expect("the tree rule")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        tree_rule.contains("overflow-y: auto") && tree_rule.contains("min-height: 0"),
        "the tree scrolls its overflow instead of compressing it: {tree_rule}"
    );
}

// Scenario: El reparto se recuerda, el primer arranque no
// Scenario: Una ventana más pequeña no deja el reparto inservible
#[test]
fn the_split_is_remembered_beside_the_other_layout_preferences() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("$state($uiState.navSplit)") && sidebar.contains("setNavSplit("),
        "the split is read back on mount and written when it changes"
    );
    // Written on release, not per pointermove: `persist` writes the WHOLE
    // object, and ~200 writes per drag would feed the race the host already has
    // with its own load-modify-save of the window geometry.
    let moved = sidebar
        .split("function onDragMove")
        .nth(1)
        .expect("the move handler")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        !moved.contains("setNavSplit"),
        "the drag does not persist on every move: {moved}"
    );
    let ended = sidebar
        .split("function endDrag")
        .nth(1)
        .expect("the release handler")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        ended.contains("setNavSplit(navSplit)"),
        "the release is what persists: {ended}"
    );

    // A remembered split is re-clamped against the bar that exists now.
    assert!(
        sidebar.contains("<svelte:window onresize={reclamp} />"),
        "a smaller window re-clamps the split"
    );
    let reclamp = sidebar
        .split("function reclamp()")
        .nth(1)
        .expect("the reclamp")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        reclamp.contains("clampNavHeight(") && reclamp.contains("if (fixed !== navSplit)"),
        "the inequality is what stops the effect writing what it just read: {reclamp}"
    );

    // Both sides of the round trip, or the value is dropped in transit.
    let ui_state = read("desktop/ui/src/lib/ui-state.ts");
    assert!(
        ui_state.contains("navSplit: number | null") && ui_state.contains("navSplit: null"),
        "the preference lives with the others and defaults to the browser's layout"
    );
    let rust = read("desktop/src/uistate.rs");
    let field = rust
        .split("pub nav_split: Option<u32>")
        .next()
        .expect("the field's preamble");
    assert!(
        rust.contains("pub nav_split: Option<u32>"),
        "the host stores the split as an optional number"
    );
    assert!(
        field.rfind("#[serde(default)]").unwrap_or(0)
            > field.rfind("pub nav_collapsed").unwrap_or(0),
        "a profile saved before this field existed still loads, at the default split"
    );
}

// Scenario: Arrastrar la línea reparte el alto
// Scenario: El reparto se ajusta con el teclado
// Scenario: Plegada la barra, no hay reparto que hacer
// Scenario: Ninguna entrada se pierde al encoger la navegación
#[test]
fn the_divider_between_the_entries_and_the_tree_is_a_control() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");

    // A separator element of its own. The projects header could not be the
    // handle: it holds a title and a button, and interactive descendants inside
    // a focusable separator is invalid ARIA.
    assert!(
        sidebar.contains("class=\"split\"") && sidebar.contains("role=\"separator\""),
        "the divider is its own element with the separator role"
    );
    for attr in [
        "tabindex=\"0\"",
        "aria-orientation=\"horizontal\"",
        "aria-controls=\"nav-entries\"",
        "aria-label={$t(\"nav.split.label\")}",
        "aria-valuenow=",
        "aria-valuemin=",
        "aria-valuemax=",
    ] {
        assert!(
            sidebar.contains(attr),
            "the divider states its role, name, value and bounds: missing {attr}"
        );
    }

    // Pointer: capture, so the drag survives the pointer leaving a 12px strip.
    for hook in [
        "onpointerdown={startDrag}",
        "onpointermove={onDragMove}",
        "onpointerup={endDrag}",
        "onpointercancel={endDrag}",
        "setPointerCapture",
    ] {
        assert!(sidebar.contains(hook), "the drag is wired: missing {hook}");
    }
    assert!(
        sidebar.contains("clampNavHeight(") && sidebar.contains("stepNavHeight("),
        "both the drag and the keys go through the tested arithmetic"
    );

    // Keyboard: a step each way and the two ends by name. No global key minted.
    let keys = sidebar
        .split("function onSplitKeys")
        .nth(1)
        .expect("the key handler")
        .split("\n  }")
        .next()
        .expect("body");
    for key in ["ArrowUp", "ArrowDown", "Home", "End"] {
        assert!(keys.contains(key), "the divider answers to {key}: {keys}");
    }
    assert!(
        keys.contains("preventDefault()") && keys.contains("stopPropagation()"),
        "the divider's keys do not leak to the shell"
    );

    let styles = sidebar.split("<style>").nth(1).expect("styles");

    // Sizing the entries without letting them shrink is how they spill out of
    // the box: a column flex item defaults to min-height auto.
    let nav_rule = styles
        .split("\n  nav {")
        .nth(1)
        .expect("the nav rule")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        nav_rule.contains("min-height: 0") && nav_rule.contains("overflow-y: auto"),
        "shrinking the navigation scrolls it instead of losing entries: {nav_rule}"
    );

    // The hairline moved from the heading to the control that now divides.
    let section_rule = styles
        .split("\n  .section {")
        .nth(1)
        .expect("the section rule")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        !section_rule.contains("border-top"),
        "the projects heading no longer draws the divider: {section_rule}"
    );
    assert!(
        styles.contains(".split::after"),
        "the divider draws its own hairline"
    );

    // Folded, there are no two zones to split.
    assert!(
        styles.contains("aside.folded .split"),
        "the divider retires with the tree on the rail"
    );
    assert!(
        sidebar.contains("style:height={folded || navSplit === null ? null : navSplit + \"px\"}"),
        "no height is imposed on the rail, nor on a profile that never dragged"
    );
}

// Scenario: Plegar y desplegar desde la cabecera
// Scenario: Plegada no pierde alcance
// Scenario: El pliegue se recuerda, el primer arranque no
#[test]
fn the_sidebar_folds_to_a_rail_without_losing_a_single_entry() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    // A visible control in the header, not a hover reveal — the rule this
    // shell already applies to its per-row actions.
    assert!(
        sidebar.contains("class=\"fold ghost\"") && sidebar.contains("onclick={toggleFold}"),
        "the fold control is a real button in the header"
    );
    assert!(
        sidebar.contains("aria-expanded={!folded}"),
        "the control states the bar's state to assistive tech"
    );
    // One toggle, both ways, and the label says which way it goes.
    let toggle = sidebar
        .split("function toggleFold()")
        .nth(1)
        .expect("the fold action")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        toggle.contains("folded = !folded"),
        "the same control folds and unfolds: {toggle}"
    );
    assert!(
        sidebar.contains("nav.fold.expand") && sidebar.contains("nav.fold.collapse"),
        "the control names the direction it will move"
    );

    // Folded, the words go and the reach stays: every entry keeps an
    // accessible name, and the rules hide labels — never the entries.
    // Every navigation entry, not just the ones the loop emits: the settings
    // entry lives outside it, and a smoke run on the real binary found it
    // nameless while a check written against the loop alone passed.
    let entries: Vec<&str> = sidebar.split("class=\"item ghost\"").skip(1).collect();
    assert!(
        entries.len() >= 2,
        "the bar has entries inside the loop and outside it"
    );
    for entry in &entries {
        // Up to the icon it renders: the attribute region. Not `split('>')` —
        // an arrow function in an `onclick` carries a `>` of its own.
        let attrs = entry.split("<Icon").next().expect("the attribute region");
        assert!(
            attrs.contains("aria-label="),
            "a folded entry is an icon; every entry needs its own name: {attrs}"
        );
    }
    let styles = sidebar.split("<style>").nth(1).expect("styles").to_string();
    assert!(
        styles.contains("aside.folded .label") && styles.contains("display: none"),
        "folding hides the words"
    );
    // The tree goes as a whole rather than reflowing into a column of single
    // letters — a smoke run on the real binary showed exactly that. Nothing is
    // lost with it: its content stays reachable from entries that remain.
    assert!(
        styles.contains("aside.folded .tree"),
        "folding retires the project tree instead of squeezing it into 52px"
    );
    for kept in [".item", ".counter"] {
        assert!(
            !styles.contains(&format!("aside.folded {kept} {{\n    display: none")),
            "folding must not hide `{kept}`: the entries and the permission counter stay"
        );
    }

    // Remembered beside the theme, and a fresh profile starts unfolded.
    assert!(
        sidebar.contains("setNavCollapsed(folded)") && sidebar.contains("$uiState.navCollapsed"),
        "the choice is persisted and read back"
    );
    let ui_state = read("desktop/ui/src/lib/ui-state.ts");
    assert!(
        ui_state.contains("navCollapsed: boolean") && ui_state.contains("navCollapsed: false"),
        "the preference lives with the others and defaults to unfolded"
    );
    let rust = read("desktop/src/uistate.rs");
    assert!(
        rust.contains("pub nav_collapsed: bool") && rust.contains("#[serde(default)]"),
        "a profile saved before this field existed still loads, unfolded"
    );
}

// Scenario: Ninguna variable de estilo se usa sin existir
// Scenario: El conmutador de proyectos cubre lo que tapa
#[test]
fn every_style_variable_used_by_the_surface_is_one_that_exists() {
    // A `var(--typo)` does not fail and does not warn: it paints nothing. That
    // is how a floating panel shipped with no background at all, letting the
    // navigation tree read through it. Source assertions could not see it —
    // the node was there, correct, and invisible — so the guard is this lint:
    // every variable used must be one the design system defines.
    let root = repo_root().join("desktop/ui/src");
    let mut stack = vec![root];
    let mut defined: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut used: Vec<(String, usize, String)> = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let is_style = path
                .extension()
                .is_some_and(|e| e == "svelte" || e == "css");
            if !is_style {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let shown = path
                .strip_prefix(repo_root())
                .unwrap_or(&path)
                .display()
                .to_string();
            for (number, line) in text.lines().enumerate() {
                // Definitions: `--name:` anywhere (`:root`, a theme block, a
                // component's own scope).
                let mut rest = line;
                while let Some(at) = rest.find("--") {
                    let tail = &rest[at..];
                    let name: String = tail
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect();
                    let after = &tail[name.len()..];
                    if after.trim_start().starts_with(':') && !name.is_empty() {
                        defined.insert(name.clone());
                    }
                    rest = &tail[name.len().max(2)..];
                }
                // Uses: `var(--name`.
                let mut rest = line;
                while let Some(at) = rest.find("var(--") {
                    let tail = &rest[at + 4..];
                    let name: String = tail
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect();
                    if !name.is_empty() {
                        used.push((shown.clone(), number + 1, name.clone()));
                    }
                    rest = &tail[name.len().max(1)..];
                }
            }
        }
    }

    assert!(
        defined.len() > 20,
        "the lint found almost no definitions — it stopped seeing the design system"
    );
    let ghosts: Vec<String> = used
        .iter()
        .filter(|(_, _, name)| !defined.contains(name))
        .map(|(file, line, name)| format!("{file}:{line} uses `var({name})`, never defined"))
        .collect();
    assert!(
        ghosts.is_empty(),
        "style variables used without being defined paint NOTHING:\n{}",
        ghosts.join("\n")
    );

    // And the panel that shipped broken states its background from a token.
    let switcher = read("desktop/ui/src/lib/components/ProjectSwitcher.svelte");
    assert!(
        switcher.contains("background: var(--surface)"),
        "the project switcher paints an opaque surface"
    );
}

fn race_board() -> String {
    read("desktop/ui/src/lib/views/Review.svelte")
}

// Scenario: Calles lado a lado con procedencia visible
#[test]
fn each_lane_shows_its_agent_its_subscription_its_state_and_its_diff() {
    let board = race_board();
    // The provenance the contract now carries, all of it read from the lane and
    // none of it inferred from a neighbour.
    for field in [
        "lane.source",
        "lane.profile",
        "lane.level",
        "lane.sessionId",
        "lane.committed",
        "lane.baseRev",
    ] {
        assert!(board.contains(field), "the lane header reads {field}");
    }
    // Absence has exactly one face, and it is not a blank.
    assert!(
        board.contains("$t(\"race.unknown\")"),
        "a lane the daemon said nothing about is marked unknown, not left empty"
    );
    assert!(
        board.contains("lane.profile ?? $t(\"race.unknown\")"),
        "a lane with no subscription says so instead of borrowing one"
    );
    // State is signal plus word in all three rows, never colour alone.
    for pair in [
        ("race.turn.running", "▸"),
        ("race.commit.done", "◆"),
        ("race.checkpoint.have", "⌖"),
    ] {
        assert!(board.contains(pair.0), "{} is a word, not a colour", pair.0);
        assert!(board.contains(pair.1), "{} carries a signal too", pair.0);
    }
    // The turn state is the live session's, not a guess from the diff.
    assert!(
        board.contains("$allSessions.find((s) => s.sessionId === lane.sessionId)"),
        "a running turn is one the daemon still lists as live"
    );
    // Each lane renders its own diff against its own base, with the shared
    // parser: the board and the review cannot read the same text two ways.
    assert!(
        board.contains("fileSections(lane.diff)")
            && board.contains("short(lane.baseRev ?? baseRev)"),
        "the lane's diff is against the lane's base"
    );
    let messages = read("desktop/ui/src/lib/messages.ts");
    for key in [
        "race.turn.never",
        "race.commit.none",
        "race.checkpoint.none",
    ] {
        assert_eq!(
            messages.matches(&format!("\"{key}\"")).count(),
            2,
            "{key} is catalogued in both languages"
        );
    }
}

// Scenario: Carrera sin competidores, estado vacío honesto
#[test]
fn a_task_without_competitors_says_so_and_offers_the_way_in() {
    let board = race_board();
    let empty = board
        .split("{#if picked && competitors.length === 0}")
        .nth(1)
        .expect("the board tells an empty race apart from a loaded one");
    let empty = empty
        .split("{:else if picked}")
        .next()
        .expect("empty branch");
    assert!(
        empty.contains("race.empty.title") && empty.contains("race.empty.hint"),
        "the empty board says what is missing: {empty}"
    );
    assert!(
        empty.contains("race.empty.assign") && empty.contains("assignRace()"),
        "and offers the gesture that fixes it, not a dead end: {empty}"
    );
    // The gesture is the contract's own assignment, with the picked task
    // already filled in — the user names the agents and nothing else.
    assert!(
        board.contains("request(\"worktree/assign\", {")
            && board.contains("tasks: [{ change: picked.change, task: picked.task, agents }]"),
        "assigning goes through the daemon with the task already known"
    );
    assert!(
        board.contains("await refreshBoard(picked.change, picked.task)"),
        "and the board shows the lanes it just created"
    );
}

// Scenario: El tablero refleja el turno concluido
#[test]
fn the_board_follows_the_turns_it_started_and_says_which_ones_it_cannot() {
    let board = race_board();
    // The board listens to the session stream while a task is open.
    assert!(
        board.contains("onSessionEvent((message) =>"),
        "the board subscribes to the session stream"
    );
    let listener = board
        .split("onSessionEvent((message) =>")
        .nth(1)
        .expect("the board listens")
        .split("\n  );")
        .next()
        .expect("body");
    // A turn is followed when it belongs to this board: one it started, or one
    // already named by a lane. Anything else is somebody else's stream.
    assert!(
        listener.contains("ownTurns.has(message.sessionId)")
            && listener
                .contains("competitors.some((lane) => lane.sessionId === message.sessionId)"),
        "the board follows its own turns and its lanes', not every session: {listener}"
    );
    // A dispatch answers only when the turn is over, so the session id is
    // learned from the stream that runs meanwhile.
    assert!(
        listener.contains("message.event.type === \"session_started\"")
            && listener.contains("running"),
        "a session started while dispatching is adopted as this board's: {listener}"
    );
    // The end of a followed turn re-reads the board — no reload.
    assert!(
        listener.contains("message.event.type === \"session_ended\"")
            && listener.contains("refreshBoard(picked.change, picked.task)"),
        "a finished turn re-reads the lanes in place: {listener}"
    );
    // What it cannot follow, it declares — with the gesture that fixes it.
    assert!(
        board.contains("race.live.note") && board.contains("race.live.refresh"),
        "the board states the limit of what it can follow, and offers the refresh"
    );
    let es = read("desktop/ui/src/lib/messages.ts");
    assert!(
        es.contains("no llega hasta aquí") && es.contains("does not reach here"),
        "the limitation is spelled in both languages, not implied"
    );
}

// Scenario: Acción destructiva solo con confirmación explícita
#[test]
fn a_destructive_race_action_reaches_the_daemon_only_after_a_confirmation() {
    let board = race_board();
    // The race acts through the contract's own verbs, on the lane it acts on.
    for method in [
        "worktree/dispatch",
        "checkpoint/revert",
        "commit/task",
        "worktree/merge-file",
    ] {
        assert!(
            board.contains(&format!("openAction(\"{method}\"")),
            "the board offers `{method}` on the lane"
        );
    }
    // Which of them is destructive is the registry's word, not this view's
    // opinion: the same mark the palette obeys.
    assert!(
        board.contains("REGISTRY.find((entry) => entry.method === action?.method)?.dangerous"),
        "the danger of an action is read from the registry, never hardcoded here"
    );

    // Submitting a destructive action raises the dialog INSTEAD of sending.
    let submit = board
        .split("function submit()")
        .nth(1)
        .expect("the board has a send button")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        submit.contains("if (dangerous) {") && submit.contains("confirming = true;"),
        "a destructive action raises the confirmation: {submit}"
    );
    assert!(
        submit.contains("return;"),
        "and returns without sending anything: {submit}"
    );
    assert!(
        !submit.contains("request("),
        "the send path of a destructive action never reaches the daemon directly: {submit}"
    );

    // The dialog's two exits: confirm sends, cancel sends nothing at all — it
    // only lowers the dialog, leaving the composed action untouched.
    let dialog = board
        .split("{#if confirming && action}")
        .nth(1)
        .expect("the board raises a confirmation dialog")
        .split("{/if}")
        .next()
        .expect("dialog");
    assert!(
        dialog.contains("onConfirm={() => void perform()}"),
        "confirming is what performs the action: {dialog}"
    );
    let cancel = dialog
        .split("onCancel=")
        .nth(1)
        .expect("the dialog can be cancelled")
        .split('\n')
        .next()
        .expect("handler");
    assert!(
        cancel.contains("confirming = false"),
        "cancelling only lowers the dialog: {cancel}"
    );
    assert!(
        !cancel.contains("perform") && !cancel.contains("request("),
        "cancelling sends nothing at all: {cancel}"
    );

    // The parameters are the contract's own typed form, and the `confirm` a
    // destructive verb carries is left FALSE: the daemon's guard stays a
    // decision the human takes, not a default this surface ticked for them.
    assert!(
        board.contains("<MethodForm") && board.contains("method={action.method}"),
        "the action is composed in the generated typed form"
    );
    assert!(
        board.contains("confirm: false"),
        "the contract's confirm is never pre-ticked by the board"
    );
}

// Scenario: Inteligencia con servidor del usuario
// Scenario: Degradación honesta sin servidor
#[test]
fn code_intelligence_rides_the_users_own_server_or_says_it_cannot() {
    let lsp = read("desktop/ui/src/lib/editor/lsp.ts");
    for method in [
        "textDocument/completion",
        "textDocument/definition",
        "textDocument/formatting",
        "textDocument/references",
        "textDocument/rename",
    ] {
        assert!(
            lsp.contains(method),
            "the surface asks the server for {method}"
        );
    }
    let hub = read("desktop/src/lsp.rs");
    assert!(
        hub.contains("publishDiagnostics"),
        "diagnostics arrive from the server, not from a local guess"
    );
    // Nothing is bundled: the server is found on PATH or it is absent.
    assert!(
        hub.contains("which") || hub.contains("PATH") || hub.contains("probe"),
        "the server is the user's own, discovered on PATH"
    );
    // Without a server the actions are not offered and the state is stated.
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    assert!(
        editor.contains("{#if lspLabel}"),
        "the server-backed actions appear only with a server"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    assert!(
        catalog.contains("\"editor.lsp.none\""),
        "and the absence is stated in words"
    );
    // A rename lands through the daemon like any other edit.
    assert!(
        editor.contains("await saveFile(root, path, after, target, true)"),
        "the rename's edits are written by the daemon, file by file"
    );
}

// Scenario: Primer uso enseña el modelo
#[test]
fn the_first_run_teaches_the_shell_that_exists() {
    let onboarding = read("desktop/ui/src/lib/components/Onboarding.svelte");
    for key in [
        "onboarding.intro",
        "onboarding.views",
        "onboarding.palette",
        "onboarding.permissions",
    ] {
        assert!(onboarding.contains(key), "the first run teaches {key}");
    }
    let catalog = read("desktop/ui/src/lib/messages.ts");
    let views: Vec<&str> = catalog
        .lines()
        .filter(|line| line.contains("Cinco vistas") || line.contains("Five views"))
        .collect();
    assert_eq!(
        views.len(),
        2,
        "the lesson counts the views the shell actually keys, in both languages: {views:?}"
    );
    // What it teaches matches what the shell binds.
    let app = app();
    let keyed = app
        .split("const KEYED_VIEWS: ViewId[] = [")
        .nth(1)
        .expect("KEYED_VIEWS")
        .split(']')
        .next()
        .expect("list");
    assert_eq!(
        keyed.matches('"').count() / 2,
        5,
        "five views are keyed: {keyed}"
    );
    assert!(
        app.contains("event.key >= \"1\" && event.key <= \"5\""),
        "and the digits cover them"
    );
}

// Scenario: Flujo completo por teclado
#[test]
fn every_action_of_the_shell_is_reachable_from_the_keyboard() {
    let app = app();
    // Views, palette, launcher, tray, help and back: all keyed.
    for (key, what) in [
        ("event.key >= \"1\" && event.key <= \"5\"", "the views"),
        ("event.key === \":\"", "the palette"),
        ("=== \"n\"", "a new session"),
        ("event.key === \"a\"", "the tray"),
        ("event.key === \"?\"", "the help"),
        ("event.key === \"Escape\"", "going back"),
    ] {
        assert!(app.contains(key), "{what} has a key: {key}");
    }
    // Overlays trap Escape themselves, so the shell never swallows their keys.
    for overlay in [
        "desktop/ui/src/lib/components/Palette.svelte",
        "desktop/ui/src/lib/components/Onboarding.svelte",
        "desktop/ui/src/lib/components/ProjectSwitcher.svelte",
        "desktop/ui/src/lib/components/ConfirmDialog.svelte",
    ] {
        let source = read(overlay);
        assert!(source.contains("Escape"), "{overlay} closes on Escape");
        assert!(
            source.contains("stopPropagation") || source.contains("svelte:window"),
            "{overlay} owns its keys while it is open"
        );
    }
    // Focus is always visible, and no control is removed from the tab order.
    let css = read("desktop/ui/src/app.css");
    assert!(
        css.contains(":focus-visible") && css.contains("outline"),
        "focus is visible by rule"
    );
    let root = repo_root().join("desktop/ui/src");
    let mut stack = vec![root];
    let mut offenders = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|e| e == "svelte") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                // A container taking programmatic focus (a dialog panel) is
                // correct focus management; an INTERACTIVE control removed from
                // the tab order is not.
                //
                // The detector now sees BOTH forms. It used to match only the
                // literal `tabindex="-1"`, so a control taken out of the tab
                // order by an expression — `tabindex={x ? 0 : -1}` — passed
                // unseen. That was the blind spot, and closing it is the point
                // of this amendment (sidebar-ajustable-y-pestanas design D8).
                for (index, line) in text.lines().enumerate() {
                    let literal = line.contains("tabindex=\"-1\"");
                    let dynamic = line.contains("tabindex={") && line.contains("-1");
                    if !literal && !dynamic {
                        continue;
                    }
                    let head = text.lines().take(index + 1).collect::<Vec<_>>().join(
                        "
",
                    );
                    let tag_start = head.rfind('<').unwrap_or(0);
                    let tag = &head[tag_start..];
                    let interactive = tag.starts_with("<button")
                        || tag.starts_with("<a ")
                        || tag.starts_with("<input")
                        || tag.starts_with("<select")
                        || tag.starts_with("<textarea");
                    // The one exemption, and it is narrow: the WAI-ARIA tabs
                    // pattern REQUIRES a roving tabindex — exactly one tab in
                    // the tab order, the arrows moving between the rest. The
                    // strip is reached by Tab and traversed by arrow keys, so
                    // nothing is unreachable; `the_tab_strip_is_traversable_by_
                    // arrow_keys` is what pays for this exemption.
                    let roving_tab = tag.contains("role=\"tab\"");
                    if interactive && !roving_tab {
                        offenders.push(format!("{}:{}", path.display(), index + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "no control is taken out of the tab order: {offenders:?}"
    );
}

// ---- shell canvas (gui-acabado-y-cierre-sdd) ---------------------------------

// Scenario: La vista ocupa el alto disponible
#[test]
fn the_routed_view_owns_the_height_between_the_bars() {
    let app = app();
    // The central column is a flex column: bars at natural height, the routed
    // view takes the remainder. A fixed grid row template is forbidden here —
    // the daemon banner and the notices are conditional children, and a
    // template that counts rows stops filling the window when they are absent.
    assert!(
        app.contains("flex-direction: column"),
        "the central column stacks as a flex column"
    );
    assert!(
        !app.contains("grid-template-rows"),
        "no fixed row template: conditional bars would displace the view"
    );
    assert!(
        app.contains("flex: 1 1 0"),
        "the routed view claims the remaining height"
    );
}

// Scenario: Filas del árbol sin recorte
#[test]
fn tree_rows_never_compress_below_their_line() {
    let editor = read("desktop/ui/src/lib/views/Editor.svelte");
    // `.tree` and `.results` are scrolling flex columns; without an explicit
    // no-shrink their rows compress below the line height when the tree
    // outgrows the panel, clipping every label.
    let node_rule = editor
        .find(".node,")
        .and_then(|start| {
            editor[start..]
                .find('}')
                .map(|end| &editor[start..start + end])
        })
        .expect("the tree row rule exists");
    assert!(
        node_rule.contains("flex: 0 0 auto"),
        "tree and result rows never shrink: {node_rule}"
    );
}

// ---- pending gate in the change listing (sesion-esperando) -------------------

// Scenario: Gate pendiente descubrible en el listado
#[test]
fn the_change_table_shows_which_artifact_awaits_a_gate() {
    let project = read("desktop/ui/src/lib/views/Project.svelte");
    // The listing carries the gate as its own column, so a change awaiting a
    // human decision is legible without opening it.
    assert!(
        project.contains("project.col.gate"),
        "the changes table has a gate column"
    );
    assert!(
        project.contains("change.gatePending"),
        "the row reads the contract's gatePending"
    );
    assert!(
        project.contains("project.gateOn") && project.contains("change.gateArtifact"),
        "a pending gate names the artifact it is about"
    );
    // Absence is stated, never invented.
    assert!(
        project.contains(":else"),
        "a change with no gate renders the empty marker, not a blank cell"
    );
    // Both languages carry the new keys (the i18n lint enforces parity; this
    // asserts the keys exist at all).
    let messages = read("desktop/ui/src/lib/messages.ts");
    for key in ["project.col.gate", "project.gateOn", "project.gateWaiting"] {
        assert_eq!(
            messages.matches(&format!("\"{key}\"")).count(),
            2,
            "`{key}` is defined in both languages"
        );
    }
}

// ---- icon-and-label controls (pulido-pre-anuncio) ----------------------------

// Scenario: Icono y etiqueta en una línea
#[test]
fn the_global_button_skin_lays_icon_and_label_on_one_line() {
    let css = read("desktop/ui/src/app.css");
    // The bare element rule, not `.ghost`/`.primary` variants: the alignment
    // must come from the skin itself, so a plain `<button>` with an icon and a
    // label renders correct without any local re-declaration (design D1 — the
    // per-component repetition WAS the root cause).
    let rule = css
        .split("\nbutton {")
        .nth(1)
        .expect("the shared button skin has a bare element rule")
        .split('}')
        .next()
        .expect("rule body");
    for declaration in [
        "display: inline-flex",
        "align-items: center",
        "gap: var(--sp-2)",
    ] {
        assert!(
            rule.contains(declaration),
            "the global button rule declares `{declaration}`: {rule}"
        );
    }
    // The root cause pair: Icon's svg is a block box, which is exactly why a
    // button without a flex display breaks the line. If Icon ever stops doing
    // this, the global rule still holds; if the rule ever goes, this test says
    // why it existed.
    let icon = read("desktop/ui/src/lib/components/Icon.svelte");
    assert!(
        icon.contains("display: block"),
        "Icon's svg is a block box, which is what the skin's flex display absorbs"
    );
}

// Scenario: La acción de flota sin falso contador
#[test]
fn the_fleet_action_carries_no_embedded_shortcut_hint() {
    let messages = read("desktop/ui/src/lib/messages.ts");
    let labels: Vec<&str> = messages
        .lines()
        .filter(|line| line.contains("\"sessions.empty.fleet\""))
        .collect();
    assert_eq!(labels.len(), 2, "the label exists in both languages");
    for label in labels {
        // In an empty state that says "no sessions", a parenthesized number
        // next to "fleet" reads as a live agent count — which it is not. The
        // shortcut hint does not travel embedded in a catalog string.
        assert!(
            !label.contains('(') && !label.contains(')'),
            "the fleet action label suggests no count: {label}"
        );
    }
}

// Scenario: El atajo conserva su afordancia
#[test]
fn the_fleet_shortcut_keeps_its_home_in_the_sidebar() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    // The shortcut's dedicated affordance: the sidebar renders a `kbd` per
    // keyed navigation item, and the Fleet item is keyed 4.
    assert!(
        sidebar.contains("<kbd>{item.key}</kbd>"),
        "the sidebar shows each item's shortcut as kbd"
    );
    assert!(
        sidebar.contains("{ id: \"fleet\", icon: \"fleet\", key: \"4\" }"),
        "the Fleet item keeps its key"
    );
}

// Scenario: Par de acciones del estado vacío a altura pareja
#[test]
fn empty_state_actions_keep_their_natural_height() {
    let empty = read("desktop/ui/src/lib/components/EmptyState.svelte");
    let rule = empty
        .split(".actions {")
        .nth(1)
        .expect("the empty state has an actions row")
        .split('}')
        .next()
        .expect("rule body");
    // The default `stretch` was what equalized a healthy button to a broken
    // one — and what made wrapped lines diverge, each stretching on its own.
    assert!(
        rule.contains("align-items: center"),
        "each action keeps its natural height instead of stretching: {rule}"
    );
    assert!(
        rule.contains("flex-wrap: wrap"),
        "the row wraps, so the alignment rule must cover the wrapped case: {rule}"
    );
}

// ---- the conversational reading ----------------------------------------------

// Scenario: Un turno se pliega de prompt a cierre
// Scenario: El pensamiento no se mezcla con la respuesta
// Scenario: Evento no clasificable cae a la vista, no al olvido
#[test]
fn the_conversation_is_a_fold_over_the_log_that_drops_nothing() {
    // The fold is a pure module precisely so its grammar can be executed rather
    // than inspected: the cases below run in CI (`npm test --prefix
    // desktop/ui`), and this test links them to the scenarios they prove.
    let tests = read("desktop/ui/tests/conversation.test.ts");
    for case in [
        "Un turno se pliega de prompt a cierre",
        "El pensamiento no se mezcla con la respuesta",
        "Evento no clasificable cae a la vista, no al olvido",
        "Las tres formas de actualizacion del agente se leen como prosa",
    ] {
        assert!(
            tests.contains(case),
            "the fold has an executed case for: {case}"
        );
    }
    // And the drill-in renders that fold rather than a second source of truth.
    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    assert!(
        detail.contains("fold(") && detail.contains("from \"../conversation\""),
        "the conversation view reads the fold, not its own parse of the events"
    );
    assert!(
        detail.contains("reading === \"conversation\"") && detail.contains("conv.reading.log"),
        "the operator log stays one switch away"
    );
}

// Scenario: El conmutador no pierde nada
// Scenario: Conmutar entre conversación y log de operador
#[test]
fn switching_readings_keeps_every_event_and_the_reader_s_place() {
    // The invariant is a property of the fold, and it is executed: each item
    // carries the ids of the events it accounts for, and the union of those is
    // exactly the events handed in — so the two readings cannot show different
    // sets, whatever the grammar does with them.
    let tests = read("desktop/ui/tests/conversation.test.ts");
    for case in [
        "El conmutador no pierde nada",
        "Conmutar entre conversación y log de operador",
    ] {
        assert!(
            tests.contains(case),
            "the switch invariant has an executed case for: {case}"
        );
    }
    let fold = read("desktop/ui/src/lib/conversation.ts");
    assert!(
        fold.contains("eventIds"),
        "every item declares which events it accounts for"
    );

    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    // The count is on screen in both readings, so the user can check it rather
    // than take the design note's word for it.
    assert!(
        detail.contains("conv.events") && detail.contains("events.length"),
        "the event count is rendered, and it counts received events"
    );
    // The injected "cut" marker is NOT a received event and must not inflate it.
    assert!(
        detail.contains("lines.filter((line) => !line.cut)"),
        "the connection-cut marker is excluded from the count and from the fold"
    );
    // Switching is a change of lens, not a reload: the reader's offset survives.
    let switcher = detail
        .split("async function switchReading")
        .nth(1)
        .expect("the switch is a function, not an inline toggle");
    assert!(
        switcher.contains("scrollTop") && switcher.contains("wasAtBottom"),
        "the switch restores the offset, and following the tail keeps following it"
    );
}

// Scenario: Decidir un permiso sin salir de la conversación
// Scenario: Tarjeta ya resuelta no es accionable
// Scenario: La bandeja sigue siendo la vista completa
#[test]
fn an_inline_permission_card_decides_the_same_queue_or_decides_nothing() {
    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    // The same method the tray calls: another view of one queue, not a second.
    assert!(
        detail.contains("\"permission/decide\"")
            && detail.contains("requestId: waitingOn.requestId"),
        "the card resolves the proxy's own request, by its id"
    );
    // The request id lives in the queue, not in the log event, so a card is
    // actionable only WHILE the queue still holds a request for this session.
    assert!(
        detail.contains("if (!waitingOn || waitingOn.expired) return null;"),
        "an expired or absent request leaves no live card"
    );
    assert!(
        detail.contains("{:else}") && detail.contains("conv.permissionStale"),
        "a card that no longer decides says so instead of keeping its buttons"
    );
    // And the tray keeps listing it: the card never removes anything from it.
    assert!(
        detail.contains("conv.alsoInTray"),
        "the live card names the tray as the complete view"
    );
    let tray = read("desktop/ui/src/lib/views/Permissions.svelte");
    assert!(
        tray.contains("refreshPending") && tray.contains("$pending"),
        "the tray still renders the whole queue on its own"
    );
}

#[test]
fn every_declared_event_type_has_a_glyph_and_a_tone() {
    // The map covered 19 of the contract's 20 types, and the gap was invisible
    // because an unmapped type falls back to the neutral style — correct
    // behaviour that also hides the omission. Derived from the contract rather
    // than counted by hand, so the next variant cannot slip in unstyled.
    let proto = read("proto/meltemi-proto/src/lib.rs");
    let body = proto
        .split("pub enum SessionEventKind {")
        .nth(1)
        .expect("the contract declares its session events")
        .split("\n}")
        .next()
        .expect("the enum closes");
    let variants: Vec<String> = body
        .lines()
        // `SessionCancelled {},` carries no field and closes on its own line.
        .filter_map(|line| {
            let name = line.strip_prefix("    ")?.trim_end_matches(',');
            name.strip_suffix(" {").or_else(|| name.strip_suffix(" {}"))
        })
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .map(|name| {
            let mut snake = String::new();
            for (index, character) in name.char_indices() {
                if character.is_uppercase() && index > 0 {
                    snake.push('_');
                }
                snake.extend(character.to_lowercase());
            }
            snake
        })
        .collect();
    assert!(variants.len() >= 20, "the enum was parsed: {variants:?}");

    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    let style = detail
        .split("const EVENT_STYLE")
        .nth(1)
        .expect("the transcript styles its events")
        .split("};")
        .next()
        .expect("the map closes");
    for variant in &variants {
        assert!(
            style.contains(&format!("{variant}: {{")),
            "event type `{variant}` has no glyph and tone"
        );
    }
}

// ---- projects in the navigation ----------------------------------------------

// Scenario: La sección de proyectos está siempre visible
#[test]
fn the_projects_section_is_permanent_chrome() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    // A titled section in the sidebar itself, not something a modal reveals.
    assert!(
        sidebar.contains("class=\"sectionTitle\"") && sidebar.contains("projects.title"),
        "the projects section is named where it lives"
    );
    assert!(
        sidebar.contains("role=\"tree\""),
        "and it renders the tree of known projects"
    );
    // With no projects it says so rather than rendering an unexplained gap.
    assert!(
        sidebar.contains("projects.empty"),
        "an empty registry is stated, not left blank"
    );
    // The shell mounts it once, outside the routed view, so no view can hide it.
    let app = app();
    assert!(
        !main_region(&app).contains("<Sidebar"),
        "the section cannot be displaced by whatever view is open"
    );
}

// Scenario: Acción rápida por proyecto lleva al compositor
#[test]
fn each_project_node_can_start_work_in_it() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    assert!(
        sidebar.contains("onNewSessionIn(group.root)"),
        "each project node offers to start a session in it"
    );
    // Named for the screen reader as well as the eye, and always rendered: a
    // control revealed on hover is a control the keyboard cannot reach.
    assert!(
        sidebar.contains("nav.tree.newSession"),
        "the quick action says which project it starts work in"
    );
    assert!(
        !sidebar.contains(":hover .quick"),
        "the quick action must not be hover-revealed"
    );
    // Switching the scope stays a separate gesture on the same node.
    assert!(
        sidebar.contains("switchProject(group.root)"),
        "the node still switches the active project"
    );
    // And the action lands on the composer with that project already chosen.
    let app = app();
    assert!(
        app.contains("onNewSessionIn={(root) => openComposer(\"free\", root)}"),
        "the quick action routes to the composer, naming the project"
    );
    let home = read("desktop/ui/src/lib/views/Home.svelte");
    assert!(
        home.contains("$state(untrack(() => initialProject))"),
        "the composer opens on the project it was handed"
    );
}

// Scenario: Abrir una carpeta la registra antes de lanzar
#[test]
fn opening_a_folder_registers_it_by_contract_before_anything_runs() {
    let stores = read("desktop/ui/src/lib/stores.ts");
    let pick = stores
        .split("export async function pickAndRegisterProject")
        .nth(1)
        .expect("the surface has one place that opens and registers")
        .split("\n}")
        .next()
        .expect("body");
    assert!(
        pick.contains("pick_project_folder") && pick.contains("registerProject(picked)"),
        "the dialog's answer goes straight to the registry: {pick}"
    );
    assert!(
        pick.contains("if (!picked) return null;"),
        "a dismissed dialog sends nothing to the daemon: {pick}"
    );
    let register = stores
        .split("export async function registerProject")
        .nth(1)
        .expect("registerProject exists")
        .split("\n}")
        .next()
        .expect("body");
    assert!(
        register.contains("\"project/register\"") && register.contains("result.project.root"),
        "it registers by the contract method and keeps the CANONICAL root: {register}"
    );

    // Both doors the requirement names: the nav and the composer's chip.
    for (file, surface) in [
        ("desktop/ui/src/lib/components/Sidebar.svelte", "the nav"),
        ("desktop/ui/src/lib/views/Home.svelte", "the composer"),
    ] {
        let source = read(file);
        assert!(
            source.contains("projects.open") && source.contains("pickAndRegisterProject()"),
            "{surface} offers to open a folder through that one path"
        );
    }
}

// Scenario: Proyecto ausente en disco marcado en el árbol
#[test]
fn forgetting_says_what_it_does_not_do_and_an_absent_root_keeps_its_node() {
    let sidebar = read("desktop/ui/src/lib/components/Sidebar.svelte");
    // Forget is offered per node and confirmed with text about a LISTING.
    assert!(
        sidebar.contains("forgetProject(root)") && sidebar.contains("projects.forget.warning"),
        "forgetting goes through the contract method, behind its own confirmation"
    );
    // An absent root keeps its node, its mark and its next step.
    assert!(
        sidebar.contains("{#if !group.exists}") && sidebar.contains("projects.absent.remedy"),
        "a vanished root is marked and given its remedy instead of disappearing"
    );
    // The words matter more than the wiring here: the warning must not read as
    // a deletion, in either language.
    let catalog = read("desktop/ui/src/lib/messages.ts");
    for promise in [
        "No borra nada del disco",
        "Nothing on disk is deleted",
        "Reaparecerá en cuanto se vuelva a usar",
        "It comes back the moment it is used",
    ] {
        assert!(
            catalog.contains(promise),
            "the confirmation states: {promise}"
        );
    }
}

// ---- the conversational home -------------------------------------------------

// Scenario: Llegar y escribir
// Scenario: El método está a un gesto en el mismo compositor
// Scenario: El compositor no inventa contrato
#[test]
fn the_composer_arrives_focused_with_its_context_and_names_its_method() {
    let home = read("desktop/ui/src/lib/views/Home.svelte");
    // The three chips the scenario names, inside the composer.
    for chip in ["nav.project", "session.new.agent", "session.new.mode"] {
        assert!(home.contains(chip), "the composer shows its {chip} chip");
    }
    assert!(
        home.contains("initialMode = \"free\""),
        "and free is what is already selected"
    );
    // The caret is in the field on arrival: the user arrived to write.
    assert!(
        home.contains("box?.focus();"),
        "the composer takes the focus when it appears"
    );
    // The method each mode dispatches is declared before sending, not after.
    assert!(
        home.contains("<code>{METHOD[mode]}</code>"),
        "the method is on screen beside the send button"
    );

    // And it is never a method the parity matrix does not carry: read the map
    // out of the source rather than trusting a list written here.
    let map = home
        .split("const METHOD: Record<Mode, string> = {")
        .nth(1)
        .expect("the composer declares one method per mode")
        .split("};")
        .next()
        .expect("the map closes");
    let matrix = read("docs/paridad-nucleo.md");
    let mut modes = 0;
    for line in map.lines() {
        let Some((_, value)) = line.split_once(": \"") else {
            continue;
        };
        let method = value.split('"').next().expect("a method name");
        modes += 1;
        assert!(
            matrix.contains(&format!("`{method}`")),
            "the composer dispatches `{method}`, which the parity matrix does not carry"
        );
    }
    assert_eq!(modes, 3, "free, propose and explore: {map}");
}

// Scenario: Enviar navega hacia adentro
// Scenario: Los puntos de entrada vigentes rutean al compositor
#[test]
fn sending_walks_in_and_every_entry_point_arrives_at_the_composer() {
    let home = read("desktop/ui/src/lib/views/Home.svelte");
    let send = home
        .split("async function send()")
        .nth(1)
        .expect("the composer sends")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        send.contains("message.event.type !== \"session_started\"")
            && send.contains("onOpenSession(message.sessionId)"),
        "the surface navigates on the session's own arrival event: {send}"
    );
    // And it does NOT settle for announcing a launch when it walked in.
    assert!(
        send.contains("if (!entered) {"),
        "the \"launched\" notice is only for the case where nothing was entered"
    );

    let app = app();
    // One door, and every entry point goes through it.
    assert_eq!(
        app.matches("openComposer(").count(),
        6,
        "the definition plus its five call sites: the shortcut, the chrome's \
         primary action, the empty state of Sessions, the Project view's \
         Propose, and a project node of the nav"
    );
    for entry in [
        "openComposer();\n      return;",              // the shortcut
        "onNewSession={() => openComposer()}",         // the chrome
        "onPropose={() => openComposer(\"propose\")}", // the Project view
        "onNewSessionIn={(root) => openComposer(\"free\", root)}", // a nav node
    ] {
        assert!(app.contains(entry), "an entry point is missing: {entry}");
    }
    // And no modal launcher survives to compete with it.
    assert!(
        !app.contains("<NewSession"),
        "the modal launcher is retired, not merely bypassed"
    );
}

// Scenario: Instrucción encolada se declara encolada
// Scenario: Sesión terminada ofrece reanudar, no enviar
// Scenario: Enviar no interrumpe
#[test]
fn the_conversation_composer_states_what_the_daemon_answered() {
    let detail = read("desktop/ui/src/lib/views/SessionDetail.svelte");
    // Queued is queued, with the position the daemon gave.
    assert!(
        detail.contains("result.disposition === \"queued\"")
            && detail.contains("queuePosition: result.queuePosition"),
        "a queued instruction is reported as queued, with its place in the queue"
    );
    assert!(
        detail.contains("conv.queued"),
        "and it is stated in the composer, not implied by silence"
    );
    // A terminated but resumable session offers Resume in place of Send.
    assert!(
        detail.contains("!LIVE.includes(session.state) && session.resumable")
            && detail.contains("? $t(\"sessions.resume\")"),
        "the button says resume when resuming is what would happen"
    );
    assert!(
        detail.contains("if (refused) return false;"),
        "a session that refused direction stops being offered a send"
    );
    // Sending never cancels: cancelling is its own control, behind confirmation.
    let send = detail
        .split("async function direct()")
        .nth(1)
        .expect("the composer directs")
        .split("\n  }")
        .next()
        .expect("body");
    assert!(
        !send.contains("session/cancel"),
        "sending must not touch the running turn: {send}"
    );
    assert!(
        detail.contains("confirmCancel = true") && detail.contains("<ConfirmDialog"),
        "cancelling stays a separate, explicit control"
    );
}
