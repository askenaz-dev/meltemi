// SPDX-License-Identifier: Apache-2.0

//! The headless session dialect (adaptadores-propios-acp design D4).
//!
//! One provider ships no server mode at all: what it documents is a
//! **non-interactive session that speaks newline-delimited JSON events** over
//! stdio, running under the account the user already signed into. This module
//! is the translation between that surface and ACP.
//!
//! What is deliberately not here, and never will be:
//!
//! - **The provider's agent SDK.** The provider's own terms name it as not
//!   authorized for subscription sign-in, and the canonical third-party adapter
//!   wraps exactly that library. This adapter launches the official binary
//!   instead — the safe path the research named and nobody had taken.
//! - **Any mode that demands an API key.** The flag that skips the sign-in is
//!   announced to become the default of headless mode one day; when that
//!   arrives, this adapter refuses with a diagnosis rather than let a key
//!   quietly start paying for what a subscription was paying for (see
//!   [`surface`]).
//! - **Any undocumented channel.** The same wire carries the SDK's own control
//!   protocol; it is not documented, so nothing here hangs off it (design D7).

pub mod surface;
pub mod wire;

use std::path::Path;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    Meta, NewSessionRequest, PromptRequest, SessionId, SessionInfoUpdate, SessionNotification,
    SessionUpdate, StopReason,
};
use agent_client_protocol::{Client, ConnectionTo};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use crate::adapter::{AdapterSpec, Dialect, ProviderDialect, ProviderSession};
use crate::diagnostic::Refusal;
use crate::ndjson::Frame;
use crate::supervisor::{
    ProcessControl, ProviderCommand, ProviderProcess, ShutdownPolicy, SpawnedProvider,
    launch_refusal, resolve_program, spawn,
};

use surface::Surface;

/// What this binary announces over ACP and which CLI it pilots.
pub const SPEC: AdapterSpec = AdapterSpec {
    name: "meltemi-claude-acp",
    provider_layer: "the official `claude` CLI",
    provider_bin: "claude",
    dialect: Dialect::HeadlessSession,
};

/// Points the adapter at a specific CLI binary instead of the one on the PATH.
///
/// It is how the end-to-end tests put a scripted wire where the provider would
/// be, and an escape hatch for a CLI installed somewhere the PATH does not
/// reach. It changes *which binary* is launched, never *what* is spoken to it.
pub const PROVIDER_BIN_ENV: &str = "MELTEMI_CLAUDE_BIN";

/// How long the initial event may take to arrive before the session is refused.
///
/// Generous, because this CLI runs on a language runtime that can take seconds
/// to come up cold on Windows and may boot MCP servers before it speaks;
/// finite, because a session that never opens and never fails is the worst of
/// both.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the CLI gets to answer what version it is.
const VERSION_TIMEOUT: Duration = Duration::from_secs(30);

/// What the session log says when the CLI would not say what version it is.
const UNKNOWN_VERSION: &str = "unknown";

/// The headless session dialect: launches the official CLI in its documented
/// non-interactive mode and pilots it.
pub struct ClaudeDialect {
    /// The binary to launch, after the environment override.
    program: String,
    /// How long a provider gets to exit on its own when a session closes.
    shutdown: ShutdownPolicy,
}

impl Default for ClaudeDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeDialect {
    /// The dialect as the binary runs it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: resolve_program(std::env::var(PROVIDER_BIN_ENV).ok(), SPEC.provider_bin),
            shutdown: ShutdownPolicy::default(),
        }
    }
}

/// What was actually launched and what it turned out to be: the facts a session
/// must be able to prove afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The binary the adapter launched, as it was resolved.
    pub program: String,
    /// The version the CLI answered with, or [`UNKNOWN_VERSION`].
    pub version: String,
    /// The feature-detection array the CLI announced, verbatim.
    pub capabilities: Vec<String>,
    /// Where the CLI said its credentials come from.
    pub key_source: String,
    /// The model the session runs, when the CLI named one.
    pub model: Option<String>,
    /// The CLI's own id for the session, which is what a resume names.
    pub provider_session: Option<String>,
}

impl Provenance {
    /// The provenance as ACP extension metadata.
    ///
    /// It travels in the `_meta` of a session update, which is the protocol's
    /// own extensibility point — not a private channel between this adapter and
    /// meltemid (design D7). The daemon records agent updates verbatim, so this
    /// lands in the session log like any other, and any third-party agent can
    /// say the same thing the same way.
    ///
    /// The announced surface travels with it on purpose: which credential the
    /// CLI came up on is the fact a human most needs on the record, and the one
    /// nobody can reconstruct afterwards.
    #[must_use]
    pub fn meta(&self) -> Meta {
        let mut meta = Meta::new();
        meta.insert(
            "meltemi".into(),
            json!({
                "providerBin": self.program,
                "providerVersion": self.version,
                "providerCapabilities": self.capabilities,
                "providerKeySource": self.key_source,
                "providerModel": self.model,
                "providerSession": self.provider_session,
            }),
        );
        meta
    }
}

/// A session over the official CLI's headless mode.
pub struct ClaudeSession {
    /// The provider process this session pilots. `None` once it has been shut
    /// down.
    provider: Mutex<Option<SpawnedProvider>>,
}

impl ProviderSession for ClaudeSession {
    async fn run_turn(&self, _prompt: PromptRequest) -> Result<StopReason, Refusal> {
        Err(Refusal::new(
            "dialect_not_wired",
            SPEC.provider_layer,
            "this adapter opens the session but does not translate turns yet".to_string(),
            "The turn mapping lands in adaptadores-propios-acp 3.2; until then pilot the agent \
             with another entry of the fleet."
                .to_string(),
        ))
    }

    async fn interrupt(&self) {
        // Nothing runs a turn yet, so nothing has to be interrupted. What
        // cancellation will mean on this wire is decided with the turn loop
        // (task 3.2), and it is not obvious: the only interruption this CLI
        // documents for a headless session is the end of its input.
    }

    async fn shutdown(&self, policy: ShutdownPolicy) {
        if let Some(mut provider) = self.provider.lock().await.take() {
            // Closing the input is what ends a headless session; the grace and
            // the kill are for a provider that ignores it.
            //
            // Worth knowing where this does *not* run: the daemon usually ends
            // an adapter by killing it, and a killed process runs no cleanup at
            // all. What saves this dialect there is the operating system —
            // the CLI's stdin is a pipe whose only writer is this process, so
            // the kill closes it and the CLI sees end of input on its own. The
            // residual risk is a provider that ignores EOF, and ending *that*
            // with certainty takes a job object on Windows and a process group
            // elsewhere; that stronger form belongs with `sandbox-propio`.
            let _ = provider.shutdown(policy).await;
        }
    }
}

impl ProviderDialect for ClaudeDialect {
    type Session = ClaudeSession;

    fn spec(&self) -> AdapterSpec {
        SPEC
    }

    async fn open(
        &self,
        session_id: SessionId,
        request: NewSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> Result<Self::Session, Refusal> {
        // Asked before anything is piloted: a binary that cannot answer what it
        // is cannot be piloted either, and the answer is what the session log
        // will hold.
        let version = announced_version(&self.program, &request.cwd).await?;

        let command = ProviderCommand {
            program: self.program.clone(),
            args: wire::session_args(),
            cwd: request.cwd.clone(),
        };
        let mut provider = spawn(&command, SPEC.provider_layer)?;

        // From here on the process exists, so every refusal has to end it. A
        // session that failed to open and left a CLI running would leak one
        // process per attempt, holding the worktree it was launched in.
        let init = match handshake(&mut provider).await {
            Ok(init) => init,
            Err(refusal) => {
                let _ = provider.shutdown(self.shutdown).await;
                return Err(refusal);
            }
        };

        let announced = Surface::of(&init);
        if let Err(refusal) = announced.check(SPEC.provider_layer) {
            let _ = provider.shutdown(self.shutdown).await;
            return Err(refusal);
        }
        if let Some(note) = announced.note() {
            eprintln!("{}: {note}", SPEC.name);
        }

        // The effective binary, the version it turned out to be and the surface
        // it announced, into the session log before anything else this session
        // does.
        let provenance = Provenance {
            program: self.program.clone(),
            version,
            capabilities: announced.capabilities.clone(),
            key_source: key_source_label(&announced),
            model: init.model.clone(),
            provider_session: init.session_id.clone(),
        };
        let _ = cx.send_notification(SessionNotification::new(
            session_id,
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta())),
        ));

        Ok(ClaudeSession {
            provider: Mutex::new(Some(provider)),
        })
    }
}

/// How the announced credential source reads in the session log.
fn key_source_label(announced: &Surface) -> String {
    match &announced.key_source {
        surface::KeySource::SignedIn => wire::SIGNED_IN_KEY_SOURCE.to_string(),
        surface::KeySource::Key(source) => source.clone(),
        surface::KeySource::Unannounced => "unannounced".to_string(),
    }
}

/// Asks the CLI what version it is.
///
/// This wire announces no version of its own — the initial event carries a
/// feature array, not a number — so the version the session log records is the
/// one the binary answers to its documented version flag. Both of its output
/// streams are read, because which one a CLI prints its version on is a detail
/// no adapter should be brittle about.
///
/// # Errors
///
/// Refuses when the binary cannot be launched at all: that is the missing layer
/// the fleet catalog names, and no session can follow it.
async fn announced_version(program: &str, cwd: &Path) -> Result<String, Refusal> {
    let asked = tokio::process::Command::new(program)
        .arg(wire::VERSION)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let Ok(answer) = tokio::time::timeout(VERSION_TIMEOUT, asked).await else {
        // A CLI too busy to say what it is will not run a session either, but
        // saying so is the caller's business: this is not the layer's absence.
        return Ok(UNKNOWN_VERSION.to_string());
    };
    let answer = answer.map_err(|error| launch_refusal(program, SPEC.provider_layer, &error))?;

    let said = [answer.stdout, answer.stderr]
        .into_iter()
        .find_map(|stream| {
            let text = String::from_utf8_lossy(&stream).trim().to_string();
            (!text.is_empty()).then_some(text)
        })
        .unwrap_or_default();
    Ok(version_in(&said).unwrap_or(UNKNOWN_VERSION.to_string()))
}

/// The version token inside whatever the CLI printed (`2.1.4 (Claude Code)`).
///
/// Kept forgiving on purpose: the surrounding words are the provider's to
/// change, and a session that failed because a product name moved would be
/// failing for nothing. A pre-release or build suffix stays part of the answer —
/// it is the version the CLI calls itself, and the log records what was said,
/// not a tidied version of it. What is *not* guessed is the number: when no
/// token looks like one, the log says so rather than invent it.
fn version_in(said: &str) -> Option<String> {
    said.lines().next()?.split_whitespace().find_map(|token| {
        let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let mut parts = token.split(['-', '+']).next()?.split('.');
        let looks_like_a_version = parts.clone().count() >= 2
            && parts.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
        looks_like_a_version.then(|| token.to_string())
    })
}

/// Reads the session's initial event: the handshake of this dialect.
///
/// # Errors
///
/// Refuses when the CLI says nothing, takes too long, ends before announcing
/// itself, or refuses the session outright — in which case the provider's own
/// words travel to the human unchanged, because an authentication failure is
/// the provider's to explain and this adapter has nothing to add to it.
async fn handshake<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
) -> Result<wire::InitEvent, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    tokio::time::timeout(HANDSHAKE_TIMEOUT, await_init(provider))
        .await
        .map_err(|_| {
            handshake_failed(&format!(
                "the CLI did not announce the session within {} seconds",
                HANDSHAKE_TIMEOUT.as_secs()
            ))
        })?
}

/// Reads until the initial event arrives.
async fn await_init<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
) -> Result<wire::InitEvent, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    loop {
        let frame = provider.receive().await.map_err(|error| {
            handshake_failed(&format!("the CLI's output could not be read ({error})"))
        })?;
        let Some(frame) = frame else {
            return Err(handshake_failed(
                "the CLI ended without announcing the session",
            ));
        };
        let event = match frame {
            Frame::Json(value) => value,
            // Not JSON at all: a banner or a warning. Kept on the adapter's own
            // stderr, where the daemon already collects the provider's noise,
            // and never mistaken for protocol.
            Frame::Unparsed(line) => {
                eprintln!("{}: provider said: {line}", SPEC.name);
                continue;
            }
        };
        if wire::is_init(&event) {
            return serde_json::from_value(event).map_err(|error| {
                handshake_failed(&format!(
                    "the CLI announced a session this adapter cannot read ({error})"
                ))
            });
        }
        if let Some(refused) = refusal_before_the_session(&event) {
            return Err(refused);
        }
        eprintln!("{}: provider said: {event}", SPEC.name);
    }
}

/// The provider's own refusal, when it ends the session before announcing it.
///
/// This is where an authentication failure lands, and the provider's message is
/// passed through verbatim: the CLI owns the sign-in, so it owns the
/// explanation too. The adapter adds a remedy and no interpretation.
fn refusal_before_the_session(event: &Value) -> Option<Refusal> {
    if event.get("type").and_then(Value::as_str) != Some(wire::RESULT) {
        return None;
    }
    let said = event
        .get("result")
        .and_then(Value::as_str)
        .or_else(|| event.get("subtype").and_then(Value::as_str))
        .unwrap_or("the CLI ended the session without saying why");
    Some(Refusal::new(
        "provider_refused_session",
        SPEC.provider_layer,
        format!("the CLI ended the session before it began: {said}"),
        "The message above is the provider's own. Run the CLI by hand once to see it in full \
         and to sign in if that is what it is asking for."
            .to_string(),
    ))
}

/// The refusal for a session the CLI never announced.
fn handshake_failed(detail: &str) -> Refusal {
    Refusal::new(
        "provider_handshake_failed",
        SPEC.provider_layer,
        detail.to_string(),
        format!(
            "Check that the official CLI is installed and signed in, and that `{} {}` starts a \
             headless session (`meltemi fleet` shows the entry and its remedy).",
            SPEC.provider_bin,
            wire::session_args().join(" ")
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A provider process that plays a script, so a whole handshake runs in
    /// memory: no binary, no pipes, same behaviour on the three platforms.
    struct FakeProcess;

    impl ProcessControl for FakeProcess {
        async fn wait_within(&mut self, _grace: Duration) -> std::io::Result<bool> {
            Ok(true)
        }

        async fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Runs a handshake against a CLI that says exactly `lines`, and hands back
    /// the outcome together with everything the adapter sent.
    async fn handshake_against(lines: &str) -> (Result<wire::InitEvent, Refusal>, String) {
        let (adapter_out, mut cli_in) = tokio::io::duplex(4096);
        let (mut cli_out, adapter_in) = tokio::io::duplex(4096);
        tokio::io::AsyncWriteExt::write_all(&mut cli_out, lines.as_bytes())
            .await
            .unwrap();
        drop(cli_out);

        let mut provider = ProviderProcess::new(FakeProcess, adapter_out, adapter_in);
        let outcome = handshake(&mut provider).await;
        drop(provider);

        let mut sent = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut cli_in, &mut sent)
            .await
            .unwrap();
        (outcome, sent)
    }

    const SIGNED_IN: &str = concat!(
        r#"{"type":"system","subtype":"init","session_id":"s-9","model":"mock-sonnet","#,
        r#""apiKeySource":"none","capabilities":["interrupt_receipt_v1"],"tools":["Read"]}"#,
        "\n"
    );

    #[tokio::test]
    async fn the_handshake_reads_the_surface_the_cli_announces() {
        // Scenario: Versión efectiva registrada en el log
        let (outcome, sent) = handshake_against(SIGNED_IN).await;
        let init = outcome.expect("the signed-in surface opens the session");
        assert_eq!(init.session_id.as_deref(), Some("s-9"));

        let announced = Surface::of(&init);
        assert_eq!(announced.key_source, surface::KeySource::SignedIn);
        assert!(announced.announces("interrupt_receipt_v1"));
        assert!(
            sent.is_empty(),
            "the handshake is something the CLI says, not something it is asked: {sent}"
        );
    }

    #[tokio::test]
    async fn a_cli_that_would_run_on_a_key_is_refused_and_nothing_is_injected() {
        // Scenario: Rehúso ante modo que exige clave de API
        //
        // The pinned risk of design D4, end to end at the handshake: the CLI
        // comes up on a key instead of the session the user signed into. The
        // adapter refuses, and the one thing it must never do — send something
        // to make the session work anyway — is asserted by what it wrote: not a
        // byte.
        let (outcome, sent) = handshake_against(concat!(
            r#"{"type":"system","subtype":"init","apiKeySource":"ANTHROPIC_API_KEY","#,
            r#""capabilities":[]}"#,
            "\n"
        ))
        .await;
        let init = outcome.expect("the event itself is readable");
        let refusal = Surface::of(&init)
            .check(SPEC.provider_layer)
            .expect_err("a key-bearing surface is not the one this adapter pilots");
        assert_eq!(refusal.kind, "provider_surface_not_signed_in");
        assert!(refusal.detail.contains("ANTHROPIC_API_KEY"));
        assert!(sent.is_empty(), "nothing was injected: {sent}");
    }

    #[tokio::test]
    async fn a_cli_that_ends_before_the_session_hands_its_own_words_to_the_human() {
        // Scenario: La autenticación queda en el binario oficial
        //
        // The sign-in belongs to the CLI, so its explanation does too: the
        // adapter passes the message through and adds a remedy, never an
        // interpretation and never a credential.
        let (outcome, sent) = handshake_against(concat!(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"#,
            r#""result":"Invalid API key · Please run /login"}"#,
            "\n"
        ))
        .await;
        let refusal = outcome.expect_err("a session that never began cannot be piloted");
        assert_eq!(refusal.kind, "provider_refused_session");
        assert!(
            refusal.detail.contains("Please run /login"),
            "the provider's own words travel unchanged: {}",
            refusal.detail
        );
        assert!(sent.is_empty(), "nothing was injected: {sent}");
    }

    #[tokio::test]
    async fn a_cli_that_announces_nothing_refuses_rather_than_opening_a_session() {
        for said in [
            "",
            "starting up\n",
            "{\"type\":\"stream_event\",\"event\":{}}\n",
        ] {
            let (outcome, _) = handshake_against(said).await;
            let refusal = outcome.expect_err("a session cannot open on a handshake that failed");
            assert_eq!(refusal.kind, "provider_handshake_failed", "for `{said}`");
            assert!(
                refusal.remedy.contains(wire::STREAM_JSON),
                "the remedy names the surface that was expected: {}",
                refusal.remedy
            );
        }
    }

    #[tokio::test]
    async fn the_official_cli_missing_refuses_before_anything_is_piloted() {
        // Scenario: CLI oficial ausente al lanzar
        let refusal = announced_version("meltemi-no-such-provider-cli", &std::env::temp_dir())
            .await
            .expect_err("a CLI that does not exist cannot be asked what it is");
        assert_eq!(refusal.kind, "provider_cli_not_launchable");
        assert_eq!(refusal.layer, SPEC.provider_layer);
        assert!(refusal.remedy.contains("PATH"), "{}", refusal.remedy);
    }

    #[test]
    fn the_version_is_taken_from_what_the_cli_printed_and_never_invented() {
        assert_eq!(version_in("2.1.4 (Claude Code)"), Some("2.1.4".into()));
        assert_eq!(version_in("1.0"), Some("1.0".into()));
        assert_eq!(
            version_in("2.0.0-mock (mock-claude-wire)"),
            Some("2.0.0-mock".into()),
            "a pre-release suffix is part of what the CLI calls itself"
        );
        assert_eq!(
            version_in("2.1.4\nsomething else"),
            Some("2.1.4".into()),
            "the first line is the answer; the rest is the CLI talking"
        );
        for said in ["", "Claude Code", "nightly", "v-beta"] {
            assert_eq!(version_in(said), None, "`{said}` carries no version");
        }
    }

    #[test]
    fn the_provenance_travels_as_acp_extension_metadata() {
        // Scenario: Versión efectiva registrada en el log
        //
        // What the log will hold: which binary was launched, which version it
        // turned out to be, and — the fact nobody can reconstruct afterwards —
        // which credential the CLI came up on. Not a private channel: ACP's own
        // `_meta` (design D7), which any agent may fill the same way.
        let provenance = Provenance {
            program: "/usr/local/bin/claude".into(),
            version: "2.1.4".into(),
            capabilities: vec!["interrupt_receipt_v1".into()],
            key_source: wire::SIGNED_IN_KEY_SOURCE.into(),
            model: Some("sonnet".into()),
            provider_session: Some("s-9".into()),
        };
        let update =
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta()));
        let logged = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerBin"],
            "/usr/local/bin/claude"
        );
        assert_eq!(logged["_meta"]["meltemi"]["providerVersion"], "2.1.4");
        assert_eq!(logged["_meta"]["meltemi"]["providerKeySource"], "none");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerCapabilities"][0],
            "interrupt_receipt_v1"
        );
    }

    #[test]
    fn the_environment_override_decides_which_binary_is_launched() {
        assert_eq!(resolve_program(None, SPEC.provider_bin), SPEC.provider_bin);
        assert_eq!(
            resolve_program(Some("/tmp/mock-claude-wire".into()), SPEC.provider_bin),
            "/tmp/mock-claude-wire"
        );
    }
}
