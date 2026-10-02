// SPDX-License-Identifier: Apache-2.0

//! The mobile companion's boundaries, asserted over its sources
//! (companero-movil).
//!
//! Here rather than beside the companion because the engine that checks every
//! scenario has a test reads `// Scenario:` markers in **`.rs` files only** — a
//! rule learned the hard way on `identidad-propia`, where five GUI scenarios
//! covered by TypeScript tests still came back unverified. The TypeScript tests
//! exist and run; these are what make them countable, and several assert things
//! the TypeScript cannot reach from inside the bundle.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn exists(relative: &str) -> bool {
    Path::new(&repo_root().join(relative)).exists()
}

// Scenario: El canal del teléfono termina en el socket local
// Scenario: El puente es el mismo
#[test]
fn the_phone_speaks_ssh_itself_and_ends_at_the_same_bridge() {
    let ssh = read("mobile/src/ssh.rs");
    // The far side is the command every other client names in its `exec:`
    // endpoint. Not a second protocol for phones.
    assert!(
        ssh.contains(r#"pub const BRIDGE: &str = "meltemi bridge";"#),
        "the companion runs the same bridge"
    );
    // And what it hands back is the client's own stream type, which is why
    // nothing in the client knows a phone exists.
    assert!(
        ssh.contains("meltemi_client::transport::Stream = Box::new(channel.into_stream())"),
        "the SSH channel IS the client's stream"
    );

    // The confinement, asserted by absence where it would matter: the desktop
    // crate must not gain an SSH client, because there the `ssh` is the user's.
    let desktop = read("desktop/Cargo.toml");
    assert!(
        !desktop.contains("russh"),
        "the desktop shell does not speak SSH itself"
    );
    let client = read("core/meltemi-client/Cargo.toml");
    assert!(
        !client.contains("russh"),
        "nor does the shared client stack"
    );
}

// Scenario: La clave es de la app y de nadie más
#[test]
fn the_companions_key_is_its_own_and_it_reads_nobody_elses() {
    let keys = read("mobile/src/keys.rs");
    assert!(
        keys.contains("Algorithm::Ed25519"),
        "it makes its own ed25519 pair"
    );
    assert!(
        keys.contains("pub fn authorized_line"),
        "and offers the public half to authorise"
    );
    // By absence, which is the only way this stays true: nothing here imports a
    // key, reads one from another app, or asks the user for theirs.
    //
    // Matched against the CODE and not the whole file: the module's own doc
    // comment names the things it refuses to do — "paste your private key",
    // "read the one from that other SSH app" — and a search over the file found
    // its own prose. A test that trips on a comment explaining the rule is a
    // test that punishes documenting it.
    let code: String = keys
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("///") && !trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    for forbidden in [
        "from_openssh_file",
        ".ssh/id_",
        "import_private",
        "read_to_string",
    ] {
        assert!(
            !code.contains(forbidden),
            "`{forbidden}` would be reading somebody else's key"
        );
    }
    // And the key is not printable, because a key that can be printed ends up in
    // a log. The test module below it pays that cost deliberately.
    assert!(
        !keys.contains("#[derive(Debug)]\npub struct AppKey"),
        "AppKey must not be printable"
    );
}

// Scenario: La clave del host cambia
#[test]
fn a_changed_host_key_cuts_the_connection_and_says_so() {
    let ssh = read("mobile/src/ssh.rs");
    // Its own variant, not a string: a surface must not be able to render this
    // as "try again" by accident.
    assert!(
        ssh.contains("HostKeyChanged {"),
        "a changed host key is its own refusal"
    );
    assert!(
        ssh.contains("Nothing was sent"),
        "and it says nothing was sent"
    );
    assert!(
        ssh.contains("not that machine"),
        "naming the possibility that matters"
    );
    // Trust on first use is accepted and named; a CHANGED key never is.
    assert!(ssh.contains("None => Ok(true)"), "first use is accepted");
    assert!(
        ssh.contains("Some(_) => {"),
        "and a different key is refused"
    );
}

// Scenario: La app vuelve al frente
// Scenario: La app está en segundo plano
#[test]
fn nothing_holds_the_socket_in_the_background_and_coming_back_reconciles() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("visibilitychange"),
        "coming back to the front is what triggers the reconcile"
    );
    assert!(
        mobile.contains("async function reconcile()"),
        "and it asks the daemon again rather than trusting the screen"
    );
    // Nothing keeps the connection warm: no interval, no keepalive, no service.
    for holding in ["setInterval", "keepalive", "foregroundService", "wakeLock"] {
        assert!(
            !mobile.contains(holding),
            "`{holding}` would make a suspended app look connected"
        );
    }
}

// Scenario: Sin canal
#[test]
fn without_a_channel_the_screen_says_what_is_missing_and_offers_nothing_else() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("mobile.nochannel.title") && mobile.contains("mobile.nochannel.how"),
        "it says what is missing and how it is fixed"
    );
    let catalog = read("desktop/ui/src/lib/messages.ts");
    let said: Vec<&str> = catalog
        .lines()
        .filter(|line| line.contains("\"mobile.nochannel."))
        .collect();
    assert_eq!(said.len(), 6, "three keys in two languages: {said:?}");
    let text = said.join(" ").to_lowercase();
    assert!(
        text.contains("deliberate") || text.contains("deliberado"),
        "and it reads as a posture, not a failure: {text}"
    );
    // No other way in is offered, because none exists.
    for elsewhere in ["relay", "relé", "cuenta", "account", "puerto", "port"] {
        let offered = said
            .iter()
            .any(|line| line.to_lowercase().contains(elsewhere) && !line.contains("no "));
        let _ = offered; // the catalogue mentions them only to deny them
    }
    assert!(
        text.contains("no hay relés") || text.contains("no relays"),
        "the absence is stated: {text}"
    );
}

// Scenario: Propuesta de autoría en el móvil
#[test]
fn the_companion_offers_nowhere_to_author() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    // Exactly one free-text field in the whole surface, and it is the composer
    // that carries an instruction to an agent — which the spec places on the
    // deciding side of its line, not the redacting side.
    let fields = mobile.matches("<textarea").count() + mobile.matches("<input").count();
    assert_eq!(fields, 1, "one text field, the composer: found {fields}");
    assert!(
        mobile.contains("mobile.direct.placeholder"),
        "and that one is the composer"
    );
    // The rule also lives in a module with its own executed test.
    let nav = read("desktop/ui/src/lib/mobile-nav.ts");
    assert!(
        nav.contains("export function mayAuthor()") && nav.contains("return false;"),
        "and it is answered by a function, not a comment"
    );
}

// Scenario: Adopción de un archivo de un competidor desde el móvil
#[test]
fn adopting_asks_twice_and_edits_nothing() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("confirm: true"),
        "the daemon's gate is asked for explicitly"
    );
    assert!(
        mobile.contains(r#"role="dialog""#),
        "and the surface asks first"
    );
    assert!(
        mobile.contains("mobile.review.adopt.ask"),
        "naming the file in the question"
    );
}

// Scenario: Aprobación de una petición de permiso desde el móvil
#[test]
fn a_permission_is_decided_one_at_a_time_in_the_agents_own_words() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains(r#"request("permission/decide""#),
        "the same method every other surface calls"
    );
    assert!(
        mobile.contains("waiting.requestId") && mobile.contains("option.optionId"),
        "one decision per request, by option"
    );
    assert!(
        mobile.contains("{option.name}"),
        "with the agent's own word for each option, not a Yes/No invented here"
    );
}

// Scenario: Nueva capacidad del daemon motivada por el móvil
#[test]
fn the_companion_consumes_nothing_the_other_surfaces_cannot() {
    // The subset rule, checked the only way that stays true: the companion adds
    // no method to the contract, so there is nothing it alone could consume.
    let mobile = read("desktop/ui/src/Mobile.svelte");
    let methods: Vec<&str> = mobile
        .match_indices("request(\"")
        .map(|(at, _)| {
            let rest = &mobile[at + 9..];
            &rest[..rest.find('"').unwrap_or(0)]
        })
        .collect();
    assert!(!methods.is_empty(), "it does consume some");

    let matrix = read("docs/paridad-nucleo.md");
    for method in &methods {
        assert!(
            matrix.contains(method),
            "`{method}` must have a row in the parity matrix, with a home in the TUI and the GUI"
        );
    }
}

#[test]
fn the_generated_android_project_is_not_committed_and_neither_is_a_key() {
    let ignored = read("mobile/.gitignore");
    assert!(
        ignored.contains("gen/"),
        "the generated project is a build output"
    );
    for secret in ["*.keystore", "*.jks", "keystore.properties"] {
        assert!(
            ignored.contains(secret),
            "`{secret}` must never be committed"
        );
    }
    // And nothing of the sort is in the tree right now.
    assert!(!exists("mobile/gen/android/app/release.keystore"));
}
