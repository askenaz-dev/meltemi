// SPDX-License-Identifier: Apache-2.0

//! Verification of the `analitica-consumo-local` scenarios the aggregator's own
//! unit tests do not cover: the ones about the query surface, the panel's
//! wiring, and the two promises that are proven by *absence* — nothing is
//! estimated, and nothing leaves the machine.

use std::path::{Path, PathBuf};

use meltemi_proto::{AnalyticsUsageParams, UsageGranularity};

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

// Scenario: Agregación por rango y granularidad
#[test]
fn a_range_with_monthly_grain_yields_one_cell_per_period_and_matching_totals() {
    use meltemi_proto::TurnStatus;
    use meltemid::session_index::{self, SessionRecord};

    let data = std::env::temp_dir().join(format!("mel-scen-an-range-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(&data).expect("temp dir");

    let record = |id: &str, started: &str, ended: &str| SessionRecord {
        mode: None,
        session_id: id.into(),
        agent_command: vec!["mock-agent".into()],
        project_root: "/repo".into(),
        level: 1,
        started_at: started.into(),
        ended_at: Some(ended.into()),
        final_status: Some(TurnStatus::Completed),
        agent_session_id: None,
        supports_load: false,
        resumed_from: None,
        agent_id: None,
        profile: None,
        source: None,
        title: None,
    };
    for r in [
        record("june", "2026-06-15T10:00:00Z", "2026-06-15T10:01:00Z"),
        record("july-a", "2026-07-05T10:00:00Z", "2026-07-05T10:02:00Z"),
        record("july-b", "2026-07-25T10:00:00Z", "2026-07-25T10:03:00Z"),
        record("august", "2026-08-02T10:00:00Z", "2026-08-02T10:01:00Z"),
    ] {
        session_index::append(&data, "k", &r).expect("index record");
    }

    let result = meltemid::analytics::aggregate(
        &data,
        &AnalyticsUsageParams {
            since: Some("2026-07-01T00:00:00Z".into()),
            until: Some("2026-08-01T00:00:00Z".into()),
            granularity: Some(UsageGranularity::Month),
            ..Default::default()
        },
    )
    .expect("aggregate");

    // One cell for the one period inside the range, holding both July sessions.
    assert_eq!(result.cells.len(), 1, "got: {result:#?}");
    assert_eq!(result.cells[0].period, "2026-07");
    assert_eq!(
        result.cells[0].activity.sessions, 2,
        "June and August are out"
    );
    // The totals are the sum of the returned cells, not of everything on disk.
    assert_eq!(result.totals.activity.sessions, 2);
    assert_eq!(result.totals.activity.active_seconds, 120 + 180);
    assert_eq!(result.totals.cells, 1);

    // The same records at a daily grain split into their own periods.
    let daily = meltemid::analytics::aggregate(
        &data,
        &AnalyticsUsageParams {
            since: Some("2026-07-01T00:00:00Z".into()),
            until: Some("2026-08-01T00:00:00Z".into()),
            granularity: Some(UsageGranularity::Day),
            ..Default::default()
        },
    )
    .expect("aggregate");
    assert_eq!(daily.cells.len(), 2, "one cell per day: {daily:#?}");
    assert_eq!(
        daily.totals.activity.sessions, 2,
        "the totals still match the cells"
    );
    let _ = std::fs::remove_dir_all(&data);
}

// Scenario: Ninguna estimación ni consulta a la cuenta
#[test]
fn the_aggregator_never_estimates_and_never_reaches_the_provider_account() {
    let source = read("core/meltemid/src/analytics.rs");
    // Tokens come from ONE place: the usage event. If the aggregator ever grew a
    // second source, this assertion would have to be weakened — which is the
    // point of making it explicit.
    let sources: Vec<&str> = source
        .lines()
        .filter(|line| line.contains("facts.measured = true"))
        .collect();
    assert_eq!(
        sources.len(),
        1,
        "measured tokens must have exactly one origin: {sources:?}"
    );
    assert!(
        source.contains("SessionEventKind::UsageReported"),
        "that origin is the usage event of the session log"
    );
    // Code only: a doc comment that says "nothing is estimated" is the promise,
    // not a violation of it. Strip comments before looking for the forbidden.
    // Production code only: the module's own tests build fixtures in a temp dir.
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production half");
    let code: String = production
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && !t.starts_with("*")
        })
        .collect::<Vec<_>>()
        .join("\n");
    // The disclosure key literally spells the promise ("nothing is estimated");
    // drop it before scanning for the forbidden words.
    // The disclosure keys spell the promises themselves ("nothing is estimated",
    // "no quota, balance or billing"); drop them before scanning, or the promise
    // would read as its own violation.
    let code_lower = code
        .replace("NothingIsEstimated", "")
        .replace("nothing-is-estimated", "")
        .replace("NoQuotaBalanceOrBilling", "")
        .replace("no-quota-balance-or-billing", "")
        .to_lowercase();
    for forbidden in [
        "estimate",
        "approx",
        "chars() / 4",
        "len() / 4",
        "quota",
        "balance",
        "billing",
    ] {
        assert!(
            !code_lower.contains(forbidden),
            "the aggregator's code must not contain `{forbidden}`"
        );
    }
    // The only place the word appears is the disclosure key, which promises the
    // opposite — and it is a key, not a computation.
    assert!(
        source.contains("NothingIsEstimated") && source.contains("NoQuotaBalanceOrBilling"),
        "the disclosure states that nothing is estimated and that the account is not visible"
    );
    // And it opens no connection: no HTTP client, no socket, no process.
    for forbidden in [
        "reqwest",
        "http",
        "TcpStream",
        "Command::new",
        "std::process",
    ] {
        assert!(
            !code.contains(forbidden),
            "the aggregator must not reach outside the data directory (`{forbidden}`)"
        );
    }
}

// Scenario: Ningún dato sale de la máquina
#[test]
fn the_aggregator_only_reads_local_records_and_writes_nothing() {
    let source = read("core/meltemid/src/analytics.rs");
    // Reads: the session index, the session logs, the project's edit log.
    assert!(
        source.contains("session_index::records_for_project")
            && source.contains("facts_of_log")
            && source.contains(".meltemi\")\n        .join(\"edits\")"),
        "the three local record sources are the only inputs"
    );
    // Writes: none. No file is created, appended to or removed.
    for forbidden in [
        "std::fs::write",
        "std::fs::OpenOptions",
        "create_dir_all",
        "std::fs::remove",
        "File::create",
    ] {
        let occurrences = source.matches(forbidden).count();
        // The test module may build fixtures; production code must not write.
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production half");
        assert!(
            !production.contains(forbidden),
            "the aggregator writes nothing outside the tests (`{forbidden}`, {occurrences} total)"
        );
    }
}

// Scenario: Método con casa en las tres superficies
#[test]
fn the_accounting_method_is_registered_in_all_three_surfaces() {
    let matrix = read("docs/paridad-nucleo.md");
    assert!(
        matrix.contains("| `analytics/usage` | `usage` | `usage` | registry + Usage view |"),
        "the parity matrix carries the row"
    );
    let palette = read("tui/src/shell/palette.rs");
    assert!(
        palette.contains("methods: &[m::ANALYTICS_USAGE]"),
        "the TUI palette gives it a home"
    );
    let registry = read("desktop/ui/src/lib/registry.ts");
    assert!(
        registry.contains("R(\"analytics/usage\""),
        "the GUI registry gives it a home"
    );
    let reference = read("docs/referencia-cli.md");
    assert!(
        reference.contains("usage [day|week|month|total]"),
        "the generated CLI reference documents the scriptable verb"
    );
    // The blocking gate that keeps it that way.
    let gate = read("tui/tests/parity.rs");
    assert!(
        gate.contains("contract methods without a GUI registry home"),
        "the parity gate fails naming the missing surface"
    );
}

// Scenario: Declaración visible con las cifras
#[test]
fn the_disclosure_is_rendered_beside_the_numbers_without_an_interaction() {
    let panel = read("desktop/ui/src/lib/views/Usage.svelte");
    // The disclosure is a plain section of the same view as the table, not a
    // dialog, a tooltip or a collapsed details element.
    let disclosure = panel
        .split("<section class=\"disclosure\"")
        .nth(1)
        .expect("the panel renders a disclosure section");
    let body = disclosure.split("</section>").next().expect("section body");
    assert!(
        body.contains("{#each result.disclosure as key (key)}"),
        "every key the daemon returned is rendered: {body}"
    );
    assert!(
        body.contains("$t((\"usage.disclosure.\" + key) as MessageKey)"),
        "the text comes from the surface catalog, not from the daemon"
    );
    for forbidden in ["<details", "<summary", "aria-expanded", "onclick"] {
        assert!(
            !disclosure.contains(forbidden),
            "the disclosure must not hide behind an interaction ({forbidden})"
        );
    }
    // It is rendered whenever there is a result — including the empty answer.
    let condition_at = panel
        .find(
            "{#if result}
    <section class=\"disclosure\"",
        )
        .or_else(|| panel.find("{#if result}"))
        .expect("the disclosure is guarded only by having an answer");
    let table_at = panel.find("<table class=\"dense\">").expect("the table");
    assert!(
        condition_at > table_at,
        "the disclosure sits with the figures, after the table, in the same view"
    );
    // Both catalogs carry a line for every key the daemon can emit.
    let catalog = read("desktop/ui/src/lib/messages.ts");
    for key in [
        "activity-from-local-records",
        "tokens-only-when-official-output-reports",
        "no-quota-balance-or-billing",
        "nothing-is-estimated",
        "nothing-leaves-this-machine",
    ] {
        assert_eq!(
            catalog
                .matches(&format!("\"usage.disclosure.{key}\""))
                .count(),
            2,
            "the key {key} must be translated in ES and EN"
        );
    }
    // And the scriptable surface prints it too, so the numbers never travel bare.
    let cli = read("tui/src/run.rs");
    assert!(
        cli.contains("for key in &result.disclosure"),
        "the CLI renders the disclosure with the numbers"
    );
}

// Scenario: Panel denso con numerales tabulares
#[test]
fn the_panel_is_dense_and_uses_tabular_numerals() {
    let panel = read("desktop/ui/src/lib/views/Usage.svelte");
    assert!(
        panel.contains("font-variant-numeric: tabular-nums"),
        "figures keep their columns when they refresh"
    );
    assert!(
        panel.contains("class=\"dense\""),
        "the cells use the design system's dense table"
    );
    for token in ["var(--panel-pad)", "var(--radius-panel)", "var(--hair)"] {
        assert!(
            panel.contains(token),
            "the panel takes its density and shape from the tokens ({token})"
        );
    }
    // Row height and cell padding come from the shared dense-table rules, so the
    // panel cannot drift from the 32/8/16 density on its own.
    let css = read("desktop/ui/src/app.css");
    let cell_rule = css
        .split("table.dense td {")
        .nth(1)
        .expect("the client declares the dense cell rule")
        .split('}')
        .next()
        .expect("rule body");
    assert!(
        cell_rule.contains("height: var(--row-h)") && cell_rule.contains("var(--cell-pad)"),
        "the dense cell takes its height and padding from the tokens: {cell_rule}"
    );
}

// Scenario: El refresco no mueve nada bajo el cursor
#[test]
fn refreshing_the_panel_animates_nothing_and_moves_no_tray() {
    let panel = read("desktop/ui/src/lib/views/Usage.svelte");
    for forbidden in [
        "transition:",
        "animate:",
        "@keyframes",
        "animation:",
        "scrollIntoView",
    ] {
        assert!(
            !panel.contains(forbidden),
            "the panel must not animate or scroll on refresh ({forbidden})"
        );
    }
    // The permission tray and the signal banners live in the shell, above the
    // routed view: the panel cannot displace them.
    let app = read("desktop/ui/src/App.svelte");
    let banner_at = app.find("$conn.state === \"unreachable\"").expect("banner");
    let notices_at = app.find("<Notices />").expect("notices");
    let main_at = app.find("<main>").expect("main");
    assert!(
        banner_at < main_at && notices_at < main_at,
        "the banner and the notices are rendered before the routed view, so a view cannot move them"
    );
}

// Scenario: Panel operable por teclado
#[test]
fn the_panel_is_fully_operable_from_the_keyboard() {
    let panel = read("desktop/ui/src/lib/views/Usage.svelte");
    // Every control is a native focusable element: buttons for the period and
    // scope, selects for the filters. No div pretending to be a control.
    assert!(
        panel.contains("<button") && panel.contains("<select bind:value={agentFilter}"),
        "period, scope and filters are native controls"
    );
    assert!(
        !panel.contains("<div role=\"button\"") && !panel.contains("tabindex=\"-1\""),
        "no control is unreachable by keyboard"
    );
    assert!(
        panel.contains("aria-pressed={granularity === period}"),
        "a toggled control announces its state"
    );
    // Focus visibility is a global rule of the client's stylesheet.
    let css = read("desktop/ui/src/app.css");
    assert!(
        css.contains(":focus-visible"),
        "the client declares a visible focus indicator"
    );
    // And the view is reachable by its own digit, not only by pointer.
    let app = read("desktop/ui/src/App.svelte");
    assert!(
        app.contains("\"analytics\"") && app.contains("event.key >= \"1\" && event.key <= \"5\""),
        "the panel has a navigation key"
    );
}

// Scenario: Sesión ACP no fabrica evento de uso
#[test]
fn an_acp_session_never_writes_a_usage_event() {
    // A usage event can only enter a log through the level-3 mapping seam. The
    // ACP driver — the code that runs a level 1/2 session — never mentions it,
    // so an ACP session cannot fabricate one.
    let acp = read("core/meltemid/src/acp.rs");
    assert!(
        !acp.contains("UsageReported"),
        "the ACP session path must not build a usage event"
    );

    // Nothing appends a usage event to a session log outside the capture seam.
    let root = repo_root();
    let mut appenders: Vec<String> = Vec::new();
    let mut stack = vec![
        root.join("core"),
        root.join("tui"),
        root.join("desktop/src"),
    ];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if !matches!(name.to_string_lossy().as_ref(), "target" | "node_modules") {
                    stack.push(path);
                }
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                // `append(<usage event>)` is the only way one reaches a log.
                for (index, line) in text.lines().enumerate() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("//") {
                        continue;
                    }
                    if line.contains("append(") && line.contains("UsageReported") {
                        let relative = path.strip_prefix(&root).unwrap_or(&path);
                        appenders.push(format!("{}:{}", relative.display(), index + 1));
                    }
                }
            }
        }
    }
    let production: Vec<&String> = appenders
        .iter()
        .filter(|site| !site.contains("tests"))
        .collect();
    assert!(
        production.is_empty(),
        "only the level-3 seam yields usage events, and it returns them for the \
         caller to log; no production site appends one directly: {production:?}"
    );

    // The seam itself is where the event is built, and it is level-3 only: it
    // reads a structured output line, which an ACP session never produces.
    let levels = read("core/meltemid/src/levels.rs");
    let seam = levels
        .split("pub fn usage_from_headless_line")
        .nth(1)
        .expect("the capture seam exists");
    let body = seam.split("\n}").next().expect("seam body");
    assert!(
        body.contains("SessionEventKind::UsageReported"),
        "the seam builds the event"
    );
    let production_levels = levels
        .split("#[cfg(test)]")
        .next()
        .expect("production half");
    assert_eq!(
        production_levels
            .matches("let event = SessionEventKind::UsageReported {")
            .count(),
        1,
        "exactly one production site in the mapping module builds a usage event"
    );
}
