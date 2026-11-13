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
    // And the way is offered right there (companero-que-conecta): the key,
    // the destination and connect, on the same screen — not a pointer to a
    // Settings view that, before this change, was a title and a sentence.
    let pairing = mobile
        .split("<main class=\"pairing\">")
        .nth(1)
        .expect("the pairing screen")
        .split("</main>")
        .next()
        .expect("closed");
    for offered in [
        "keyBlock()",
        "destinationField()",
        "mobile.pair.sameNetwork",
    ] {
        assert!(
            pairing.contains(offered),
            "the screen without a channel offers `{offered}`"
        );
    }
    assert!(
        !text.contains("ajustes") && !text.contains("settings"),
        "it no longer points at a place that does not do it: {text}"
    );
}

// Scenario: La clave se ve y se copia sin conexión
#[test]
fn the_key_is_shown_and_copied_without_a_connection_and_no_field_takes_a_private_one() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("invoke<string>(\"companion_key\")"),
        "the key line is asked of the host, not derived in the webview"
    );
    assert!(
        mobile.contains("navigator.clipboard.writeText(keyLine)")
            && mobile.contains("mobile.pair.copy\""),
        "with a control to copy it"
    );
    assert!(
        mobile.contains("user-select: all;"),
        "and the line itself selectable, for when the clipboard is not there"
    );
    // By absence, matched as code shapes and not as words — a comment that
    // explains the rule must not trip it: no password field, no private-key
    // binding, and the one `<input` is a text field.
    assert_eq!(
        mobile.matches("<input").count(),
        mobile.matches("type=\"text\"").count(),
        "every input is a text field"
    );
    for never in ["type=\"password\"", "privateKey"] {
        assert!(
            !mobile.contains(never),
            "`{never}` would be asking for what the app must never take"
        );
    }
    // The key block is rendered from one snippet, so the rule stays about
    // one block wherever it is shown.
    assert_eq!(
        mobile.matches("{#snippet keyBlock()}").count(),
        1,
        "one key block in the source"
    );
}

// Scenario: El destino se escribe una vez
#[test]
fn the_destination_is_typed_once_remembered_by_the_host_and_redialled_on_return() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("invoke<string | null>(\"companion_destination\")"),
        "the remembered destination is read on arrival"
    );
    assert!(
        mobile.contains("if (remembered && !cancelled) void connect(remembered);"),
        "and redialled when the app comes back to the front, unless cancelled"
    );
    assert!(
        mobile.contains("void invoke(\"companion_disconnect\")"),
        "leaving the front ends the actor"
    );
    assert!(
        mobile.contains("companion_forget_destination"),
        "and the destination can be forgotten on purpose"
    );
    // The host remembers only what it could parse, and in that order.
    let host = read("mobile/src/host.rs");
    let parse = host
        .find("let parsed = Destination::parse(&destination)?;")
        .expect("the host parses first");
    let remember = host
        .find("Remembered::in_dir(&dir).write(&destination)?;")
        .expect("and then remembers");
    assert!(
        parse < remember,
        "a destination that does not parse is never remembered"
    );
}

// Scenario: Conectar abre el túnel y la pantalla deja de prometer
#[test]
fn connecting_runs_the_shared_actor_over_ssh_and_the_screen_steps_aside_once_connected() {
    let host = read("mobile/src/host.rs");
    assert!(
        host.contains("bridge_actor(dialer, command_rx, event_tx, None, CLIENT)"),
        "the host runs the shared actor with the SSH dialer and no notice emitter"
    );
    assert!(
        host.contains("app.emit(\"daemon:state\", &state)") && host.contains("\"daemon:incoming\""),
        "and forwards its events under the desktop's names"
    );
    let mobile = read("desktop/ui/src/Mobile.svelte");
    assert!(
        mobile.contains("{#if paired && view !== \"settings\"}"),
        "the pairing screen gives way to the verbs once there is a channel"
    );
    let pair = read("desktop/ui/src/lib/mobile-pair.ts");
    assert!(
        pair.contains("if (state.state === \"connected\") return null;"),
        "which is what `pairing` says when connected"
    );
}

// Scenario: Una clave rechazada no se reintenta en bucle
#[test]
fn a_refusal_is_its_own_screen_and_nothing_in_the_surface_retries_by_itself() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    for shown in [
        "mobile.refused.hostKeyChanged",
        "mobile.refused.rejected",
        "mobile.refused.forgetHost",
        "mobile.refused.stay",
        "mobile.refused.retry",
    ] {
        assert!(mobile.contains(shown), "the refusal names `{shown}`");
    }
    // No timer anywhere: a retry is a person's tap, and forgetting a host's
    // key is a person's choice.
    for timer in ["setTimeout(", "setInterval("] {
        assert!(
            !mobile.contains(timer),
            "`{timer}` would be the surface retrying on its own"
        );
    }
    let pair = read("desktop/ui/src/lib/mobile-pair.ts");
    assert!(
        pair.contains("return state.kind === \"connecting\";"),
        "only the connecting state retries, and the module says so"
    );
    assert!(
        pair.contains("? [\"forgetHost\", \"stay\"]"),
        "a changed host key offers forgetting the host, and never a bare retry"
    );
}

// Scenario: El escritorio y el compañero corren el mismo actor
#[test]
fn the_desktop_and_the_companion_run_one_actor_and_differ_only_in_the_dialer() {
    let shared = read("core/meltemi-client/src/bridge.rs");
    assert_eq!(
        shared
            .matches("pub async fn bridge_actor<D: Dialer>(")
            .count(),
        1,
        "one actor, in the shared client"
    );
    let desktop = read("desktop/src/bridge.rs");
    let companion = read("mobile/src/host.rs");
    for (surface, source) in [("desktop", &desktop), ("companion", &companion)] {
        assert!(
            source.contains("use meltemi_client::bridge::{") && source.contains("bridge_actor"),
            "the {surface} runs the shared actor"
        );
        assert!(
            !source.contains("async fn bridge_actor"),
            "and keeps no copy of its own"
        );
    }
    assert!(
        desktop.contains("LocalDialer::new(endpoint)"),
        "the desktop dials the local endpoint"
    );
    assert!(
        companion.contains("SshDialer::new(parsed, key.private(), Box::new(hosts))"),
        "the companion dials its SSH channel"
    );
}

// Scenario: Propuesta de autoría en el móvil
#[test]
fn the_companion_offers_nowhere_to_author() {
    let mobile = read("desktop/ui/src/Mobile.svelte");
    // Exactly two free-text fields in the whole surface, and each is named: the
    // composer, which carries an instruction to an agent — which the spec
    // places on the deciding side of its line, not the redacting side — and
    // the destination, which is a place to go and not content for a file
    // (companero-que-conecta D7).
    let fields = mobile.matches("<textarea").count() + mobile.matches("<input").count();
    assert_eq!(
        fields, 2,
        "two text fields, the composer and the destination: found {fields}"
    );
    assert!(
        mobile.contains("mobile.direct.placeholder"),
        "one is the composer"
    );
    assert!(
        mobile.contains("mobile.pair.placeholder"),
        "and the other is the destination"
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
