// SPDX-License-Identifier: Apache-2.0

//! Following and reading a session from any shell (verbos-del-agent-boss).
//!
//! `watch` and `log` are streams: their useful output is the session's record,
//! which already is a succession of objects, so they write one record per line
//! in every format — one YAML document per record under `--yaml` — and an error
//! halfway closes the stream with its error object (design D5). What `watch`
//! prints is the record `log` will hold (D7), with every character a terminal
//! would obey written as a JSON escape (D8). And `watch` never outlives its
//! session or its daemon (D6).

use std::io::Write;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::UnboundedReceiver;

use meltemi_client::rpc::{Incoming, Peer, RpcError};
use meltemi_proto::{
    SessionInfo, SessionListParams, SessionListResult, SessionLogParams, SessionLogResult,
    SessionWatchParams, StatusResult, methods,
};

use crate::approver::{clean, is_invisible_format};
use crate::format::Format;
use crate::output::CliError;

/// How often `watch` asks whether its session still lives (design D6): the
/// window between a session's close being published and the session leaving
/// the registry, and a close lost to a lagging stream, both end here.
pub const LIVENESS: Duration = Duration::from_secs(2);

/// Lines asked for per page of `session/log`.
const PAGE: u32 = 500;

/// Writes every character a terminal would obey — controls other than the
/// line break, and the characters that reorder a line or hide in it — as a JSON
/// escape (design D8).
///
/// In a JSON line those characters can only stand inside a string, where the
/// escape means exactly the same character: the line stays valid JSON and,
/// read as JSON, keeps its value. In the YAML this crate writes, every string
/// is double-quoted, where the same escape is YAML's too.
#[must_use]
pub fn escape_obeyed(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if (c.is_control() && c != '\n') || is_invisible_format(c) {
            // Every character in the set lies in the Basic Multilingual Plane,
            // so one four-digit escape says it whole.
            out.push_str(&format!("\\u{:04x}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

/// Writes one record: as the JSON line it is (human and `--json`), or as one
/// YAML document (`--yaml`). Flushed at once — a stream that waits for a buffer
/// to fill is not live.
///
/// # Errors
///
/// When stdout cannot be written.
pub fn write_record(out: &mut impl Write, format: Format, line: &str) -> std::io::Result<()> {
    match format {
        Format::Yaml => {
            // A line that is not JSON is carried as the string it is, never
            // dropped: the record is the record.
            let value = serde_json::from_str::<Value>(line)
                .unwrap_or_else(|_| Value::String(line.to_string()));
            write!(
                out,
                "---\n{}",
                escape_obeyed(&crate::format::to_yaml(&value))
            )?;
        }
        Format::Human | Format::Json => writeln!(out, "{}", escape_obeyed(line))?,
    }
    out.flush()
}

/// The error a session id the daemon does not know comes to: the same fact
/// `direct` gets from the daemon as `session_not_found`, with the same exit
/// code (design D4).
#[must_use]
pub fn unknown_session(session: &str) -> CliError {
    CliError::not_found(
        "session_not_found",
        format!(
            "no session `{}` is known to this daemon (`meltemi sessions` lists a project's \
             sessions)",
            clean(session)
        ),
    )
}

/// Whether the session is running now: the daemon's registry, as `status`
/// reports it.
///
/// # Errors
///
/// When the daemon does not answer.
pub async fn running(peer: &Peer, session: &str) -> Result<bool, CliError> {
    let status = peer
        .request(methods::STATUS, &serde_json::json!({}))
        .await
        .map_err(CliError::contract)?;
    let status: StatusResult = serde_json::from_value(status).map_err(CliError::internal)?;
    Ok(status
        .sessions
        .iter()
        .any(|summary| summary.session_id == session))
}

/// The session as the daemon lists it, across every project: `None` when it
/// knows no session by that id.
///
/// # Errors
///
/// When the daemon does not answer.
pub async fn find(peer: &Peer, session: &str) -> Result<Option<SessionInfo>, CliError> {
    let listed = peer
        .request(methods::SESSION_LIST, &SessionListParams::default())
        .await
        .map_err(CliError::contract)?;
    let listed: SessionListResult = serde_json::from_value(listed).map_err(CliError::internal)?;
    Ok(listed
        .sessions
        .into_iter()
        .find(|info| info.session_id == session))
}

/// The lowercase word for a session's state.
#[must_use]
pub fn state_word(info: &SessionInfo) -> String {
    serde_json::to_value(info.state)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// `log`: the session's append-only record, as it is, from its first record to
/// now. The session is found in the daemon's listing, which spans every
/// project, so nobody has to say where it ran.
///
/// # Errors
///
/// An unknown session, a daemon that does not answer, or a stdout that cannot
/// be written.
pub async fn log(
    session: &str,
    endpoint: &str,
    format: Format,
    out: &mut impl Write,
) -> Result<(), CliError> {
    // The receiver is held for the whole read: dropping it would stop the
    // connection's reader at the first notification the daemon sends unasked,
    // and fail the page in flight.
    let (peer, _incoming) = crate::run::connect_reading(endpoint).await?;
    let result = read_log(&peer, session, format, out).await;
    peer.close();
    result
}

async fn read_log(
    peer: &Peer,
    session: &str,
    format: Format,
    out: &mut impl Write,
) -> Result<(), CliError> {
    let Some(info) = find(peer, session).await? else {
        return Err(unknown_session(session));
    };
    let mut offset = 0u32;
    loop {
        let page = peer
            .request(
                methods::SESSION_LOG,
                &SessionLogParams {
                    project_root: info.project_root.clone(),
                    session_id: info.session_id.clone(),
                    offset: Some(offset),
                    limit: Some(PAGE),
                },
            )
            .await
            .map_err(CliError::contract)?;
        let page: SessionLogResult = serde_json::from_value(page).map_err(CliError::internal)?;
        if page.lines.is_empty() {
            return Ok(());
        }
        for line in &page.lines {
            write_record(out, format, line).map_err(CliError::internal)?;
        }
        offset = page.offset + u32::try_from(page.lines.len()).unwrap_or(u32::MAX);
        if offset >= page.total {
            return Ok(());
        }
    }
}

/// `watch`: every record of a live session as it happens, until the session
/// ends (design D6).
///
/// Interest is declared BEFORE the session is looked up: the other order would
/// lose a close that landed in between. It ends after the session's close; or,
/// when a periodic look finds the session gone without its close having come,
/// saying so; or, if the daemon closes the connection, with `10`. Stopping it
/// only stops following: the daemon forgets the interest and the session
/// keeps running.
///
/// # Errors
///
/// An unknown session (`11`), a daemon that goes away (`10`), or a stdout that
/// cannot be written.
pub async fn watch(
    session: &str,
    endpoint: &str,
    format: Format,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<(), CliError> {
    let (peer, mut incoming) = crate::run::connect_reading(endpoint).await?;
    let result = follow(&peer, &mut incoming, session, format, out, err).await;
    peer.close();
    result
}

async fn follow(
    peer: &Peer,
    incoming: &mut UnboundedReceiver<Incoming>,
    session: &str,
    format: Format,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<(), CliError> {
    let shown = clean(session);
    peer.request(
        methods::SESSION_WATCH,
        &SessionWatchParams {
            session_id: session.to_string(),
            watch: true,
        },
    )
    .await
    .map_err(CliError::contract)?;

    if !running(peer, session).await? {
        let Some(info) = find(peer, session).await? else {
            return Err(unknown_session(session));
        };
        let _ = writeln!(
            err,
            "meltemi: session {shown} is not running ({}) — nothing to follow; its record: \
             meltemi log {shown}",
            state_word(&info)
        );
        return Ok(());
    }
    if !format.is_machine() {
        let _ = writeln!(
            err,
            "following session {shown} — Ctrl+C stops following; the session keeps running"
        );
    }

    let gone = || {
        CliError::unreachable(format!(
            "the daemon closed the connection while following session {shown}"
        ))
    };
    let mut look = tokio::time::interval(LIVENESS);
    look.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick is immediate, and the session was just seen running.
    look.tick().await;
    loop {
        tokio::select! {
            message = incoming.recv() => {
                let Some(message) = message else {
                    return Err(gone());
                };
                if handle(peer, message, session, format, out)? == Stream::Ended {
                    return Ok(());
                }
            }
            _ = look.tick() => {
                let alive = running(peer, session).await.map_err(|_| gone())?;
                if alive {
                    continue;
                }
                // Whatever arrived while the daemon was asked is written
                // first: the close may be among it.
                while let Ok(message) = incoming.try_recv() {
                    if handle(peer, message, session, format, out)? == Stream::Ended {
                        return Ok(());
                    }
                }
                let _ = writeln!(
                    err,
                    "meltemi: session {shown} is no longer running and its close did not reach \
                     this stream; its record has it: meltemi log {shown}"
                );
                return Ok(());
            }
        }
    }
}

/// Whether the stream goes on after a message.
#[derive(Debug, PartialEq, Eq)]
enum Stream {
    Continues,
    Ended,
}

/// One message from the daemon: a record of the session is written, and its
/// close ends the stream; a request nobody here serves is refused, so the
/// daemon never waits on this connection for an answer.
/// `sessions --follow`: every affirmation of state the daemon makes, one record
/// per line, until the user stops or the daemon goes.
///
/// Two differences from [`watch`], both of them consequences rather than
/// choices. There is no interest to declare: the daemon affirms every session to
/// every connection, which is the whole point of the channel. And there is no
/// session to outlive, so the only endings are the user's and the daemon's —
/// `watch` needs a liveness poll because it follows one session and must not
/// outlast it (estado-en-vivo design D8).
///
/// The first records are the snapshot the daemon sends on `initialize`, so a
/// reader that starts here knows where everything stands without asking.
pub async fn states(
    project_root: Option<&str>,
    endpoint: &str,
    format: Format,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<(), CliError> {
    let (peer, mut incoming) = crate::run::connect_reading(endpoint).await?;
    let result = follow_states(project_root, &mut incoming, format, out, err).await;
    peer.close();
    result
}

/// Whether `root` contains the session at `own`, by the same rule the surfaces
/// use: separators and case normalized, and a worktree under the root counts.
fn within(root: &str, own: &str) -> bool {
    let norm = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let (root, own) = (norm(root), norm(own));
    own == root || own.starts_with(&format!("{root}/"))
}

async fn follow_states(
    project_root: Option<&str>,
    incoming: &mut UnboundedReceiver<Incoming>,
    format: Format,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<(), CliError> {
    if !format.is_machine() {
        let _ = writeln!(
            err,
            "following every session's state — Ctrl+C stops following; the sessions keep running"
        );
    }
    loop {
        let Some(message) = incoming.recv().await else {
            return Err(CliError::unreachable(
                "the daemon closed the connection while following session state".to_string(),
            ));
        };
        let Incoming::Notification { method, params } = message else {
            continue;
        };
        if method != methods::SESSION_STATE {
            continue;
        }
        if let Some(root) = project_root {
            let own = params.get("projectRoot").and_then(Value::as_str);
            if !own.is_some_and(|own| within(root, own)) {
                continue;
            }
        }
        let line = serde_json::to_string(&params).map_err(CliError::internal)?;
        write_record(out, format, &line).map_err(CliError::internal)?;
    }
}

fn handle(
    peer: &Peer,
    message: Incoming,
    session: &str,
    format: Format,
    out: &mut impl Write,
) -> Result<Stream, CliError> {
    match message {
        Incoming::Notification { method, params } if method == methods::SESSION_EVENT => {
            if params.get("sessionId").and_then(Value::as_str) != Some(session) {
                return Ok(Stream::Continues);
            }
            let record = params.get("event").cloned().unwrap_or(Value::Null);
            let line = serde_json::to_string(&record).map_err(CliError::internal)?;
            write_record(out, format, &line).map_err(CliError::internal)?;
            if record.get("type").and_then(Value::as_str) == Some("session_ended") {
                return Ok(Stream::Ended);
            }
            Ok(Stream::Continues)
        }
        Incoming::Request { id, method, .. } => {
            peer.respond(id, Err(RpcError::method_not_found(&method)));
            Ok(Stream::Continues)
        }
        Incoming::Notification { .. } => Ok(Stream::Continues),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: Una línea del flujo lleva el texto del agente como escape JSON
    #[test]
    fn a_record_line_carries_what_the_terminal_would_obey_as_json_escapes() {
        // What the daemon writes: serde already escapes C0 controls, and leaves
        // the bidi override, the zero-width space and C1 controls raw.
        let record = serde_json::json!({
            "type": "agent_update",
            "payload": { "text": "fine\u{202e}txt.exe \u{200b}\u{1b}[2J \u{9b}31m done" },
        });
        let line = serde_json::to_string(&record).unwrap();
        assert!(line.contains('\u{202e}'), "serde leaves it raw: {line}");

        let mut out = Vec::new();
        write_record(&mut out, Format::Json, &line).unwrap();
        let printed = String::from_utf8(out).unwrap();
        assert_eq!(printed.lines().count(), 1, "one line, one record");
        for obeyed in ['\u{202e}', '\u{200b}', '\u{1b}', '\u{9b}'] {
            assert!(
                !printed.contains(obeyed),
                "{obeyed:?} reached the terminal: {printed:?}"
            );
        }
        assert!(printed.contains("\\u202e"), "{printed}");
        assert!(printed.contains("\\u009b"), "{printed}");
        // Read as JSON, the line keeps exactly the agent's words.
        let back: Value = serde_json::from_str(printed.trim()).expect("still JSON");
        assert_eq!(back, record);
    }

    // Scenario: Un flujo en YAML es un documento por registro
    #[test]
    fn a_yaml_stream_is_one_document_per_record() {
        let mut out = Vec::new();
        for line in [
            r#"{"v":1,"type":"prompt_sent","payload":{"text":"go\u202e"}}"#,
            r#"{"v":1,"type":"session_ended","payload":{"reason":"completed"}}"#,
            "not json at all",
        ] {
            write_record(&mut out, Format::Yaml, line).unwrap();
        }
        let printed = String::from_utf8(out).unwrap();
        let documents: Vec<&str> = printed.split("---\n").skip(1).collect();
        assert_eq!(documents.len(), 3, "{printed}");
        assert!(documents[1].contains("\"session_ended\""), "{printed}");
        assert!(documents[2].contains("\"not json at all\""), "{printed}");
        // The agent's words are escaped inside the quoted scalar, never raw,
        // and the document's own line breaks stay line breaks.
        assert!(!printed.contains('\u{202e}'), "{printed:?}");
        assert!(printed.contains("\\u202e"), "{printed}");
        assert!(!printed.contains('\u{1b}'), "never painted");
    }

    #[test]
    fn escaping_keeps_line_breaks_and_plain_text() {
        assert_eq!(escape_obeyed("a\nb"), "a\nb");
        assert_eq!(escape_obeyed("año · ok"), "año · ok");
        assert_eq!(escape_obeyed("\t\u{7f}"), "\\u0009\\u007f");
    }

    #[test]
    fn an_unknown_session_is_a_contract_error_that_says_where_sessions_are() {
        let error = unknown_session("s-\u{1b}");
        assert_eq!(error.exit, crate::exit::ExitCode::Contract);
        assert_eq!(error.kind, "session_not_found");
        assert!(
            error.message.contains("meltemi sessions"),
            "{}",
            error.message
        );
        assert!(!error.message.contains('\u{1b}'), "escaped");
    }
}
