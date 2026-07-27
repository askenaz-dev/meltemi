// SPDX-License-Identifier: Apache-2.0

//! The JSON-RPC server dialect (adaptadores-propios-acp design D6).
//!
//! One provider ships a documented server mode — JSON-RPC 2.0 with newline
//! delimitation over stdio, the same interface its own editor extension uses —
//! and this module is the translation between that surface and ACP. The
//! official binary is launched as a subprocess and nothing else: the pattern
//! the archived and community Rust adapters take, embedding the provider's
//! runtime as a library, would put the network and the provider's auth store
//! inside this process, which constitution §2 forbids however permissive the
//! licence is.

pub mod version;
pub mod wire;

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
    ProcessControl, ProviderCommand, ProviderProcess, ShutdownPolicy, SpawnedProvider, spawn,
};

/// What this binary announces over ACP and which CLI it pilots.
pub const SPEC: AdapterSpec = AdapterSpec {
    name: "meltemi-codex-acp",
    provider_layer: "the official `codex` CLI",
    provider_bin: "codex",
    dialect: Dialect::JsonRpcServer,
};

/// Points the adapter at a specific CLI binary instead of the one on the PATH.
///
/// It is how the end-to-end tests put a scripted wire where the provider would
/// be, and an escape hatch for a CLI installed somewhere the PATH does not
/// reach. It changes *which binary* is launched, never *what* is spoken to it.
pub const PROVIDER_BIN_ENV: &str = "MELTEMI_CODEX_BIN";

/// How long the handshake waits for the server to answer before refusing.
///
/// Generous, because a cold CLI on Windows can take seconds to come up; finite,
/// because a session that never opens and never fails is the worst of both.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// The id the handshake request carries. The adapter's own request numbering
/// starts after it.
const HANDSHAKE_ID: i64 = 1;

/// The JSON-RPC server dialect: launches the official CLI in its server mode
/// and pilots it.
pub struct CodexDialect {
    /// The binary to launch, after the environment override.
    program: String,
    /// How long a provider gets to exit on its own when a session closes.
    shutdown: ShutdownPolicy,
}

impl Default for CodexDialect {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexDialect {
    /// The dialect as the binary runs it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: resolve_program(std::env::var(PROVIDER_BIN_ENV).ok()),
            shutdown: ShutdownPolicy::default(),
        }
    }
}

/// Which binary to launch: the override when it says something, the registry's
/// name otherwise. An override set to whitespace is somebody's empty variable,
/// not an instruction.
fn resolve_program(override_value: Option<String>) -> String {
    override_value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| SPEC.provider_bin.to_string())
}

/// What was actually launched and what it turned out to be: the two facts a
/// session must be able to prove afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The binary the adapter launched, as it was resolved.
    pub program: String,
    /// The version read out of the handshake.
    pub version: String,
    /// The user agent the server announced, verbatim.
    pub user_agent: String,
}

impl Provenance {
    /// The provenance as ACP extension metadata.
    ///
    /// It travels in the `_meta` of a session update, which is the protocol's
    /// own extensibility point — not a private channel between this adapter and
    /// meltemid (design D7). The daemon records agent updates verbatim, so this
    /// lands in the session log like any other, and any third-party agent can
    /// say the same thing the same way.
    #[must_use]
    pub fn meta(&self) -> Meta {
        let mut meta = Meta::new();
        meta.insert(
            "meltemi".into(),
            json!({
                "providerBin": self.program,
                "providerVersion": self.version,
                "providerUserAgent": self.user_agent,
            }),
        );
        meta
    }
}

/// A session over the official CLI's server mode.
pub struct CodexSession {
    /// The provider process. `None` once the session has been shut down.
    provider: Mutex<Option<SpawnedProvider>>,
}

impl ProviderSession for CodexSession {
    async fn run_turn(&self, _prompt: PromptRequest) -> Result<StopReason, Refusal> {
        Err(Refusal::new(
            "dialect_not_wired",
            SPEC.name,
            "this adapter has shaken hands with the server but does not translate turns yet"
                .to_string(),
            "The thread/turn/item mapping and the approval relay land in \
             adaptadores-propios-acp 2.3-2.4."
                .to_string(),
        ))
    }

    async fn interrupt(&self) {}

    async fn shutdown(&self, policy: ShutdownPolicy) {
        if let Some(mut provider) = self.provider.lock().await.take() {
            let _ = provider.shutdown(policy).await;
        }
    }
}

impl ProviderDialect for CodexDialect {
    type Session = CodexSession;

    fn spec(&self) -> AdapterSpec {
        SPEC
    }

    async fn open(
        &self,
        session_id: SessionId,
        request: NewSessionRequest,
        cx: ConnectionTo<Client>,
    ) -> Result<Self::Session, Refusal> {
        let command = ProviderCommand {
            program: self.program.clone(),
            args: vec![wire::SERVER_MODE_ARG.to_string()],
            cwd: request.cwd.clone(),
        };
        let mut provider = spawn(&command, SPEC.provider_layer)?;

        // From here on the process exists, so every refusal has to end it. A
        // session that failed to open and left a CLI running would leak one
        // process per attempt, holding the worktree it was launched in.
        let provenance = match handshake(&mut provider, &self.program).await {
            Ok(provenance) => provenance,
            Err(refusal) => {
                let _ = provider.shutdown(self.shutdown).await;
                return Err(refusal);
            }
        };

        // The effective binary and the version it turned out to be, into the
        // session log, before anything else this session does.
        let _ = cx.send_notification(SessionNotification::new(
            session_id,
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta())),
        ));

        Ok(CodexSession {
            provider: Mutex::new(Some(provider)),
        })
    }
}

/// Shakes hands with the server and checks that it is a version this adapter
/// was verified against.
///
/// # Errors
///
/// Refuses — and never assumes compatibility — when the server does not answer,
/// answers something unreadable, or reports a version outside the declared
/// range.
async fn handshake<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
    program: &str,
) -> Result<Provenance, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    let params = wire::InitializeParams {
        client_info: wire::ClientInfo {
            name: SPEC.name.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    let request = json!({
        "jsonrpc": "2.0",
        "id": HANDSHAKE_ID,
        "method": wire::INITIALIZE,
        "params": serde_json::to_value(&params).map_err(|error| handshake_failed(
            &format!("the handshake could not be encoded ({error})")
        ))?,
    });
    provider
        .send(&request)
        .await
        .map_err(|error| handshake_failed(&format!("the handshake could not be sent ({error})")))?;

    let answer = tokio::time::timeout(HANDSHAKE_TIMEOUT, await_answer(provider, HANDSHAKE_ID))
        .await
        .map_err(|_| {
            handshake_failed(&format!(
                "the server did not answer the handshake within {} seconds",
                HANDSHAKE_TIMEOUT.as_secs()
            ))
        })??;

    let result: wire::InitializeResult = serde_json::from_value(answer).map_err(|error| {
        handshake_failed(&format!(
            "the server answered the handshake with something this adapter cannot read ({error})"
        ))
    })?;

    let Some(version) = version::of(&result.user_agent) else {
        return Err(version::unreadable(SPEC.provider_layer, &result.user_agent));
    };
    if !version::supported(version) {
        return Err(version::skew(SPEC.provider_layer, version));
    }

    // The wire expects this once the handshake is done. It is a notification:
    // nothing answers it, and nothing should wait for anything to.
    provider
        .send(&json!({"jsonrpc": "2.0", "method": wire::INITIALIZED}))
        .await
        .map_err(|error| {
            handshake_failed(&format!("the server closed during the handshake ({error})"))
        })?;

    Ok(Provenance {
        program: program.to_string(),
        version: version::render(version),
        user_agent: result.user_agent,
    })
}

/// Reads until the answer to `id` arrives.
///
/// Anything else the server says meanwhile is skipped rather than treated as
/// the answer: the wire is free to start talking before it finishes replying,
/// and a handshake that mistook a notification for its own answer would be
/// wrong in the one place that decides whether the session may exist.
async fn await_answer<C, W, R>(
    provider: &mut ProviderProcess<C, W, R>,
    id: i64,
) -> Result<Value, Refusal>
where
    C: ProcessControl,
    W: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    loop {
        let frame = provider.receive().await.map_err(|error| {
            handshake_failed(&format!("the server's output could not be read ({error})"))
        })?;
        let Some(frame) = frame else {
            return Err(handshake_failed(
                "the server closed its output without answering the handshake",
            ));
        };
        let message = match frame {
            Frame::Json(value) => value,
            // Not JSON at all: a banner or a warning. Kept on the adapter's own
            // stderr, where the daemon already collects the provider's noise,
            // and never mistaken for protocol.
            Frame::Unparsed(line) => {
                eprintln!("{}: provider said: {line}", SPEC.name);
                continue;
            }
        };
        if message.get("id").and_then(Value::as_i64) != Some(id) {
            continue;
        }
        if let Some(error) = message.get("error") {
            return Err(handshake_failed(&format!(
                "the server refused the handshake ({error})"
            )));
        }
        return message.get("result").cloned().ok_or_else(|| {
            handshake_failed("the server answered the handshake with neither a result nor an error")
        });
    }
}

/// The refusal for a handshake that did not happen.
fn handshake_failed(detail: &str) -> Refusal {
    Refusal::new(
        "provider_handshake_failed",
        SPEC.provider_layer,
        detail.to_string(),
        format!(
            "Check that the official CLI is installed and signed in, and that `{} {}` starts its \
             documented server mode (`meltemi fleet` shows the entry and its remedy).",
            SPEC.provider_bin,
            wire::SERVER_MODE_ARG
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A provider process that answers from a script, so a whole handshake runs
    /// in memory: no binary, no pipes, same behaviour on the three platforms.
    struct FakeProcess;

    impl ProcessControl for FakeProcess {
        async fn wait_within(&mut self, _grace: Duration) -> std::io::Result<bool> {
            Ok(true)
        }

        async fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Runs a handshake against a server that says exactly `lines`, and hands
    /// back the outcome together with everything the adapter sent.
    async fn handshake_against(lines: &str) -> (Result<Provenance, Refusal>, Vec<Value>) {
        let (adapter_out, mut server_in) = tokio::io::duplex(4096);
        let (mut server_out, adapter_in) = tokio::io::duplex(4096);
        tokio::io::AsyncWriteExt::write_all(&mut server_out, lines.as_bytes())
            .await
            .unwrap();
        drop(server_out);

        let mut provider = ProviderProcess::new(FakeProcess, adapter_out, adapter_in);
        let outcome = handshake(&mut provider, "codex").await;
        drop(provider);

        let mut sent = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut server_in, &mut sent)
            .await
            .unwrap();
        let sent = sent
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("the adapter speaks JSON"))
            .collect();
        (outcome, sent)
    }

    fn announcing(user_agent: &str) -> String {
        format!("{{\"id\":1,\"result\":{{\"userAgent\":\"{user_agent}\"}}}}\n")
    }

    #[tokio::test]
    async fn the_handshake_reads_the_version_and_answers_with_what_was_launched() {
        // Scenario: Versión efectiva registrada en el log
        //
        // Note the server's answer carries no `jsonrpc` member: that is what the
        // real CLI sends (verified against 0.77.0), and an adapter that demanded
        // it would refuse every real session while every fixture stayed green.
        let (outcome, sent) = handshake_against(&announcing(
            "codex_cli_rs/0.77.0 (Windows 10.0.26200; x86_64) (meltemi-codex-acp; 0.1.0)",
        ))
        .await;

        let provenance = outcome.expect("a supported version opens the session");
        assert_eq!(provenance.program, "codex");
        assert_eq!(provenance.version, "0.77.0");
        assert!(provenance.user_agent.starts_with("codex_cli_rs/0.77.0"));

        assert_eq!(sent[0]["method"], wire::INITIALIZE);
        assert_eq!(sent[0]["id"], HANDSHAKE_ID);
        assert_eq!(sent[0]["params"]["clientInfo"]["name"], SPEC.name);
        assert_eq!(
            sent[1]["method"],
            wire::INITIALIZED,
            "the wire expects this once the handshake is done"
        );
        assert!(
            sent[1].get("id").is_none(),
            "and it is a notification: nothing answers it"
        );
    }

    #[tokio::test]
    async fn what_the_server_says_before_answering_is_not_mistaken_for_the_answer() {
        // The wire may start talking before it finishes replying. A handshake
        // that took the first line it saw would read a version out of a
        // notification — and be wrong in the one place that decides whether the
        // session may exist at all.
        let noise = concat!(
            "starting up\n",
            "{\"method\":\"deprecationNotice\",\"params\":{\"summary\":\"soon\"}}\n",
            "{\"id\":99,\"result\":{\"userAgent\":\"codex_cli_rs/0.1.0\"}}\n",
        );
        let (outcome, _) = handshake_against(&format!(
            "{noise}{}",
            announcing("codex_cli_rs/0.77.0 (linux)")
        ))
        .await;
        assert_eq!(
            outcome.expect("the answer was the one with our id").version,
            "0.77.0"
        );
    }

    #[tokio::test]
    async fn a_version_outside_the_range_refuses_instead_of_assuming_compatibility() {
        // Scenario: Desfase de versión rehusado con remedio
        let (outcome, _) = handshake_against(&announcing("codex_cli_rs/0.1.0 (linux)")).await;
        let refusal = outcome.expect_err("an unverified version is not assumed compatible");
        assert_eq!(refusal.kind, "provider_version_skew");
        assert!(refusal.detail.contains("0.1.0"), "{}", refusal.detail);
        assert!(
            refusal.detail.contains(wire::VENDORED_SCHEMA_VERSION),
            "the refusal names both versions: {}",
            refusal.detail
        );
        assert!(refusal.remedy.contains("Update"), "{}", refusal.remedy);
    }

    #[tokio::test]
    async fn a_handshake_the_server_never_answers_refuses_rather_than_opening_a_session() {
        // Scenario: Desfase de versión rehusado con remedio
        for said in [
            "",
            "not json at all\n",
            "{\"id\":1,\"error\":{\"code\":-1}}\n",
        ] {
            let (outcome, _) = handshake_against(said).await;
            let refusal = outcome.expect_err("a session cannot open on a handshake that failed");
            assert_eq!(refusal.kind, "provider_handshake_failed", "for `{said}`");
            assert!(!refusal.remedy.is_empty());
        }

        // An answer whose user agent carries no version is the same story: read
        // it or refuse it, never guess it.
        let (outcome, _) = handshake_against(&announcing("codex_cli_rs/nightly")).await;
        assert_eq!(
            outcome.expect_err("no version, no session").kind,
            "provider_version_unreadable"
        );
    }

    #[test]
    fn the_provenance_travels_as_acp_extension_metadata() {
        // Scenario: Versión efectiva registrada en el log
        //
        // The daemon records an agent's session updates verbatim, so what the
        // log will hold is exactly this: which binary was launched, and which
        // version it turned out to be. Not a private channel — ACP's own `_meta`
        // (design D7), which any agent may fill the same way.
        let provenance = Provenance {
            program: "/opt/homebrew/bin/codex".into(),
            version: "0.77.0".into(),
            user_agent: "codex_cli_rs/0.77.0 (macOS)".into(),
        };
        let update =
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().meta(provenance.meta()));
        let logged = serde_json::to_value(&update).expect("an update serializes");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerBin"],
            "/opt/homebrew/bin/codex"
        );
        assert_eq!(logged["_meta"]["meltemi"]["providerVersion"], "0.77.0");
        assert_eq!(
            logged["_meta"]["meltemi"]["providerUserAgent"],
            "codex_cli_rs/0.77.0 (macOS)"
        );
    }

    #[test]
    fn the_environment_override_decides_which_binary_is_launched() {
        // The knob the end-to-end test turns to put a scripted wire where the
        // provider would be. Without it, the registry's binary name stands —
        // and an empty variable is not an instruction to launch nothing.
        assert_eq!(resolve_program(None), SPEC.provider_bin);
        assert_eq!(resolve_program(Some("  ".into())), SPEC.provider_bin);
        assert_eq!(
            resolve_program(Some("/tmp/mock-codex-wire".into())),
            "/tmp/mock-codex-wire"
        );
    }

    #[test]
    fn a_handshake_that_did_not_happen_refuses_with_a_remedy() {
        let refusal = handshake_failed("the server closed its output");
        assert_eq!(refusal.kind, "provider_handshake_failed");
        assert_eq!(refusal.layer, SPEC.provider_layer);
        assert!(refusal.remedy.contains(wire::SERVER_MODE_ARG));
    }
}
