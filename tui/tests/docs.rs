// SPDX-License-Identifier: Apache-2.0

//! Documentation lint (documentacion-inicial): the root README and the `docs/`
//! skeleton must be present with their sections, internal links must resolve,
//! the generated CLI reference must be fresh, and the quickstart's scriptable
//! steps must run against the real built binary.
//!
//! Reads the repository's own static docs and runs the built `meltemi` binary;
//! it is a docs/pipeline lint, not an agent e2e.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root (this crate lives at `<root>/tui`).
fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // <root>
    dir
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("missing doc: {rel}"))
}

fn norm(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn readme_and_docs_are_present_with_their_sections() {
    // Scenario: Primer contacto completo
    // README explains what/status/how.
    let root = repo_root();
    let readme = read(&root, "README.md");
    for section in ["## What it is", "## Status", "## Install", "## First step"] {
        assert!(readme.contains(section), "README missing `{section}`");
    }
    assert!(
        readme.contains("LEEME.md"),
        "README links its Spanish mirror"
    );
    // No third-party product names in the public README (only open standards).
    let lower = readme.to_ascii_lowercase();
    assert!(
        !lower.contains("claude") && !lower.contains("gemini") && !lower.contains("copilot"),
        "the public README must not name third-party products"
    );

    // The docs skeleton is present.
    for doc in [
        "LEEME.md",
        "docs/quickstart.md",
        "docs/arquitectura.md",
        "docs/metodo-sdd.md",
        "docs/accesibilidad.md",
        "docs/plataformas.md",
        "docs/agentes.md",
        "docs/referencia-cli.md",
        "docs/acceso-remoto.md",
    ] {
        assert!(root.join(doc).is_file(), "missing doc: {doc}");
    }
}

// Scenario: Los cuatro cuadrantes usan el mismo camino
// Scenario: La malla del usuario no es una dependencia de Meltemi
// Scenario: Lo de fase 3 está anotado y no prometido
#[test]
fn the_rendezvous_pattern_is_documented_with_its_boundary() {
    let doc = read(&repo_root(), "docs/acceso-remoto.md");
    // Prose wraps: a sentence split across lines is the same sentence, so the
    // whitespace is normalised before any phrase is looked for.
    let lower = doc
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // The bridge: the last metre, with its exact command.
    assert!(
        doc.contains("meltemi bridge") && lower.contains("último metro"),
        "the bridge is documented as the last metre, with its command"
    );

    // Both ends dial OUT — the rule that makes the four quadrants one path.
    assert!(
        lower.contains("hacia afuera"),
        "the rule is stated: nobody receives connections"
    );
    for quadrant in ["En casa | Fuera", "Fuera | En casa", "Fuera | Fuera"] {
        assert!(
            doc.contains(quadrant),
            "the quadrant `{quadrant}` is covered by the same path"
        );
    }
    // And the cost of the rendezvous is said, not hidden.
    assert!(
        lower.contains("apagado significa sin acceso remoto"),
        "the availability cost of the rendezvous is declared"
    );

    // BYO-network with its licences AND its boundary: the mesh is the user's,
    // never a dependency of the workspace.
    assert!(
        lower.contains("byo-network") && doc.contains("BSD-3") && doc.contains("Apache-2.0"),
        "the mesh variant names its pieces and their licences"
    );
    assert!(
        lower.contains("no se empaqueta") || lower.contains("no es dependencia"),
        "the boundary is explicit: none of that infrastructure is Meltemi's"
    );
    assert!(
        lower.contains("no exige cuenta ni red"),
        "compiling and testing Meltemi needs no account and no network"
    );

    // The user's mesh identity is DOCUMENTED AS BUILT, with the boundary that
    // makes it legal: read from the user's own client, never issued, never
    // executed, and presence that controls nothing (identidad-propia 4.1).
    let identity = doc
        .split("## Identidad: la de tu malla")
        .nth(1)
        .expect("the identity section exists")
        .split("\n## ")
        .next()
        .expect("the section has an end");
    let identity_lower = identity
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        identity_lower.contains("no tiene cuentas"),
        "the section opens by saying Meltemi has no accounts"
    );
    assert!(
        identity.contains("[identity]") && identity_lower.contains("de usuario"),
        "the table is documented as user scope"
    );
    assert!(
        identity_lower.contains("se ignora con diagnóstico"),
        "and a project's table is said to be ignored, with its diagnostic"
    );
    assert!(
        identity.contains("status --json") && identity_lower.contains("solo lectura"),
        "the one read-only invocation is named"
    );
    for forbidden in ["`up`", "`login`", "`logout`"] {
        assert!(
            identity.contains(forbidden),
            "the verbs the daemon never runs are named: {forbidden}"
        );
    }
    assert!(
        identity_lower.contains("tiempo límite"),
        "the read has a deadline and the page says so"
    );
    assert!(
        identity_lower.contains("presencia no es control"),
        "presence is declared read-only: knowing a machine is online opens nothing"
    );
    for reason in ["not_configured", "client_missing", "not_linked"] {
        assert!(
            identity.contains(reason),
            "the three refusals ask three remedies and are named: {reason}"
        );
    }
    assert!(
        identity.contains("identity-link") && identity_lower.contains("jamás ejecuta"),
        "the gesture is composed and never run by the daemon"
    );
    assert!(
        identity_lower.contains("--exec") && identity_lower.contains("heredadas"),
        "and running it is the user's own binary with stdio inherited"
    );
    // The field verification that is still pending is written as pending, with
    // the version the shape was read against. An honest page over a tidy one.
    assert!(
        identity_lower.contains("pendiente") && identity.contains("v1.102.5"),
        "the pending field check names the version the type was read at"
    );

    // The machine selector is DOCUMENTED AS BUILT too, with the boundaries that
    // keep it legal: user scope, no key material, nothing that polls, and no
    // aggregation across machines (maquinas-remotas 4.1).
    let machines = doc
        .split("## Tus máquinas, con nombre")
        .nth(1)
        .expect("the machines section exists")
        .split("\n## ")
        .next()
        .expect("the section has an end");
    let machines_lower = machines
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        machines.contains("[[client.machines]]") && machines_lower.contains("de usuario"),
        "the list is documented as user scope"
    );
    assert!(
        machines_lower.contains("sin material de claves"),
        "and as carrying no key material"
    );
    assert!(
        machines.contains("~/.ssh/config"),
        "with the place that data actually lives"
    );
    assert!(
        machines_lower.contains("rehúsa la máquina entera"),
        "writing one is refused loudly rather than ignored"
    );
    assert!(
        machines_lower.contains("cambiar de máquina es reconectar"),
        "switching is reconnecting, not restarting"
    );
    assert!(
        machines_lower.contains("trabajo sin guardar"),
        "and the unsaved-work guard is named"
    );
    assert!(
        machines_lower.contains("no arranca nada"),
        "testing a machine starts nothing"
    );
    assert!(
        machines_lower.contains("nunca de fondo"),
        "presence is read on demand and never polled"
    );
    assert!(
        machines_lower.contains("no dice que allí corra"),
        "and presence is not a promise about a daemon"
    );
    assert!(
        machines_lower.contains("un agregador es un servicio"),
        "nothing aggregates sessions across machines, with the reason"
    );
    // The CLI's own limit is written where somebody would look for it, instead
    // of being discovered by trying.
    assert!(
        machines_lower.contains("lista y no cambia"),
        "the CLI lists and does not switch, and says so"
    );

    // Phase 3 is annotated, never promised — and what has since been BUILT is no
    // longer in the notes. This half of the test has now been rewritten twice,
    // by the two changes that built what it used to reserve; each time it pinned
    // the old semantics and each time that was the point of rewriting it.
    let phase3 = doc
        .split("## Notas para el compañero móvil")
        .nth(1)
        .expect("the phase-3 notes section exists")
        // Bounded to the section: unbounded it ran to the end of the page and
        // swallowed the sections after it, so "this is still a note" would be
        // satisfied by a heading that says the opposite.
        .split(
            "
## ",
        )
        .next()
        .expect("the section has an end");
    assert!(
        phase3.contains("no existe todavía"),
        "the phase-3 notes say plainly that none of it exists yet"
    );
    assert!(
        phase3.contains("Certificados SSH del bastión"),
        "the one design note left is recorded"
    );
    // What is left is a decision for a FUTURE change, and the note says what it
    // is rather than naming a change that has already happened: `companero-movil`
    // built the companion, so pointing at it would read as "coming soon" for
    // something shipped.
    assert!(
        phase3.contains("certificados SSH de vida corta"),
        "the note says what the decision is"
    );
    // The notes must not promise what is already built, which is the failure mode
    // a stale note creates: a reader deciding the feature is years away.
    let phase3_lower = phase3
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for built in [
        "byo-identity",
        "selector que conecta",
        "aviso de espera",
        "android primero",
    ] {
        assert!(
            !phase3_lower.contains(built),
            "`{built}` is built and must not still read as a future thing"
        );
    }
    assert!(
        phase3_lower.contains("existen ya"),
        "and the notes point at what already exists instead of restating it"
    );
}

#[test]
fn the_remote_access_frontier_is_documented() {
    // Scenario: La frontera está documentada como postura
    // Scenario: Propuesta de push sin túnel se rechaza
    let doc = read(&repo_root(), "docs/acceso-remoto.md").to_ascii_lowercase();
    // The frontier: remote control requires a live tunnel.
    assert!(
        doc.contains("túnel vivo"),
        "the live-tunnel requirement is declared"
    );
    // The three remote verbs are named.
    for verb in ["monitorear", "aprobar", "dirigir"] {
        assert!(doc.contains(verb), "the `{verb}` remote verb is documented");
    }
    // The absence of push without a connection, explained as a posture with its
    // why (§3 / no cloud relay), not as a missing feature.
    assert!(
        doc.contains("postura de privacidad"),
        "the no-push stance is framed as a posture"
    );
    assert!(
        doc.contains("§3") && doc.contains("relé"),
        "the why is grounded in §3 and the no-relay rule"
    );
    // A push-without-tunnel proposal is rejected save a foundational amendment.
    assert!(
        doc.contains("se rechaza") && doc.contains("enmienda"),
        "the rejection rule (save amendment) is stated"
    );
}

/// A document with its line breaks and runs of spaces folded, lowercased: a
/// sentence wrapped across lines is still the same sentence.
fn flat(rel: &str) -> String {
    read(&repo_root(), rel)
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Scenario: El PC Windows servido está documentado con lo no medido
#[test]
fn the_served_windows_pc_is_documented_with_what_is_not_measured() {
    let doc = read(&repo_root(), "docs/acceso-remoto.md");
    for step in [
        "Add-WindowsCapability -Online -Name OpenSSH.Server",
        "Start-Service sshd",
        "ssh <tu-pc> meltemi bridge",
        "ssh -t <tu-pc> meltemi",
    ] {
        assert!(doc.contains(step), "the served PC documents `{step}`");
    }
    let flat = flat("docs/acceso-remoto.md");
    assert!(flat.contains("**medido**"), "what is measured is said");
    assert!(
        flat.contains("**no medido**: el tramo a través de un `sshd` real de windows"),
        "and what is not"
    );
}

// Scenario: El teléfono de hoy está documentado con sus límites
#[test]
fn todays_phone_is_documented_with_its_limits() {
    let flat = flat("docs/acceso-remoto.md");
    assert!(
        flat.contains("## desde el teléfono, hoy"),
        "its own section"
    );
    // The verbs without the TUI (verbos-del-agent-boss), the TUI over `ssh -t`
    // with its floor, and the bell.
    assert!(flat.contains("ssh <tu-pc> meltemi approve perm-3"));
    assert!(flat.contains("ssh -t <tu-pc> meltemi"));
    assert!(flat.contains("80×24"), "the minimum size is stated");
    assert!(flat.contains("meltemi_bell=1"), "the bell is named");
    // And what there is not.
    assert!(
        flat.contains("no hay push sin conexión"),
        "no push without a connection"
    );
}

// Scenario: Cada espera tiene su ajuste y su valor
#[test]
fn every_wait_has_its_setting_and_its_default() {
    let doc = read(&repo_root(), "docs/acceso-remoto.md");
    for row in [
        "| `[permissions] wait` | `\"while-connected\"` |",
        "| `[permissions] implement-wait` | `30` |",
        "| `[permissions] no-client-grace` | `30` |",
        "| `[sessions] idle-timeout` | `900` |",
        "| `[sessions] max-idle` | `3` |",
        "| `[sessions] lever-wait` | `30` |",
    ] {
        assert!(doc.contains(row), "the waits table has `{row}`");
    }
}

// Scenario: Plataformas y el helper dicen lo mismo
#[test]
fn the_platforms_page_and_the_tunnel_helper_agree() {
    let doc = read(&repo_root(), "docs/plataformas.md");
    // The advice that contradicted the helper is gone…
    assert!(
        !doc.contains(
            "Forward the Unix socket
(`ssh -L`), then point the client"
        ),
        "the old -L-only advice"
    );
    // …and each end says what it runs.
    let flat = flat("docs/plataformas.md");
    assert!(flat.contains("`meltemi tunnel <user@host>` prints the `ssh -r` command"));
    assert!(flat.contains("`ssh -l <local.sock>:<daemon.sock> <host>`"));
    assert!(flat.contains("`ssh <pc> meltemi bridge`"));
    assert!(
        flat.contains("`meltemi_start_here=1`"),
        "the start flag is documented"
    );
}

// Scenario: Las carpetas de cada plataforma
// Scenario: La guía coincide con el código
#[test]
fn the_platforms_page_names_the_folders_the_code_resolves() {
    use meltemi_client::paths;

    let bs = char::from(92u8);
    let guide = norm(&read(&repo_root(), "docs/plataformas.md"));
    let section = guide
        .split("## Data paths")
        .nth(1)
        .expect("a `Data paths` section")
        .split("\n## ")
        .next()
        .unwrap_or_default();

    // The guide names the data and the configuration folder of the three platforms…
    let named = [
        format!("%APPDATA%{bs}meltemi{bs}data"),
        format!("%APPDATA%{bs}meltemi{bs}config"),
        "~/Library/Application Support/meltemi".to_string(),
        "$XDG_DATA_HOME/meltemi".to_string(),
        "~/.local/share/meltemi".to_string(),
        "$XDG_CONFIG_HOME/meltemi".to_string(),
        "~/.config/meltemi".to_string(),
    ];
    for folder in &named {
        assert!(
            section.contains(folder.as_str()),
            "the guide does not name {folder}"
        );
    }
    assert!(
        !section.contains("LOCALAPPDATA"),
        "the daemon does not use the local profile"
    );

    // …and the ones the code resolves on THIS platform are those.
    if std::env::var_os(paths::ENV_DATA_DIR).is_some()
        || std::env::var_os(paths::ENV_CONFIG_DIR).is_some()
    {
        eprintln!("the folders are overridden by the environment: the defaults are not under test");
        return;
    }
    let (data, config) = (paths::data_dir(), paths::config_dir());
    if cfg!(windows) {
        assert!(
            data.ends_with("meltemi/data") && config.ends_with("meltemi/config"),
            "windows folders: {data:?}, {config:?}"
        );
        assert!(
            data.components().any(|c| c.as_os_str() == "Roaming"),
            "the data folder is in the roaming profile: {data:?}"
        );
    } else if cfg!(target_os = "macos") {
        assert_eq!(data, config, "macOS keeps both in one folder");
        assert!(
            data.ends_with("Library/Application Support/meltemi"),
            "macOS folder: {data:?}"
        );
    } else {
        let xdg = |var: &str, home_relative: &str, got: &Path| match std::env::var_os(var)
            .filter(|v| !v.is_empty())
        {
            Some(base) => assert_eq!(got, Path::new(&base).join("meltemi")),
            None => assert!(got.ends_with(home_relative), "linux folder: {got:?}"),
        };
        xdg("XDG_DATA_HOME", ".local/share/meltemi", &data);
        xdg("XDG_CONFIG_HOME", ".config/meltemi", &config);
    }
}

// Scenario: El smoke dice qué se midió y qué no
#[test]
fn the_tunnel_smoke_says_what_was_measured_and_what_was_not() {
    let smoke = read(&repo_root(), "docs/qa/2026-09-25-acceso-por-tunel-smoke.md");
    // Dated, with the platform and the binaries it measured.
    assert!(smoke.contains("**Fecha**: 2026-09-25"), "dated");
    assert!(smoke.contains("Windows 11"), "the platform");
    assert!(smoke.contains("`meltemi` 0.1.1"), "the version");
    // What was measured…
    assert!(smoke.contains("## Qué se midió"), "a measured part");
    assert!(
        smoke.contains("`exit 10` en 34 ms"),
        "the refusal, measured"
    );
    // …and every leg not measured, pending with what it needs.
    assert!(
        smoke.contains("| Tramo | Estado | Qué falta |"),
        "the pending legs"
    );
    for leg in [
        "GUI y TUI contra un daemon macOS/Linux reenviado",
        "`ssh <pc> meltemi bridge` a través de un `sshd` real de Windows",
    ] {
        let row = smoke
            .lines()
            .find(|line| line.contains(leg))
            .unwrap_or_else(|| panic!("the smoke lists `{leg}`"));
        assert!(row.contains("**pendiente**"), "{row}");
    }
}

// Scenario: La documentación no promete lo no medido
#[test]
fn the_remote_access_page_does_not_promise_what_is_not_measured() {
    let doc = read(&repo_root(), "docs/acceso-remoto.md");
    let flat = flat("docs/acceso-remoto.md");
    // The frontier's claim is what is expected, pointing at what is measured.
    assert!(
        !flat.contains("con el túnel establecido, funciona todo"),
        "the unconditional claim is gone"
    );
    assert!(
        flat.contains("lo esperado es que funcione todo"),
        "what is expected"
    );
    assert!(
        doc.contains("(qa/2026-09-25-acceso-por-tunel-smoke.md)"),
        "and where it is measured"
    );
}

// Scenario: La receta de aprobar desde el teléfono está documentada
#[test]
fn the_phone_approval_recipe_is_documented() {
    let doc = read(&repo_root(), "docs/acceso-remoto.md");
    // The recipe itself, as the commands a phone's terminal app runs.
    for line in [
        "ssh <tu-pc> meltemi permissions",
        "ssh <tu-pc> meltemi approve perm-3",
        "ssh <tu-pc> meltemi watch <sesión>",
    ] {
        assert!(doc.contains(line), "the recipe runs `{line}`");
    }
    let flat = doc
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // No tunnel is needed for it, and nothing decides in bulk.
    assert!(
        flat.contains("no hace falta túnel"),
        "the route needs no tunnel"
    );
    assert!(flat.contains("nunca en bloque"), "one request at a time");
    // Where the rest of a request is, since the queue carries tool and title.
    assert!(
        flat.contains("meltemi log <sesión>"),
        "the record has the full call"
    );
}

#[test]
fn the_git_bash_gotcha_is_documented() {
    // Scenario: La trampa de git-bash documentada
    // the H6 finding with remedy.
    let notes = read(&repo_root(), "docs/plataformas.md");
    assert!(
        notes.contains("MELTEMI_ENDPOINT"),
        "the mangled var is named"
    );
    assert!(
        notes.contains("MSYS_NO_PATHCONV"),
        "the remedy for the git-bash mangling is documented"
    );
}

#[test]
fn the_cli_reference_is_generated_and_fresh() {
    // Scenario: Referencia nunca desincronizada
    // the committed reference must
    // equal what the generator produces from the grammar.
    // Scenario: La referencia es la enumeración autoritativa
    let committed = read(&repo_root(), "docs/referencia-cli.md");
    let generated = meltemi::cli::reference();
    assert_eq!(
        norm(&committed),
        norm(&generated),
        "docs/referencia-cli.md is stale; regenerate with \
         `cargo run --example gen_cli_ref > docs/referencia-cli.md`"
    );
}

#[test]
fn the_captures_the_site_copies_are_well_formed_svg() {
    // meltemi-web copies docs/media/ into the site as is (docs/ux/capturas.md).
    // XML forbids `--` inside a comment, and a browser renders nothing for an
    // SVG that is not well-formed, so one stray `--` costs the site its figure.
    let media = repo_root().join("docs/media");
    let mut problems = Vec::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(&media).expect("docs/media exists") {
        let path = entry.expect("readable entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }
        checked += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let svg = std::fs::read_to_string(&path).expect("readable svg");
        if !svg.trim_start().starts_with("<svg") || !svg.trim_end().ends_with("</svg>") {
            problems.push(format!("{name}: not a single <svg> root element"));
        }
        let mut rest = svg.as_str();
        while let Some(start) = rest.find("<!--") {
            let after = &rest[start + 4..];
            let Some(end) = after.find("-->") else {
                problems.push(format!("{name}: a comment is never closed"));
                break;
            };
            let body = &after[..end];
            if body.contains("--") || body.ends_with('-') {
                problems.push(format!("{name}: `--` inside the comment `{}`", body.trim()));
            }
            rest = &after[end + 3..];
        }
    }
    assert!(checked > 0, "docs/media holds no SVG capture to check");
    assert!(
        problems.is_empty(),
        "docs/media captures are not well-formed:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_terminal_capture_carries_the_brands_colours() {
    // Scenario: La captura muestra los colores
    //
    // The capture is generated with a 24-bit terminal (capture_svg), so what
    // the TUI emphasises is a block in the brand's blue with bold text, written
    // as attributes the site renders without a stylesheet of its own. The well
    // formedness half of the scenario is the test above, over every capture.
    let svg = read(&repo_root(), "docs/media/tui-sessions.svg");
    let accent_blocks = svg.matches("<rect fill=\"#2563eb\"").count();
    assert!(
        accent_blocks > 0,
        "the emphasised runs are blocks in the brand's blue (#2563eb)"
    );
    assert!(
        svg.contains("font-weight=\"700\""),
        "and their text is bold"
    );
}

#[test]
fn both_readmes_carry_the_ci_licence_and_release_badges() {
    // antes-de-anunciar design D2: in the READMEs, never on the site, which
    // keeps no external origins.
    for doc in ["README.md", "LEEME.md"] {
        let text = read(&repo_root(), doc);
        for (what, link) in [
            ("CI", "actions/workflows/ci.yml/badge.svg"),
            ("licence", "img.shields.io/badge/license-Apache--2.0"),
            (
                "latest release",
                "img.shields.io/github/v/release/askenaz-dev/meltemi",
            ),
        ] {
            assert!(text.contains(link), "{doc} lacks the {what} badge");
        }
    }
}

#[test]
fn the_faq_starts_with_the_diagnosis() {
    // Scenario: Lo primero que pide la FAQ es el diagnóstico
    let faq = read(&repo_root(), "docs/faq.md");
    let first = faq
        .split("\n## ")
        .nth(1)
        .expect("the FAQ has a first question");
    assert!(
        first.contains("meltemi doctor"),
        "the first answer sends to the diagnosis: {first}"
    );
    let bug = read(&repo_root(), ".github/ISSUE_TEMPLATE/bug.md");
    assert!(bug.contains("meltemi doctor"), "and the bug template asks for it");
}

#[test]
fn the_faq_answers_what_is_asked_first() {
    // Scenario: Cada pregunta tiene su respuesta
    let faq = read(&repo_root(), "docs/faq.md");
    for question in [
        "## The fleet says my agent is not launchable",
        "## The command line denies every permission",
        "## How do I use Meltemi from another machine?",
        "## The session ended without saying anything",
        "## The agent edited files without asking me",
        "## The installer warns that it is not signed",
        "## Where are my data and my logs?",
    ] {
        assert!(faq.contains(question), "the FAQ lacks `{question}`");
    }
    for doc in ["README.md", "LEEME.md", "docs/quickstart.md"] {
        assert!(
            read(&repo_root(), doc).contains("faq.md"),
            "{doc} does not link the FAQ"
        );
    }
}

#[test]
fn internal_doc_links_resolve() {
    // Docs lint: internal markdown links point at files that exist.
    let root = repo_root();
    let docs = [
        "README.md",
        "LEEME.md",
        "docs/quickstart.md",
        "docs/arquitectura.md",
        "docs/metodo-sdd.md",
        "docs/accesibilidad.md",
        "docs/plataformas.md",
        "docs/acceso-remoto.md",
        "docs/faq.md",
    ];
    let mut problems = Vec::new();
    for doc in docs {
        let content = read(&root, doc);
        let base = root.join(doc);
        let base_dir = base.parent().unwrap();
        for target in markdown_link_targets(&content) {
            // Skip external links and pure anchors.
            if target.starts_with("http") || target.starts_with('#') {
                continue;
            }
            let path = target.split('#').next().unwrap_or(&target);
            if path.is_empty() {
                continue;
            }
            if !base_dir.join(path).exists() {
                problems.push(format!("{doc} -> `{target}` does not resolve"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "broken internal links:\n{}",
        problems.join("\n")
    );
}

/// Extracts the targets of `[text](target)` markdown links.
fn markdown_link_targets(content: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b']'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'('
            && let Some(end) = content[i + 2..].find(')')
        {
            targets.push(content[i + 2..i + 2 + end].trim().to_string());
            i += 2 + end;
        }
        i += 1;
    }
    targets
}

/// The built `meltemi` binary next to the test executable.
fn meltemi_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "meltemi.exe"
    } else {
        "meltemi"
    })
}

#[test]
fn the_quickstart_starts_at_the_release_and_not_at_the_compiler() {
    // Scenario: El quickstart no pide compilar
    let quickstart = read(&repo_root(), "docs/quickstart.md");
    for asked in ["cargo build", "[agent]", "command = ["] {
        assert!(
            !quickstart.contains(asked),
            "the quickstart asks for `{asked}`: building and hand-written agent \
             configuration belong to contributors, not to a first install"
        );
    }
    for needed in [
        "releases/latest/download/",
        "SHA256SUMS",
        "gh attestation verify",
        "meltemi fleet",
        "meltemi session",
    ] {
        assert!(
            quickstart.contains(needed),
            "the quickstart lacks `{needed}`"
        );
    }
    // Building from source stays documented, where contributors look.
    assert!(read(&repo_root(), "README.md").contains("Option B — from source"));
    assert!(read(&repo_root(), "CONTRIBUTING.md").contains("cargo build"));
}

#[test]
fn quickstart_local_steps_run_against_the_binary() {
    // Scenario: Quickstart en CI
    // the scriptable steps run against the real
    // binary and a divergent output fails.
    let bin = meltemi_bin();
    assert!(
        bin.exists(),
        "build the `meltemi` binary first (cargo test builds it)"
    );

    // `meltemi version` — local, prints a semver-shaped version, exit 0.
    let out = Command::new(&bin)
        .arg("version")
        .output()
        .expect("run version");
    assert!(out.status.success(), "`meltemi version` exits 0");
    let version = String::from_utf8_lossy(&out.stdout);
    assert!(
        version.trim().split('.').count() >= 2,
        "version looks like a version: {version:?}"
    );

    // `meltemi help` — local, lists the documented subcommands.
    let out = Command::new(&bin).arg("help").output().expect("run help");
    assert!(out.status.success(), "`meltemi help` exits 0");
    let help = String::from_utf8_lossy(&out.stdout);
    for verb in ["propose", "review", "implement", "archive"] {
        assert!(
            help.contains(verb),
            "help documents `{verb}`: divergence would fail here"
        );
    }
}
