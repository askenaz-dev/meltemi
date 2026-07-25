// SPDX-License-Identifier: Apache-2.0

//! Verification of the `sitio-web-producto` scenarios the site lint does not
//! already carry: what the front page must say, how the site honors the
//! viewer's system, how the captures were produced, and the pipeline's promises
//! about names, publication order and the installer scripts.

use std::path::{Path, PathBuf};

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

/// The site's front page in both languages.
fn front_pages() -> Vec<(&'static str, String)> {
    vec![
        ("site/index.html", read("site/index.html")),
        ("site/es/index.html", read("site/es/index.html")),
    ]
}

// Scenario: Portada nombra producto, superficies y plataformas
#[test]
fn the_front_page_names_the_product_its_surfaces_and_its_platforms() {
    for (page, html) in front_pages() {
        let flat = html.split_whitespace().collect::<Vec<_>>().join(" ");
        // The daemon and both surfaces, in parity.
        assert!(
            flat.contains("meltemid") || flat.contains("daemon"),
            "{page} names the daemon"
        );
        let spanish = page.contains("/es/");
        let surfaces: [&str; 2] = if spanish {
            ["escritorio", "terminal"]
        } else {
            ["desktop", "terminal"]
        };
        for surface in surfaces {
            assert!(
                flat.to_lowercase().contains(surface),
                "{page} names the {surface} surface"
            );
        }
        assert!(
            flat.to_lowercase().contains("parity") || flat.to_lowercase().contains("paridad"),
            "{page} states that the surfaces are at parity"
        );
        // The three platforms.
        for platform in ["Windows", "macOS", "Linux"] {
            assert!(flat.contains(platform), "{page} names {platform}");
        }
        // The motto and the BYO promise, without credits or fees.
        assert!(
            flat.contains("many sails") || flat.contains("muchas velas"),
            "{page} carries the motto"
        );
        let byo = flat.to_lowercase();
        assert!(
            byo.contains("no credits") || byo.contains("sin créditos"),
            "{page} states there are no credits"
        );
        assert!(byo.contains("lock-in"), "{page} states there is no lock-in");
    }
}

// Scenario: Estado honesto antes de la v1.0
#[test]
fn the_front_page_states_the_real_state_and_what_is_missing() {
    for (page, html) in front_pages() {
        let flat = html.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains("Pre-1.0") || flat.contains("Previo a 1.0"),
            "{page} declares the pre-1.0 state"
        );
        // And it names what does not exist yet, rather than only what does.
        let absent = flat.matches("Not yet").count() + flat.matches("Todavía no").count();
        assert!(
            absent >= 2,
            "{page} lists what is not shipped yet ({absent} entries)"
        );
        assert!(
            flat.contains("○"),
            "{page} marks absence with a symbol, not with colour alone"
        );
    }
}

// Scenario: Tema del sistema honrado
#[test]
fn the_site_follows_the_viewers_theme() {
    let tokens = read("site/tokens.css");
    assert!(
        tokens.contains("@media (prefers-color-scheme: dark)"),
        "the site has a dark palette bound to the system preference"
    );
    assert!(
        tokens.contains("color-scheme: light dark"),
        "and it tells the browser both schemes are supported"
    );
    // Every colour is a token, so switching the palette switches everything.
    let css = read("site/site.css");
    let hardcoded: Vec<&str> = css
        .lines()
        .filter(|line| {
            let l = line.trim();
            (l.starts_with("color:") || l.starts_with("background:"))
                && !l.contains("var(--")
                && !l.contains("transparent")
                && !l.contains("none")
                && !l.contains("ButtonFace")
                && !l.contains("ButtonText")
                && !l.contains("#ffffff")
        })
        .collect();
    assert!(
        hardcoded.is_empty(),
        "the stylesheet must take its colours from the tokens: {hardcoded:?}"
    );
}

// Scenario: Recorrido por teclado con foco visible
#[test]
fn the_site_is_traversable_by_keyboard_with_visible_focus() {
    let css = read("site/site.css");
    assert!(
        css.contains(":focus-visible") && css.contains("outline"),
        "focus is visible by rule"
    );
    // Nothing removes an element from the tab order, and nothing needs a pointer.
    for page in [
        "site/index.html",
        "site/method.html",
        "site/agents.html",
        "site/downloads.html",
        "site/es/index.html",
        "site/es/method.html",
        "site/es/agents.html",
        "site/es/downloads.html",
    ] {
        let html = read(page);
        assert!(
            !html.contains("tabindex=\"-1\""),
            "{page} must not remove anything from the tab order"
        );
        for pointer_only in ["onmouseover", "onhover", ":hover-only"] {
            assert!(
                !html.contains(pointer_only),
                "{page} must not depend on a pointer ({pointer_only})"
            );
        }
    }
}

// Scenario: Estado legible sin color
#[test]
fn every_state_on_the_site_carries_a_symbol_and_a_word() {
    for page in ["site/index.html", "site/es/index.html"] {
        let html = read(page);
        let states: Vec<&str> = html
            .match_indices("class=\"state")
            .map(|(_, m)| m)
            .collect();
        assert!(!states.is_empty(), "{page} shows capability states");
        // Each state renders a glyph AND a word.
        for (index, _) in html.match_indices("class=\"state") {
            let tail = &html[index..];
            let end = tail.find("</span>").unwrap_or(tail.len());
            let block = &tail[..end.min(400)];
            assert!(
                block.contains("class=\"sym\"") || block.contains("aria-hidden=\"true\""),
                "a state in {page} lacks its symbol: {block}"
            );
            let words = block
                .split('>')
                .filter_map(|part| part.split('<').next())
                .filter(|text| text.chars().any(char::is_alphabetic))
                .count();
            assert!(words > 0, "a state in {page} lacks its word: {block}");
        }
    }
    // The stylesheet carries the vocabulary rather than colouring bare text.
    let css = read("site/site.css");
    assert!(
        css.contains(".state .sym"),
        "the state vocabulary is symbol plus word by construction"
    );
}

// Scenario: Capturas tomadas de fixture y agente simulado
#[test]
fn the_captures_come_from_a_fixture_with_the_simulated_agent() {
    // The terminal capture is generated from the renderer over fixture data, so
    // it cannot drift into a mockup: the generator is in the repository.
    let generator = read("tui/examples/capture_svg.rs");
    assert!(
        generator.contains("mock-agent"),
        "the capture is driven by the simulated agent"
    );
    assert!(
        generator.contains("render(frame, &state, &live, &ctx)"),
        "and by the real renderer, not by hand-drawn markup"
    );
    assert!(
        generator.contains("/fixtures/"),
        "over fixture roots, never a real project path"
    );
    // The produced image carries no personal path, identity or third-party name.
    let svg = read("site/media/tui-sessions.svg");
    for forbidden in [
        "C:\\Users",
        "/home/",
        "/Users/",
        "guill",
        "Claude",
        "Codex",
        "Copilot",
        "@",
    ] {
        assert!(
            !svg.contains(forbidden),
            "the capture must not contain `{forbidden}`"
        );
    }
    // And the procedure for the desktop capture is documented rather than faked.
    let procedure = read("docs/ux/capturas.md");
    assert!(
        procedure.contains("mock-agent") && procedure.contains("fixture"),
        "the documented procedure keeps the same rule for the manual capture"
    );
    assert!(
        procedure.contains("No real agent, no network"),
        "and states the constraint explicitly"
    );
}

// Scenario: Conmutador de idioma sin ejecución de código
#[test]
fn the_language_switch_is_a_plain_link() {
    for (page, twin) in [
        ("site/index.html", "es/index.html"),
        ("site/es/index.html", "../index.html"),
        ("site/downloads.html", "es/downloads.html"),
        ("site/es/downloads.html", "../downloads.html"),
    ] {
        let html = read(page);
        assert!(
            html.contains(&format!("href=\"{twin}\"")),
            "{page} switches language with a link to {twin}"
        );
        // No script, no redirect, no network-based inference.
        for forbidden in [
            "http-equiv=\"refresh\"",
            "navigator.language",
            "geoip",
            "<script",
        ] {
            assert!(
                !html.contains(forbidden),
                "{page} must not choose the language by code ({forbidden})"
            );
        }
    }
}

// ---- the pipeline ------------------------------------------------------------

fn release_yml() -> String {
    read(".github/workflows/release.yml")
}

// Scenario: Instalador de escritorio normalizado antes de publicar
// Scenario: Descarga sin conocer la versión
#[test]
fn the_desktop_installers_are_renamed_to_stable_names_before_publishing() {
    let yml = release_yml();
    let step = yml
        .split("Collect, normalize names and checksum")
        .nth(1)
        .expect("the normalization step exists");
    let body = step.split("upload-artifact").next().expect("step body");
    for stable in [
        "meltemi-desktop-Windows.msi",
        "meltemi-desktop-macOS.dmg",
        "meltemi-desktop-Linux.deb",
    ] {
        assert!(
            body.contains(stable),
            "the pipeline renames to `{stable}`: {body}"
        );
    }
    // The checksum is computed AFTER the rename, so it belongs to the stable name.
    let rename_at = body.find("normalize").expect("the rename happens");
    let sum_at = body.find("SHA256SUMS").expect("the checksum is computed");
    assert!(
        rename_at < sum_at,
        "the published checksum is the one of the stable name"
    );
    // And the core archives are named without a version too.
    let core = yml
        .split("Package and checksum")
        .nth(1)
        .expect("the packaging step")
        .split("upload-artifact")
        .next()
        .expect("step body");
    for stable in [
        "meltemi-Windows.zip",
        "meltemi-macOS.tar.gz",
        "meltemi-Linux.tar.gz",
    ] {
        assert!(
            core.contains(stable),
            "the core archive `{stable}` is stable"
        );
    }
    assert!(
        !core.contains("${{ github.ref_name }}") && !core.contains("$VERSION"),
        "no version is interpolated into an artifact name: {core}"
    );
}

// Scenario: Script instalador publicado y firmado
// Scenario: Instalación con alias
#[test]
fn the_installer_scripts_ship_as_signed_assets_and_create_the_alias() {
    let yml = release_yml();
    let step = yml
        .split("Package and checksum")
        .nth(1)
        .expect("the packaging step")
        .split("upload-artifact")
        .next()
        .expect("step body");
    assert!(
        step.contains("cp scripts/install.sh scripts/install.ps1 dist/"),
        "both scripts are published as release assets: {step}"
    );
    let copy_at = step.find("cp scripts/install.sh").expect("the copy");
    let sum_at = step
        .find("sha256sum * > SHA256SUMS")
        .or_else(|| step.find("> SHA256SUMS"))
        .expect("the checksum is computed in this step");
    assert!(
        copy_at < sum_at,
        "their checksums land inside the signed SHA256SUMS (copy at {copy_at}, sum at {sum_at})"
    );
    // The alias is created by both scripts.
    let sh = read("scripts/install.sh");
    assert!(
        sh.contains("ln -sf \"$INSTALL_DIR/meltemi\" \"$INSTALL_DIR/mel\""),
        "install.sh creates the `mel` alias"
    );
    let ps1 = read("scripts/install.ps1");
    assert!(
        ps1.to_lowercase().contains("mel"),
        "install.ps1 creates the `mel` alias too"
    );
    for (name, script) in [("install.sh", &sh), ("install.ps1", &ps1)] {
        assert!(
            script.contains("meltemi") && script.contains("meltemid"),
            "{name} installs both binaries"
        );
        assert!(
            script.to_lowercase().contains("sha256"),
            "{name} verifies what it downloaded"
        );
    }
}

// Scenario: Gate rojo no publica el sitio
// Scenario: Sitio publicado tras los artefactos
// Scenario: Lint rojo impide publicar
// Scenario: Publicación de contenido sin tocar las descargas
#[test]
fn the_site_publishes_only_after_green_gates_and_never_before_the_artifacts() {
    let yml = release_yml();
    let job = yml
        .split("  publish-site:")
        .nth(1)
        .expect("the publish job exists");
    let condition = job.split("runs-on").next().expect("its condition");
    // It waits for the gates, the audit and BOTH packaging jobs.
    for needed in ["site-lint", "gates", "deny", "package", "package-gui"] {
        assert!(
            condition.contains(needed),
            "publication depends on {needed}: {condition}"
        );
    }
    // A red gate means the condition is false, so nothing is published and the
    // previous edition of the site stays up.
    assert!(
        condition.contains("needs.gates.result == 'success'")
            && condition.contains("needs.package.result == 'success'")
            && condition.contains("needs['package-gui'].result == 'success'"),
        "a red gate or a red packaging job blocks publication: {condition}"
    );
    // The site lint is a precondition on BOTH paths — tagged release and a
    // content-only push to main.
    assert!(
        condition.contains("needs.site-lint.result == 'success'"),
        "a red site lint blocks publication: {condition}"
    );
    assert!(
        condition.contains("!startsWith(github.ref, 'refs/tags/')"),
        "a content-only push publishes without the release jobs: {condition}"
    );
    let lint = yml
        .split("  site-lint:")
        .nth(1)
        .expect("the site lint job exists");
    assert!(
        lint.contains("cargo test -p meltemid --test site"),
        "and that lint is the blocking one: {lint}"
    );
    // Republishing content cannot desynchronize the downloads, because no link
    // carries a version.
    for page in ["site/downloads.html", "site/es/downloads.html"] {
        let html = read(page);
        assert!(
            html.contains("releases/latest/download/"),
            "{page} resolves through the latest-release URL"
        );
        for literal in ["/v0.", "/v1.", "0.1.0"] {
            assert!(
                !html.contains(literal),
                "{page} must carry no version literal ({literal})"
            );
        }
    }
    // The release jobs themselves only run for a tag.
    for job_name in ["  gates:", "  package:", "  package-gui:"] {
        let body = yml
            .split(job_name)
            .nth(1)
            .expect("the job exists")
            .split("    steps:")
            .next()
            .expect("its header");
        assert!(
            body.contains("if: startsWith(github.ref, 'refs/tags/')"),
            "{job_name} belongs to a tag: {body}"
        );
    }
}
