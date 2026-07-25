// SPDX-License-Identifier: Apache-2.0

//! The product site lint (sitio-web-producto design D8): a workspace test that
//! reads the repository and refuses a site that would lie or leak.
//!
//! No browser, no network: the e2e suite and every gate of this project stay
//! offline, and a lint that depends on live third parties is an intermittent
//! lint. What it enforces:
//!
//! - required pages and sections, and every internal link resolving;
//! - zero JavaScript and zero external origins (privacy is verifiable by
//!   inspection, not promised);
//! - download links free of any version literal, naming only artifacts the
//!   release pipeline actually produces, resolved against ONE canonical base;
//! - site tokens identical, name and value, to the desktop client's;
//! - a twin page in the other language for every page;
//! - no block of six or more lines copied from `docs/` (one truth, one place);
//! - a figure with alternative text and declared provenance;
//! - no third-party product name outside interoperability facts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // core/meltemid/tests -> repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The English pages at the root; each one must have a Spanish twin under `es/`.
const PAGES: [&str; 4] = ["index.html", "method.html", "agents.html", "downloads.html"];

/// Sections every page must carry, by page (the `id` of its heading).
fn required_sections(page: &str) -> &'static [&'static str] {
    match page {
        "index.html" => &["what", "byo", "surfaces", "state", "not"],
        "method.html" => &["loop", "ears", "agentic", "read"],
        "agents.html" => &["levels", "subscriptions", "fair", "start"],
        "downloads.html" => &["pick", "script", "verify", "build"],
        _ => &[],
    }
}

/// Every HTML file of the site, relative to `site/`.
fn site_pages() -> Vec<String> {
    let mut pages: Vec<String> = PAGES.iter().map(|p| (*p).to_string()).collect();
    pages.extend(PAGES.iter().map(|p| format!("es/{p}")));
    pages
}

fn site_dir() -> PathBuf {
    repo_root().join("site")
}

/// The canonical download base, declared once in `docs/release.md`.
fn canonical_base() -> String {
    let doc = read(&repo_root().join("docs/release.md"));
    let line = doc
        .lines()
        .find(|line| line.starts_with("CANONICAL_DOWNLOAD_BASE="))
        .expect("docs/release.md declares CANONICAL_DOWNLOAD_BASE=<url>");
    line.trim_start_matches("CANONICAL_DOWNLOAD_BASE=")
        .trim()
        .to_string()
}

// Scenario: Página sin gemela rehusada
#[test]
fn every_page_exists_with_its_language_twin() {
    let site = site_dir();
    for page in site_pages() {
        let path = site.join(&page);
        assert!(path.is_file(), "missing site page: site/{page}");
    }
    // A page added in one language only is a half-language: refuse it.
    for entry in std::fs::read_dir(&site).expect("read site/") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "html") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            assert!(
                site.join("es").join(&name).is_file(),
                "site/{name} has no Spanish twin at site/es/{name}"
            );
        }
    }
    for entry in std::fs::read_dir(site.join("es")).expect("read site/es/") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "html") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            assert!(
                site.join(&name).is_file(),
                "site/es/{name} has no English twin at site/{name}"
            );
        }
    }
}

// Scenario: Sección requerida ausente rompe la CI
#[test]
fn every_page_declares_its_language_alternates_and_sections() {
    let site = site_dir();
    for page in site_pages() {
        let html = read(&site.join(&page));
        let spanish = page.starts_with("es/");
        let expected_lang = if spanish { "es" } else { "en" };
        assert!(
            html.contains(&format!("<html lang=\"{expected_lang}\">")),
            "site/{page} must declare lang=\"{expected_lang}\""
        );
        // hreflang alternates in both directions, plus x-default.
        for needle in [
            "hreflang=\"en\"",
            "hreflang=\"es\"",
            "hreflang=\"x-default\"",
        ] {
            assert!(
                html.contains(needle),
                "site/{page} is missing its {needle} alternate"
            );
        }
        // The language switch is a plain link: no code chooses the language.
        let leaf = page.trim_start_matches("es/");
        let twin = if spanish {
            format!("../{leaf}")
        } else {
            format!("es/{leaf}")
        };
        assert!(
            html.contains(&format!("href=\"{twin}\"")),
            "site/{page} must link its twin as a plain link ({twin})"
        );
        for section in required_sections(leaf) {
            assert!(
                html.contains(&format!("id=\"{section}\"")),
                "site/{page} lost its required section `{section}`"
            );
        }
    }
}

// Scenario: Ninguna página ejecuta código
// Scenario: Sin cookies ni recolección de datos
#[test]
fn the_site_runs_no_code_and_collects_nothing() {
    let site = site_dir();
    for page in site_pages() {
        let html = read(&site.join(&page));
        let lower = html.to_lowercase();
        for forbidden in [
            "<script",
            "javascript:",
            "<form",
            "<iframe",
            "<embed",
            "localstorage",
            "sessionstorage",
            "document.cookie",
        ] {
            assert!(
                !lower.contains(forbidden),
                "site/{page} contains `{forbidden}`: the site executes no code and collects nothing"
            );
        }
        // Inline event handlers are code too.
        for handler in [
            "onclick=",
            "onload=",
            "onsubmit=",
            "onmouseover=",
            "onerror=",
        ] {
            assert!(
                !lower.contains(handler),
                "site/{page} contains the inline handler `{handler}`"
            );
        }
    }
}

// Scenario: Origen externo rehusado
#[test]
fn no_page_loads_a_resource_from_an_external_origin() {
    let site = site_dir();
    // Loading attributes only: `href` on an <a> is a link, not a fetch.
    let loaders = ["src=\"", "srcset=\"", "@import", "url("];
    for page in site_pages() {
        let html = read(&site.join(&page));
        for loader in loaders {
            let mut rest = html.as_str();
            while let Some(at) = rest.find(loader) {
                rest = &rest[at + loader.len()..];
                let value: String = rest
                    .chars()
                    .take_while(|c| *c != '"' && *c != ')')
                    .collect();
                assert!(
                    !value.starts_with("http") && !value.starts_with("//"),
                    "site/{page} loads an external resource: {value}"
                );
            }
        }
        // A stylesheet <link> is a fetch as well.
        for link in html.match_indices("rel=\"stylesheet\"") {
            let tail = &html[link.0..];
            let end = tail.find('>').unwrap_or(tail.len());
            let tag = &tail[..end];
            assert!(
                !tag.contains("http"),
                "site/{page} loads an external stylesheet: {tag}"
            );
        }
        // No remote font provider, under any spelling.
        assert!(
            !html.contains("fonts.googleapis") && !html.contains("@font-face"),
            "site/{page} must not load a remote or self-hosted font family"
        );
    }
    for sheet in ["tokens.css", "site.css"] {
        let css = read(&site.join(sheet));
        assert!(
            !css.contains("@import") && !css.contains("http"),
            "site/{sheet} must not reach an external origin"
        );
    }
}

// Scenario: Enlaces requeridos presentes y resueltos
#[test]
fn internal_links_resolve_and_the_required_sources_are_linked() {
    let site = site_dir();
    let root = repo_root();
    let required = [
        "meltemi.md",
        ".meltemi/constitution.md",
        "docs/agentes.md",
        "docs/release.md",
        "docs/referencia-cli.md",
    ];
    let mut linked_repo_files: BTreeSet<String> = BTreeSet::new();

    for page in site_pages() {
        let html = read(&site.join(&page));
        let dir = site.join(&page).parent().expect("page dir").to_path_buf();
        let mut rest = html.as_str();
        while let Some(at) = rest.find("href=\"") {
            rest = &rest[at + 6..];
            let target: String = rest.chars().take_while(|c| *c != '"').collect();
            if target.starts_with("http") {
                // Remember the repository files the site points at, so the
                // required-link assertion below is about real targets.
                if let Some(tail) = target.split("/blob/main/").nth(1) {
                    linked_repo_files.insert(tail.to_string());
                }
                continue;
            }
            if target.starts_with('#') || target.starts_with('/') {
                continue;
            }
            let resolved = dir.join(&target);
            // Brand marks are staged into media/ from brand/ at publish time
            // (single source: no binary copies live in site/).
            if let Some(mark) = target
                .rsplit('/')
                .next()
                .filter(|_| target.contains("media/"))
            {
                if resolved.exists() {
                    continue;
                }
                assert!(
                    root.join("brand").join(mark).is_file(),
                    "site/{page} references media/{mark}, absent from brand/"
                );
                continue;
            }
            assert!(
                resolved.exists(),
                "site/{page} links `{target}`, which does not resolve"
            );
        }
    }

    for needed in required {
        assert!(
            linked_repo_files.iter().any(|linked| linked == needed),
            "the site must link {needed}; linked: {linked_repo_files:?}"
        );
        assert!(
            root.join(needed).exists(),
            "{needed} is linked by the site but missing from the repository"
        );
    }
}

// Scenario: Literal de versión rehusado
// Scenario: Nombre de artefacto inexistente rehusado
// Scenario: Verificación alcanzable desde la descarga
#[test]
fn downloads_are_version_free_and_name_real_artifacts() {
    let site = site_dir();
    let base = canonical_base();
    let release_yml = read(&repo_root().join(".github/workflows/release.yml"));
    let release_doc = read(&repo_root().join("docs/release.md"));

    // The stable artifact names the site is allowed to offer, taken from the
    // release documentation's table so there is one list, not two.
    let artifacts: Vec<String> = release_doc
        .lines()
        .filter(|line| line.starts_with('|'))
        .flat_map(|line| line.split('`').skip(1).step_by(2))
        .filter(|name| name.starts_with("meltemi-"))
        .map(str::to_string)
        .collect();
    assert!(
        artifacts.len() >= 5,
        "docs/release.md must table the stable artifact names, found {artifacts:?}"
    );

    for page in ["downloads.html", "es/downloads.html"] {
        let html = read(&site.join(page));
        for (index, _) in html.match_indices("releases/latest/download/") {
            let tail = &html[index..];
            // A link ends at its quote; a URL inside a code block ends at
            // whitespace or the tag that follows it.
            let asset: String = tail
                .trim_start_matches("releases/latest/download/")
                .chars()
                .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '<' | ')'))
                .collect();
            let known = artifacts.contains(&asset)
                || ["SHA256SUMS", "install.sh", "install.ps1"].contains(&asset.as_str());
            assert!(
                known,
                "site/{page} offers `{asset}`, which the release pipeline does not publish"
            );
            // Every published name must be produced or renamed by the pipeline.
            if asset.starts_with("meltemi-") {
                assert!(
                    release_yml.contains(&asset),
                    "site/{page} offers `{asset}`; .github/workflows/release.yml never names it"
                );
            }
        }
        // Every download URL — linked or shown inside a command — hangs off the
        // one canonical base.
        let mut rest = html.as_str();
        while let Some(at) = rest.find("releases/latest/download/") {
            let before = &rest[..at];
            let start = before
                .rfind("https://")
                .expect("a download URL is absolute");
            let url = &before[start..];
            assert!(
                format!("{url}releases/latest/download/").starts_with(&base),
                "site/{page} downloads from `{url}`, not from the canonical base `{base}`"
            );
            rest = &rest[at + 1..];
        }
        // No version literal anywhere on the page.
        for token in ["/v0.", "/v1.", "meltemi-v", "version=v"] {
            assert!(
                !html.contains(token),
                "site/{page} contains the version literal `{token}`: use the latest-release URL"
            );
        }
        // Verification is reachable from the download page.
        assert!(
            html.contains("SHA256SUMS") && html.contains("docs/release.md"),
            "site/{page} must link the checksums and the verification procedure"
        );
    }
}

#[test]
fn the_canonical_base_is_the_only_one_the_installers_use() {
    // Scenario: Descarga sin conocer la versión (one base, one repository).
    let base = canonical_base();
    let repository = base.trim_end_matches("/releases").to_string();
    for script in ["scripts/install.sh", "scripts/install.ps1"] {
        let body = read(&repo_root().join(script));
        assert!(
            body.contains(&format!("{repository}/releases/download")),
            "{script} must download from the canonical base `{base}`"
        );
        // A stale second base would silently resolve elsewhere.
        assert!(
            !body.contains("github.com/meltemi/meltemi"),
            "{script} still names a non-canonical repository"
        );
    }
    for page in ["downloads.html", "es/downloads.html"] {
        let html = read(&site_dir().join(page));
        assert!(
            html.contains(&repository),
            "site/{page} must name the canonical repository"
        );
    }
}

// Scenario: Token divergente rehusado
#[test]
fn site_tokens_match_the_desktop_client_token_for_token() {
    let root = repo_root();
    let client = read(&root.join("desktop/ui/src/app.css"));
    let site = read(&site_dir().join("tokens.css"));

    /// `--name: value;` declarations of a stylesheet, first occurrence wins per
    /// scope-insensitive name (both sheets declare light first, dark second).
    fn tokens(css: &str) -> BTreeMap<String, Vec<String>> {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in css.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("--")
                && let Some((name, value)) = rest.split_once(':')
            {
                let value = value.trim().trim_end_matches(';').trim().to_string();
                map.entry(format!("--{}", name.trim()))
                    .or_default()
                    .push(value);
            }
        }
        map
    }

    let client_tokens = tokens(&client);
    let site_tokens = tokens(&site);
    assert!(
        site_tokens.len() > 30,
        "the site must declare the design system's tokens, found {}",
        site_tokens.len()
    );
    for (name, site_values) in &site_tokens {
        let client_values = client_tokens
            .get(name)
            .unwrap_or_else(|| panic!("site token {name} does not exist in the desktop client"));
        // The site declares light and dark; the client may also declare the
        // explicit theme overrides. Every site value must appear in the client.
        for value in site_values {
            assert!(
                client_values.contains(value),
                "token {name} diverged: site has `{value}`, client has {client_values:?}"
            );
        }
    }
}

// Scenario: Bloque copiado de la documentación rehusado
#[test]
fn no_page_copies_a_block_of_documentation() {
    const WINDOW: usize = 6;
    let site = site_dir();
    let docs = repo_root().join("docs");
    let mut doc_windows: BTreeMap<String, String> = BTreeMap::new();
    let mut stack = vec![docs];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read docs/") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|e| e == "md") {
                let text = read(&path);
                let lines: Vec<&str> = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect();
                for window in lines.windows(WINDOW) {
                    doc_windows.insert(
                        window.join("\n"),
                        path.strip_prefix(repo_root())
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    );
                }
            }
        }
    }

    for page in site_pages() {
        let html = read(&site.join(&page));
        let lines: Vec<&str> = html
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        for window in lines.windows(WINDOW) {
            let block = window.join("\n");
            if let Some(source) = doc_windows.get(&block) {
                panic!("site/{page} copies {WINDOW} lines from {source}:\n{block}");
            }
        }
    }
}

// Scenario: Captura sin alternativa textual rehusada
// Scenario: Procedencia declarada por captura
#[test]
fn every_figure_carries_alt_text_and_provenance() {
    let site = site_dir();
    let mut figures = 0;
    for page in site_pages() {
        let html = read(&site.join(&page));
        for (index, _) in html.match_indices("<figure") {
            figures += 1;
            let tail = &html[index..];
            let end = tail.find("</figure>").expect("figure closes");
            let figure = &tail[..end];
            let alt_start = figure
                .find("alt=\"")
                .unwrap_or_else(|| panic!("a figure in site/{page} has no alt text"));
            let alt: String = figure[alt_start + 5..]
                .chars()
                .take_while(|c| *c != '"')
                .collect();
            assert!(
                alt.split_whitespace().count() >= 8,
                "the alt text of a figure in site/{page} is not descriptive: {alt:?}"
            );
            let caption_start = figure
                .find("<figcaption>")
                .unwrap_or_else(|| panic!("a figure in site/{page} declares no provenance"));
            let caption = &figure[caption_start..];
            // Provenance: surface, capture source and product state.
            for expected in ["fixture", "Meltemi"] {
                assert!(
                    caption.contains(expected),
                    "the caption of a figure in site/{page} must declare its provenance ({expected})"
                );
            }
        }
    }
    assert!(figures >= 2, "both language trees must show the surfaces");
}

// Scenario: Sin nombres de terceros como argumento de venta
#[test]
fn no_third_party_product_name_appears_in_the_copy() {
    // The same rule the README carries: third-party products are named only in
    // interoperability data (the agents guide), never as a selling point.
    let forbidden = [
        "Claude",
        "Codex",
        "Copilot",
        "Gemini",
        "Cursor",
        "OpenAI",
        "Anthropic",
        "VS Code",
        "JetBrains",
        "Aider",
        "OpenCode",
    ];
    for page in site_pages() {
        let html = read(&site_dir().join(&page));
        for name in forbidden {
            assert!(
                !html.contains(name),
                "site/{page} names the third-party product `{name}`"
            );
        }
    }
}

#[test]
fn the_site_declares_its_domain_and_privacy_posture() {
    let site = site_dir();
    let cname = read(&site.join("CNAME")).trim().to_string();
    assert!(
        !cname.is_empty() && !cname.contains(' ') && cname.contains('.'),
        "site/CNAME must hold the site's domain, found {cname:?}"
    );
    for page in site_pages() {
        let html = read(&site.join(&page));
        assert!(
            html.contains("no cookies") || html.contains("sin cookies"),
            "site/{page} must state the privacy posture in its footer"
        );
    }
}
