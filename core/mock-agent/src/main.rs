// SPDX-License-Identifier: Apache-2.0

//! Scripted ACP agent for Meltemi's automated end-to-end test (task 4.6).
//!
//! This is NOT a real coding agent: it plays a fixed script that exercises
//! the whole ACP path meltemid drives — `initialize`, `session/new`, a
//! streamed agent message, one permission request, and (when the client
//! grants it) a file write — then ends the turn. The daemon's e2e test
//! asserts each of these steps.
//!
//! Contract with meltemid's propose flow: the prompt text carries a line
//! `PROPOSAL_PATH: <absolute path>`. When the permission is granted, the
//! agent writes a marker into that file, standing in for a real agent
//! filling in the proposal. Real agents ignore the marker line.

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ConfigOptionUpdate, ContentBlock, ContentChunk,
    CurrentModeUpdate, InitializeRequest, InitializeResponse, LoadSessionRequest,
    LoadSessionResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, SessionConfigBoolean, SessionConfigKind, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigOptionValue, SessionConfigSelect,
    SessionConfigSelectOption, SessionId, SessionMode, SessionModeId, SessionModeState,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Result, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Fixed session id the scripted agent hands back from `session/new`.
const SESSION_ID: &str = "mock-session";
/// Option id the agent offers (and checks for) to allow the file write.
const ALLOW_OPTION: &str = "allow";

/// The two config options the mock announces under `--config-options`: a
/// selector and a toggle, because the two ACP kinds take different paths on the
/// daemon side and a mock that only had one would leave the other untested.
const MODEL_OPTION: &str = "model";
const THINKING_OPTION: &str = "thinking";
/// Effort as a selector (`--reasoning`), the realistic shape of it
/// (modelo-por-opcion-anunciada design D7).
const REASONING_OPTION: &str = "reasoning";
/// A second option of category `model` (`--config-twins`), so which one is
/// "the model" cannot be told.
const TWIN_OPTION: &str = "model-backup";
/// A `mode` option (`--config-mode-shift`) that setting the model flips.
const CONFIG_MODE_OPTION: &str = "mode";

/// The two modes the mock announces under `--modes`, by the protocol's OLDER
/// field — the one that describes modes and nothing else.
///
/// This is the shape of an agent born before configuration options existed,
/// and it is the case the daemon used not to read at all. A mock that could
/// only announce the newer shape could not exercise it.
const DEFAULT_MODE: &str = "default";
const ACCEPT_EDITS_MODE: &str = "acceptEdits";

/// What the announced options currently hold. Real state, not a canned answer:
/// the point of the flag is that `session/set_config_option` actually changes
/// what the next announcement says, which is what the daemon reads back.
static CURRENT_MODEL: Mutex<String> = Mutex::new(String::new());
static CURRENT_THINKING: AtomicBool = AtomicBool::new(false);
/// The reasoning level under `--reasoning`.
static CURRENT_REASONING: Mutex<String> = Mutex::new(String::new());
/// The `mode` option under `--config-mode-shift`.
static CURRENT_CONFIG_MODE: Mutex<String> = Mutex::new(String::new());
/// Which mode the mock is in, for the same reason: `session/set_mode` has to
/// change something a later announcement reflects.
static CURRENT_MODE: Mutex<String> = Mutex::new(String::new());

/// The modes as they stand right now, in the older form.
fn announced_modes() -> SessionModeState {
    let current = CURRENT_MODE.lock().expect("mock mode state").clone();
    let current = if current.is_empty() {
        DEFAULT_MODE.to_string()
    } else {
        current
    };
    SessionModeState::new(
        SessionModeId::new(current),
        vec![
            SessionMode::new(SessionModeId::new(DEFAULT_MODE), "Ask first"),
            SessionMode::new(SessionModeId::new(ACCEPT_EDITS_MODE), "Accept edits")
                .description("Writes without asking"),
        ],
    )
}

/// A second announcement of configuration options, deliberately NOT the one
/// `announced_options` returns: one option, of an id the first set never had.
///
/// It is what `--options-drift` sends, and its whole job is to be
/// distinguishable from what came before, so a test can tell "replaced" from
/// "merged" without reading the daemon's mind.
fn drifted_options() -> Vec<SessionConfigOption> {
    vec![
        SessionConfigOption::new(
            "verbosity",
            "Verbosity",
            SessionConfigKind::Select(SessionConfigSelect::new(
                "brief",
                vec![
                    SessionConfigSelectOption::new("brief", "Brief"),
                    SessionConfigSelectOption::new("full", "Full"),
                ],
            )),
        )
        .category(SessionConfigOptionCategory::Other("output".into())),
    ]
}

/// Which options the mock announces, from its switches (design D7 of
/// modelo-por-opcion-anunciada). Off by default, like every other switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Shape {
    /// `--reasoning`: effort as a `low`/`high` selector instead of the toggle.
    reasoning: bool,
    /// `--config-twins`: a second option of category `model`.
    twins: bool,
    /// `--config-mode-shift`: a `mode` option, which setting the model flips.
    mode_shift: bool,
}

impl Shape {
    fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let mut shape = Self::default();
        for arg in args {
            match arg.as_str() {
                "--reasoning" => shape.reasoning = true,
                "--config-twins" => shape.twins = true,
                "--config-mode-shift" => shape.mode_shift = true,
                _ => {}
            }
        }
        shape
    }
}

/// What a state variable holds, or its default while nothing set it.
fn current(state: &Mutex<String>, default: &str) -> String {
    let value = state.lock().expect("mock config state").clone();
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}

/// The options as they stand right now.
fn announced_options() -> Vec<SessionConfigOption> {
    options_for(
        Shape::from_args(std::env::args()),
        &current(&CURRENT_MODEL, "fast"),
        CURRENT_THINKING.load(Ordering::SeqCst),
        &current(&CURRENT_REASONING, "low"),
        &current(&CURRENT_CONFIG_MODE, "ask"),
    )
}

/// The announcement for a shape and a state: a model selector always; the
/// thinking toggle or, with `--reasoning`, a reasoning selector; a twin model
/// selector with `--config-twins`; a `mode` option with `--config-mode-shift`.
fn options_for(
    shape: Shape,
    model: &str,
    thinking: bool,
    reasoning: &str,
    mode: &str,
) -> Vec<SessionConfigOption> {
    let models = || {
        vec![
            SessionConfigSelectOption::new("fast", "Fast"),
            SessionConfigSelectOption::new("slow", "Slow").description("The careful one"),
        ]
    };
    let mut options = vec![
        SessionConfigOption::new(
            MODEL_OPTION,
            "Model",
            SessionConfigKind::Select(SessionConfigSelect::new(model.to_string(), models())),
        )
        .category(SessionConfigOptionCategory::Model),
    ];
    options.push(if shape.reasoning {
        SessionConfigOption::new(
            REASONING_OPTION,
            "Reasoning",
            SessionConfigKind::Select(SessionConfigSelect::new(
                reasoning.to_string(),
                vec![
                    SessionConfigSelectOption::new("low", "Low"),
                    SessionConfigSelectOption::new("high", "High"),
                ],
            )),
        )
        .category(SessionConfigOptionCategory::ThoughtLevel)
    } else {
        SessionConfigOption::new(
            THINKING_OPTION,
            "Extended thinking",
            SessionConfigKind::Boolean(SessionConfigBoolean::new(thinking)),
        )
        .category(SessionConfigOptionCategory::ThoughtLevel)
    });
    if shape.twins {
        options.push(
            SessionConfigOption::new(
                TWIN_OPTION,
                "Backup model",
                SessionConfigKind::Select(SessionConfigSelect::new("fast".to_string(), models())),
            )
            .category(SessionConfigOptionCategory::Model),
        );
    }
    if shape.mode_shift {
        options.push(
            SessionConfigOption::new(
                CONFIG_MODE_OPTION,
                "Mode",
                SessionConfigKind::Select(SessionConfigSelect::new(
                    mode.to_string(),
                    vec![
                        SessionConfigSelectOption::new("ask", "Ask first"),
                        SessionConfigSelectOption::new("auto", "Act on its own"),
                    ],
                )),
            )
            .category(SessionConfigOptionCategory::Mode),
        );
    }
    options
}

/// The other model: what `--config-clamp` leaves instead of what was asked.
fn clamped(asked: &str) -> &'static str {
    if asked == "fast" { "slow" } else { "fast" }
}

/// What `--echo-levers` says it read from the environment the daemon
/// composed: the proof that the launch-flag route reaches the agent, and that
/// the announced route leaves that environment alone.
fn levers_line(model: Option<String>, effort: Option<String>) -> String {
    format!(
        "levers read: model={} effort={}",
        model.as_deref().unwrap_or("-"),
        effort.as_deref().unwrap_or("-")
    )
}

/// Set when the client sends `session/cancel` AND `--honor-cancel` was passed.
///
/// Off by default on purpose: a mock that honoured cancel unconditionally would
/// change the stop reason every existing cancellation test reads, and those
/// tests assert what the daemon does when an agent *ignores* the cancel — which
/// is the case the daemon has to survive (redirigir-turno design D6).
static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

/// How many prompts this process has received, so a switch can single out the
/// first turns of a session.
static TURNS_SEEN: AtomicU32 = AtomicU32::new(0);

#[tokio::main]
async fn main() -> Result<()> {
    // `--load-session` makes the mock announce session-load support and handle
    // `session/load`, so the resume path can be exercised end to end.
    let supports_load = std::env::args().any(|a| a == "--load-session");
    // `--mcp` makes the mock announce MCP support, so the passthrough injection
    // path can be exercised end to end.
    let supports_mcp = std::env::args().any(|a| a == "--mcp");
    let honors_cancel = std::env::args().any(|a| a == "--honor-cancel");
    // `--cancel-turn` makes the mock end its turn `Cancelled` with nobody having
    // asked: the agent giving up on its own, which the daemon must keep treating
    // as the end of the session (redirigir-turno).
    let cancels_itself = std::env::args().any(|a| a == "--cancel-turn");
    // `--config-options` makes the mock announce session config options and
    // honour `session/set_config_option`, so the live-change path can be
    // exercised without any provider (modelo-y-esfuerzo design D2).
    //
    // Off by default like every other mock switch, and for the same reason:
    // announcing options unconditionally would make every existing session
    // start emit a `config_options_announced` event that the tests reading the
    // log were never written to expect.
    let announces_config = std::env::args().any(|a| a == "--config-options");
    // `--modes` makes the mock announce its modes by the protocol's older
    // field and honour `session/set_mode`. Off by default like every other
    // switch, and mutually exclusive with `--config-options` in practice: an
    // agent announcing both is the precedence case, which the unit tests
    // cover without needing a process (apagado-entero-y-modos design D6).
    let announces_modes = std::env::args().any(|a| a == "--modes");
    // `--mode-drift` makes the mock change its own mode mid-turn and say so,
    // which is the agent acting on its own account rather than answering.
    let drifts_mode = std::env::args().any(|a| a == "--mode-drift");
    // `--options-drift` makes it send a wholly different list of options
    // mid-turn, so "replaced whole" can be told apart from "merged".
    let drifts_options = std::env::args().any(|a| a == "--options-drift");
    // `--config-clamp` answers a model change with the OTHER model, the agent
    // that falls back on its own; `--config-hang` never answers one; and
    // `--config-mode-shift` flips its `mode` option when the model is set
    // (modelo-por-opcion-anunciada design D7).
    let clamps = std::env::args().any(|a| a == "--config-clamp");
    let hangs = std::env::args().any(|a| a == "--config-hang");
    let shifts_mode = Shape::from_args(std::env::args()).mode_shift;

    Agent
        .builder()
        .name("mock-agent")
        .on_receive_notification(
            async move |cancel: CancelNotification, _cx| {
                let _ = cancel;
                if honors_cancel {
                    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _cx| {
                let mut capabilities = AgentCapabilities::new();
                capabilities.load_session = supports_load;
                capabilities.mcp_capabilities.http = supports_mcp;
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version)
                        .agent_capabilities(capabilities),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_new_session: NewSessionRequest, responder, _cx| {
                let mut response = NewSessionResponse::new(SessionId::new(SESSION_ID));
                if announces_config {
                    response = response.config_options(announced_options());
                }
                if announces_modes {
                    response = response.modes(announced_modes());
                }
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // Loading a prior session succeeds (the mock keeps no real state);
            // its presence lets the daemon exercise the resume path.
            async move |_load: LoadSessionRequest, responder, _cx| {
                let mut response = LoadSessionResponse::new();
                if announces_config {
                    response = response.config_options(announced_options());
                }
                if announces_modes {
                    response = response.modes(announced_modes());
                }
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // The live change. Values the announcement does not cover are
            // refused here too: the daemon checks first, and an agent that
            // accepted anything anyway would let a bug on that side pass
            // unnoticed.
            async move |set: SetSessionConfigOptionRequest, responder, _cx| {
                if !announces_config {
                    return responder.respond(SetSessionConfigOptionResponse::new(Vec::new()));
                }
                // The answer never comes: the responder is kept, never used.
                if hangs {
                    std::mem::forget(responder);
                    return Ok(());
                }
                match (set.config_id.0.as_ref(), &set.value) {
                    (MODEL_OPTION, SessionConfigOptionValue::ValueId { value })
                        if matches!(value.0.as_ref(), "fast" | "slow") =>
                    {
                        let kept = if clamps {
                            clamped(value.0.as_ref())
                        } else {
                            value.0.as_ref()
                        };
                        *CURRENT_MODEL.lock().expect("mock config state") = kept.to_string();
                        if shifts_mode {
                            *CURRENT_CONFIG_MODE.lock().expect("mock config state") =
                                "auto".to_string();
                        }
                    }
                    (REASONING_OPTION, SessionConfigOptionValue::ValueId { value })
                        if matches!(value.0.as_ref(), "low" | "high") =>
                    {
                        *CURRENT_REASONING.lock().expect("mock config state") = value.0.to_string();
                    }
                    (THINKING_OPTION, SessionConfigOptionValue::Boolean { value }) => {
                        CURRENT_THINKING.store(*value, Ordering::SeqCst);
                    }
                    _ => {}
                }
                responder.respond(SetSessionConfigOptionResponse::new(announced_options()))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // The protocol's own mode verb. It answers with no list at all, so
            // acceptance is the whole answer -- which is exactly why the mock
            // has to keep real state: a later announcement is the only place
            // the change can be seen.
            async move |set: SetSessionModeRequest, responder, _cx| {
                if announces_modes
                    && matches!(set.mode_id.0.as_ref(), DEFAULT_MODE | ACCEPT_EDITS_MODE)
                {
                    *CURRENT_MODE.lock().expect("mock mode state") = set.mode_id.0.to_string();
                }
                responder.respond(SetSessionModeResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |prompt: PromptRequest, responder, cx: ConnectionTo<Client>| {
                // Run the turn on a spawned task: it sends a permission request
                // and awaits it, which must NOT happen on the dispatch loop —
                // the loop has to stay free to deliver that very response
                // (a `block_task` inside a handler deadlocks by design).
                cx.spawn({
                    let cx = cx.clone();
                    async move {
                        // A silent turn says nothing and does nothing: the
                        // agent whose provider failed and who closed the turn
                        // anyway (turno-vacio-es-error).
                        // It still holds the turn open for `--turn-delay-ms`,
                        // so a test can stop it in flight, and answers with the
                        // same stop reason a turn that worked would.
                        let turn = TURNS_SEEN.fetch_add(1, Ordering::SeqCst);
                        if silent_in(std::env::args(), turn) {
                            hold_turn().await;
                            let stop = if cancels_itself
                                || CANCEL_REQUESTED.swap(false, Ordering::SeqCst)
                            {
                                StopReason::Cancelled
                            } else {
                                StopReason::EndTurn
                            };
                            return responder.respond(PromptResponse::new(stop));
                        }
                        // The agent changing things on its own account, before
                        // it does any work: nobody asked, and the daemon has to
                        // notice anyway.
                        if drifts_mode {
                            *CURRENT_MODE.lock().expect("mock mode state") =
                                ACCEPT_EDITS_MODE.to_string();
                            let _ = cx.send_notification(SessionNotification::new(
                                prompt.session_id.clone(),
                                SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(
                                    SessionModeId::new(ACCEPT_EDITS_MODE),
                                )),
                            ));
                        }
                        if drifts_options {
                            let _ = cx.send_notification(SessionNotification::new(
                                prompt.session_id.clone(),
                                SessionUpdate::ConfigOptionUpdate(ConfigOptionUpdate::new(
                                    drifted_options(),
                                )),
                            ));
                        }
                        run_scripted_turn(&prompt, &cx).await;
                        // The stop reason is read AFTER the turn, and reset, so a
                        // session that keeps going reports the next turn on its
                        // own merits rather than inheriting this one's ending.
                        let stop =
                            if cancels_itself || CANCEL_REQUESTED.swap(false, Ordering::SeqCst) {
                                StopReason::Cancelled
                            } else {
                                StopReason::EndTurn
                            };
                        responder.respond(PromptResponse::new(stop))
                    }
                })?;
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}

/// Plays the fixed script: stream a message, ask one permission, and — if
/// granted — write the proposal marker into the path from the prompt.
async fn run_scripted_turn(prompt: &PromptRequest, cx: &ConnectionTo<Client>) {
    let session_id = prompt.session_id.clone();

    // `--turn-delay-ms <N>` holds each turn open for N milliseconds before doing
    // any work, so a test can race a `session/direct` into an active turn
    // (control-remoto-asistido). Default: no delay.
    if !hold_turn().await {
        return;
    }

    // 0. `--think` streams a thought chunk before speaking, so the surfaces
    // that show an agent's reasoning can be exercised without a real agent
    // (pensamiento-a-la-vista). Off by default: a mock that thinks by default
    // would change what every existing transcript test reads.
    if thinks() {
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Reading the proposal before writing it."),
            ))),
        ));
    }

    // 0b. With `--echo-levers`, say what the launch environment carried, so a
    // test can see the launch-flag route arrive (modelo-por-opcion-anunciada).
    if std::env::args().any(|a| a == "--echo-levers") {
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(levers_line(
                    std::env::var("MELTEMI_SESSION_MODEL").ok(),
                    std::env::var("MELTEMI_SESSION_EFFORT").ok(),
                )),
            ))),
        ));
    }

    // 1. Stream a chunk of the agent's "response".
    let _ = cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Filling in the proposal.",
        )))),
    ));

    // 1b. With `--ask`, put a real question first: the agent's own options,
    // one of them recommended in its label, exactly as a provider would send
    // it. The answer comes back as the selected option id.
    if asks() {
        let question = RequestPermissionRequest::new(
            session_id.clone(),
            ToolCallUpdate::new("ask-route", ToolCallUpdateFields::new()),
            vec![
                PermissionOption::new(
                    "option-0",
                    "Rewrite it (recommended)",
                    PermissionOptionKind::AllowOnce,
                ),
                PermissionOption::new("option-1", "Patch it", PermissionOptionKind::AllowOnce),
            ],
        );
        let chosen = cx.send_request(question).block_task().await;
        // The choice is echoed into the transcript, so a test can assert the
        // turn actually continued with the answer rather than merely receiving
        // one.
        let _ = cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(format!("answering with {}", chosen_option(&chosen))),
            ))),
        ));
    }

    // 2. Ask the client for permission to write the file.
    // The path this write would touch rides with the request, because a real
    // agent's tool call carries it (`rawInput.path` or `locations[0].path`, ACP
    // v1) and the daemon's rule matchers read it. A mock that omitted it was
    // under-modelling the protocol, and any posture bounded BY PATH would have
    // looked broken against it (modos-de-autonomia design D5).
    let fields = permission_fields(
        titled_in(std::env::args()),
        proposal_path(prompt).or_else(|| sdd_artifact_path(prompt)),
    );
    let request = RequestPermissionRequest::new(
        session_id,
        ToolCallUpdate::new("write-proposal", fields),
        vec![
            PermissionOption::new(ALLOW_OPTION, "Allow", PermissionOptionKind::AllowOnce),
            PermissionOption::new("reject", "Reject", PermissionOptionKind::RejectOnce),
        ],
    );
    let decision = cx.send_request(request).block_task().await;

    // 3. If explicitly allowed, write the requested artifact(s).
    if granted(&decision) {
        if let Some(path) = proposal_path(prompt) {
            // Echo the profile name AND a marker env var, so a launch profile's
            // env overlay is observable end-to-end (flota-multiproveedor): the
            // marker proves which auth context the subprocess ran under.
            let marker = std::env::var("MELTEMI_MOCK_MARKER").unwrap_or_default();
            let _ = std::fs::write(
                &path,
                format!(
                    "# Proposal ({profile})\n\nGenerated by the scripted end-to-end agent \
                     with the `{profile}` profile.\nmarker={marker}\n",
                    profile = profile()
                ),
            );
        }
        write_sdd_artifact(prompt);
    }
}

/// Which option a permission answer selected, or `none` when it selected
/// nothing. Written for a transcript a test reads, never for a protocol.
fn chosen_option(
    decision: &std::result::Result<
        agent_client_protocol::schema::v1::RequestPermissionResponse,
        agent_client_protocol::Error,
    >,
) -> String {
    match decision {
        Ok(response) => match &response.outcome {
            RequestPermissionOutcome::Selected(selected) => selected.option_id.0.to_string(),
            RequestPermissionOutcome::Cancelled => "cancelled".to_string(),
            _ => "unknown".to_string(),
        },
        Err(_) => "none".to_string(),
    }
}

/// Reads the optional `--turn-delay-ms <N>` argument (milliseconds to hold each
/// turn open before scripting it).
/// Whether `--think` was passed: the mock streams a thought chunk before it
/// speaks.
fn thinks() -> bool {
    std::env::args().any(|a| a == "--think")
}

/// Whether `--ask` was passed: the mock puts a question with options before it
/// works, so the answering flow is exercisable without a real agent and without
/// the network. Off by default, like every other knob here — a mock that asked
/// on its own would change what every existing permission test reads
/// (preguntas-del-agente design D6).
fn asks() -> bool {
    std::env::args().any(|a| a == "--ask")
}

/// Whether `--titled-permission` was passed: the mock's permission request then
/// carries a kind and a title, as a real agent's does, so the rule a human
/// remembers from a terminal has a tool to name (aprobador-en-el-cli design
/// D10). Off by default: without it the request carries neither, which is what
/// every existing permission test reads.
fn titled_in(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|arg| arg == "--titled-permission")
}

/// The fields of the mock's permission request: the path it would write when
/// the prompt names one, and a kind and a title when `titled`.
fn permission_fields(titled: bool, path: Option<std::path::PathBuf>) -> ToolCallUpdateFields {
    let mut fields = ToolCallUpdateFields::new();
    if let Some(path) = path {
        fields = fields.raw_input(serde_json::json!({ "path": path }));
    }
    if titled {
        fields = fields.kind(ToolKind::Edit).title("Write the proposal");
    }
    fields
}

/// Holds the turn open for `--turn-delay-ms`, if given. `false` when a
/// cancellation this mock honours arrived meanwhile, which ends the turn's work.
async fn hold_turn() -> bool {
    if let Some(ms) = turn_delay_ms() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if CANCEL_REQUESTED.load(Ordering::SeqCst) {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    true
}

/// Whether the turn numbered `turn` (from zero) ends in silence: no update, no
/// permission request, just its stop reason (turno-vacio-es-error design D8).
///
/// `--silent-turn` silences every turn; `--silent-turn-at <N>` only turn `N`,
/// so a session of several turns can run work, then silence, then work again.
/// Off by default, like every other switch here: a mock that fell silent on its
/// own would change what every existing test reads.
fn silent_in(args: impl IntoIterator<Item = String>, turn: u32) -> bool {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--silent-turn" {
            return true;
        }
        if arg == "--silent-turn-at" {
            return args.next().and_then(|n| n.parse::<u32>().ok()) == Some(turn);
        }
    }
    false
}

fn turn_delay_ms() -> Option<u64> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--turn-delay-ms" {
            return args.next().and_then(|v| v.parse().ok());
        }
    }
    None
}

/// The mock's profile name (`--profile <name>`, default `mock`). Two instances
/// with distinct profiles produce distinguishable output, so provider
/// parallelism is observable in the acceptance script (hito-v01-aceptacion).
fn profile() -> String {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            return args.next().unwrap_or_else(|| "mock-agent".to_string());
        }
    }
    "mock-agent".to_string()
}

/// Concatenates the prompt's text blocks.
fn prompt_text(prompt: &PromptRequest) -> String {
    let mut text = String::new();
    for block in &prompt.prompt {
        if let ContentBlock::Text(t) = block {
            text.push_str(&t.text);
            text.push('\n');
        }
    }
    text
}

/// Reads a `KEY: value` line from the prompt.
fn value_after(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix(key).map(|r| r.trim().to_string()))
}

/// Writes the SDD artifact the authoring prompt asked for, with valid content
/// (or, with `--sdd-invalid`, an intentionally invalid delta spec so the
/// engine's validation-as-gate can be exercised).
/// The path an SDD authoring turn is about to write, read from the same prompt
/// keys `write_sdd_artifact` uses. It rides with the permission request so the
/// daemon's path matchers have something to match, exactly as a real agent's
/// tool call would carry it.
fn sdd_artifact_path(prompt: &PromptRequest) -> Option<std::path::PathBuf> {
    let text = prompt_text(prompt);
    for key in [
        "CONSTITUTION_PATH:",
        "TASKS_PATH:",
        "ARTIFACT_PATH:",
        "DESIGN_PATH:",
    ] {
        if let Some(path) = value_after(&text, key) {
            return Some(std::path::PathBuf::from(path));
        }
    }
    None
}

fn write_sdd_artifact(prompt: &PromptRequest) {
    let text = prompt_text(prompt);
    let invalid = std::env::args().any(|a| a == "--sdd-invalid");

    if let Some(path) = value_after(&text, "CONSTITUTION_PATH:") {
        let _ = std::fs::write(
            path,
            "# Constitution (mock)\n\n## Principles\n\n1. Spec-first.\n",
        );
        return;
    }
    if let Some(path) = value_after(&text, "TASKS_PATH:") {
        let _ = std::fs::write(path, "## 1. Work\n\n- [ ] 1.1 Do the thing (dep: none)\n");
        return;
    }
    let Some(artifact) = value_after(&text, "ARTIFACT:") else {
        return;
    };
    let Some(path) = value_after(&text, "ARTIFACT_PATH:") else {
        return;
    };
    match artifact.as_str() {
        "proposal" => {
            let _ = std::fs::write(
                &path,
                "## Why\n\nBecause.\n\n## What Changes\n\n## Impact\n",
            );
        }
        "specs" => {
            // ARTIFACT_PATH is the specs directory; write one capability delta.
            let cap_dir = std::path::Path::new(&path).join("example-capability");
            let _ = std::fs::create_dir_all(&cap_dir);
            let spec = if invalid {
                // A requirement with no scenario: the engine must reject this.
                "## ADDED Requirements\n\n### Requirement: Broken\nThe system SHALL do a thing.\n"
            } else {
                "## ADDED Requirements\n\n### Requirement: Example\n\
                 The system SHALL do a thing.\n\n#### Scenario: It works\n\
                 - **WHEN** invoked\n- **THEN** it responds\n"
            };
            let _ = std::fs::write(cap_dir.join("spec.md"), spec);
        }
        "design" => {
            let _ = std::fs::write(&path, "## Context\n\n## Decisions\n\n### D1\nA choice.\n");
        }
        "tasks" => {
            let _ = std::fs::write(&path, "## 1. Work\n\n- [ ] 1.1 Do the thing\n");
        }
        _ => {}
    }
}

/// Whether the client granted the write by selecting the allow option.
fn granted(
    decision: &std::result::Result<
        agent_client_protocol::schema::v1::RequestPermissionResponse,
        agent_client_protocol::Error,
    >,
) -> bool {
    match decision {
        Ok(response) => match &response.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                selected.option_id.0.as_ref() == ALLOW_OPTION
            }
            _ => false,
        },
        Err(_) => false,
    }
}

/// Extracts the `PROPOSAL_PATH:` line from the prompt's text content.
fn proposal_path(prompt: &PromptRequest) -> Option<std::path::PathBuf> {
    for block in &prompt.prompt {
        if let ContentBlock::Text(text) = block {
            for line in text.text.lines() {
                if let Some(rest) = line.strip_prefix("PROPOSAL_PATH:") {
                    return Some(std::path::PathBuf::from(rest.trim()));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        CONFIG_MODE_OPTION, MODEL_OPTION, REASONING_OPTION, Shape, THINKING_OPTION, TWIN_OPTION,
        ToolKind, clamped, levers_line, options_for, permission_fields, silent_in, titled_in,
    };

    #[test]
    fn the_switches_shape_what_is_announced() {
        let args = |list: &[&str]| -> Vec<String> {
            std::iter::once("mock-agent")
                .chain(list.iter().copied())
                .map(str::to_string)
                .collect()
        };
        let ids = |shape: Shape| -> Vec<String> {
            options_for(shape, "fast", false, "low", "ask")
                .into_iter()
                .map(|option| option.id.0.to_string())
                .collect()
        };
        // Off by default: the model selector and the thinking toggle, as ever.
        let plain = Shape::from_args(args(&["--config-options"]));
        assert_eq!(plain, Shape::default());
        assert_eq!(ids(plain), vec![MODEL_OPTION, THINKING_OPTION]);
        // `--reasoning` puts a selector where the toggle was.
        let reasoning = Shape::from_args(args(&["--reasoning"]));
        assert_eq!(ids(reasoning), vec![MODEL_OPTION, REASONING_OPTION]);
        // `--config-twins` adds a second model selector; `--config-mode-shift`
        // a mode option.
        let twins = Shape::from_args(args(&["--config-twins"]));
        assert_eq!(ids(twins), vec![MODEL_OPTION, THINKING_OPTION, TWIN_OPTION]);
        let shifting = Shape::from_args(args(&["--config-mode-shift"]));
        assert_eq!(
            ids(shifting),
            vec![MODEL_OPTION, THINKING_OPTION, CONFIG_MODE_OPTION]
        );
    }

    #[test]
    fn a_clamp_leaves_the_other_model_and_the_echo_says_what_was_read() {
        assert_eq!(clamped("slow"), "fast");
        assert_eq!(clamped("fast"), "slow");
        assert_eq!(
            levers_line(Some("gpt-x".into()), None),
            "levers read: model=gpt-x effort=-"
        );
        assert_eq!(levers_line(None, None), "levers read: model=- effort=-");
    }

    #[test]
    fn a_titled_permission_names_its_kind_and_title_and_only_when_asked() {
        let args = |list: &[&str]| -> Vec<String> {
            std::iter::once("mock-agent")
                .chain(list.iter().copied())
                .map(str::to_string)
                .collect()
        };
        assert!(titled_in(args(&["--titled-permission"])));
        assert!(!titled_in(args(&["--think"])));

        let titled = permission_fields(true, Some(std::path::PathBuf::from("/repo/proposal.md")));
        assert_eq!(titled.kind, Some(ToolKind::Edit));
        assert_eq!(titled.title.as_deref(), Some("Write the proposal"));
        assert!(titled.raw_input.is_some(), "the path still travels");

        let bare = permission_fields(false, None);
        assert!(bare.kind.is_none() && bare.title.is_none() && bare.raw_input.is_none());
    }

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("mock-agent")
            .chain(list.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn no_switch_silences_nothing() {
        for turn in 0..3 {
            assert!(!silent_in(args(&[]), turn));
            assert!(!silent_in(args(&["--think", "--ask"]), turn));
        }
    }

    #[test]
    fn the_silent_switch_silences_every_turn() {
        for turn in 0..3 {
            assert!(silent_in(args(&["--silent-turn"]), turn));
        }
    }

    #[test]
    fn a_numbered_turn_is_the_only_silent_one() {
        let at_one = args(&["--silent-turn-at", "1"]);
        assert!(!silent_in(at_one.clone(), 0), "work first");
        assert!(silent_in(at_one.clone(), 1), "then silence");
        assert!(!silent_in(at_one, 2), "then work again");
        assert!(
            !silent_in(args(&["--silent-turn-at", "x"]), 0),
            "an unreadable turn silences none"
        );
    }
}
