// SPDX-License-Identifier: Apache-2.0

//! The approver's conversation with the daemon: one question at a time,
//! answered with the agent's own options (design D3), withdrawn the moment it
//! stops standing (D6), stopped for real by Ctrl+C (D11), and never abandoned
//! half-way with the terminal in raw mode (D12). Without a terminal to ask in,
//! every request is refused with the agent's own rejection and a notice that
//! says how to grant without a human (D1).

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;

use meltemi_client::rpc::{Incoming, Peer, RpcError};
use meltemi_proto::{
    PendingPermission, PermissionChangedParams, PermissionDecideParams, PermissionDecideResult,
    PermissionDecideStatus, PermissionOutcome, PermissionPendingResult, PermissionRequestParams,
    PermissionRequestResult, PermissionRule, PermissionTimeoutParams, SessionCancelParams, methods,
};

use super::keys::{Arming, KeySource, Read, TerminalKeys, read_answer};
use super::{
    Answer, Confirmation, Key, Question, Remember, StreamedCalls, Withdrawal, allow_once_option,
    answer_for, calls_for_check, clean, confirmation_for, describe_rule, no_terminal_notice,
    reject_option, rule_to_remember,
};

/// Consecutive reads with keys typed before the question could be read, after
/// which the input is taken for one no person is reading.
const STRIKES: u32 = 3;
/// How long Ctrl+C waits for the daemon to take the request out of the queue.
const STOP_WAIT: Duration = Duration::from_secs(5);

/// How a command answers the permission requests the daemon pushes to it.
///
/// Cloning shares it: every connection of a command asks through the same
/// terminal, one question at a time, and a Ctrl+C on any of them stops them
/// all.
#[derive(Clone)]
pub struct Approver {
    inner: Arc<Inner>,
}

struct Inner {
    /// Where the approver writes: stderr, or a test's buffer. Never stdout.
    out: Mutex<Box<dyn Write + Send>>,
    /// The keys, when there is a terminal to ask in. The source is taken out
    /// for each blocking read and put back; it stays out once the terminal
    /// failed, and from then on requests are refused as without a terminal.
    keys: Option<tokio::sync::Mutex<Option<Box<dyn KeySource>>>>,
    arming: Arming,
    /// Whether the reads change the real terminal's mode.
    terminal: bool,
    /// The human pressed Ctrl+C: the command ends with `13`.
    stopped: AtomicBool,
    /// A blocking read is in raw mode.
    reading: Arc<AtomicBool>,
    /// The stop flag of the read in progress, so settling can end it.
    current: Mutex<Option<Arc<AtomicBool>>>,
}

impl Approver {
    /// Refuses every request, saying why on stderr: the library's default, and
    /// the binary's when there is no terminal to ask in.
    #[must_use]
    pub fn deny() -> Self {
        Self::denying(Box::new(std::io::stderr()))
    }

    /// Refuses every request, saying why on `out`.
    #[must_use]
    pub fn denying(out: Box<dyn Write + Send>) -> Self {
        Self::build(None, out, Arming::TERMINAL, false)
    }

    /// Asks on this process's terminal, writing to stderr.
    #[must_use]
    pub fn terminal() -> Self {
        Self::build(
            Some(Box::new(TerminalKeys::new())),
            Box::new(std::io::stderr()),
            Arming::TERMINAL,
            true,
        )
    }

    /// Asks with these keys, writing to `out`: a test's scripted human.
    #[must_use]
    pub fn asking(keys: Box<dyn KeySource>, out: Box<dyn Write + Send>, arming: Arming) -> Self {
        Self::build(Some(keys), out, arming, false)
    }

    fn build(
        keys: Option<Box<dyn KeySource>>,
        out: Box<dyn Write + Send>,
        arming: Arming,
        terminal: bool,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                out: Mutex::new(out),
                keys: keys.map(|keys| tokio::sync::Mutex::new(Some(keys))),
                arming,
                terminal,
                stopped: AtomicBool::new(false),
                reading: Arc::new(AtomicBool::new(false)),
                current: Mutex::new(None),
            }),
        }
    }

    /// Whether the human stopped the command with Ctrl+C at a question.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.inner.stopped.load(Ordering::SeqCst)
    }

    /// Makes sure no read is left in raw mode: ends the one in progress, waits
    /// for it, and as a last resort leaves raw mode from here (design D12).
    pub async fn settle(&self) {
        if let Some(stop) = lock(&self.inner.current).as_ref() {
            stop.store(true, Ordering::SeqCst);
        }
        for _ in 0..100 {
            if !self.inner.reading.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.restore();
    }

    /// Leaves raw mode if a read was left in it. Synchronous, for the last
    /// line before the process exits, which runs no destructors.
    pub fn restore(&self) {
        if self.inner.terminal && self.inner.reading.load(Ordering::SeqCst) {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
        }
    }

    /// Serves one connection until it closes or `closing` is signalled:
    /// answers every permission request the daemon pushes, and refuses every
    /// other request it has no handler for.
    pub(crate) async fn serve(
        &self,
        peer: Peer,
        mut incoming: UnboundedReceiver<Incoming>,
        closing: Arc<Notify>,
        root: Option<String>,
    ) {
        let mut talk = Talk {
            peer: &peer,
            incoming: &mut incoming,
            closing: &closing,
            waiting: VecDeque::new(),
            calls: StreamedCalls::default(),
            root,
            ended: false,
        };
        while !talk.ended {
            let push = match talk.waiting.pop_front() {
                Some(push) => push,
                None => match talk.next_push().await {
                    Some(push) => push,
                    None => break,
                },
            };
            let (id, params) = push;
            let outcome = self.answer(&mut talk, params).await;
            peer.respond(
                id,
                Ok(serde_json::to_value(PermissionRequestResult { outcome })
                    .expect("a decision serializes")),
            );
        }
    }

    /// Writes one line (or block) of the approver's own.
    fn say(&self, text: &str) {
        let mut out = lock(&self.inner.out);
        let _ = out.write_all(text.as_bytes());
        if !text.ends_with('\n') {
            let _ = out.write_all(b"\n");
        }
        let _ = out.flush();
    }

    /// The answer to one push.
    async fn answer(&self, talk: &mut Talk<'_>, params: Value) -> PermissionOutcome {
        let Ok(push) = serde_json::from_value::<PermissionRequestParams>(params) else {
            self.say("meltemi: a malformed permission request was denied");
            return PermissionOutcome::Cancelled;
        };
        let streamed = talk.streamed(&push);
        let question = Question::from_push(&push, streamed.as_ref());
        let Some(keys) = &self.inner.keys else {
            self.say(&no_terminal_notice(&question));
            return denial(&question);
        };
        if self.stopped() {
            self.say(&format!(
                "meltemi: permission requested by session {} denied without asking: the \
                 session was stopped",
                clean(&question.session_id)
            ));
            return PermissionOutcome::Cancelled;
        }
        // One question at a time, whichever connection it came from.
        let mut slot = keys.lock().await;
        if slot.is_none() {
            self.say(&no_terminal_notice(&question));
            return denial(&question);
        }
        if self.stopped() {
            return PermissionOutcome::Cancelled;
        }
        // Does it still stand? The answer postdates the request, because the
        // push does (design D6).
        let entry = match &question.request_id {
            Some(request_id) => match lookup(talk.peer, request_id).await {
                Lookup::Entry(entry) if entry.expired => {
                    self.withdrawn_unasked(&question, Withdrawal::Expired);
                    return PermissionOutcome::Cancelled;
                }
                Lookup::Entry(entry) => Some(*entry),
                Lookup::Absent => {
                    self.withdrawn_unasked(&question, Withdrawal::Resolved);
                    return PermissionOutcome::Cancelled;
                }
                Lookup::Unknown => None,
            },
            None => None,
        };
        self.ask(talk, &mut slot, &push, entry).await
    }

    /// A request that stopped standing before it could be asked.
    fn withdrawn_unasked(&self, question: &Question, why: Withdrawal) {
        let tool = question
            .shown
            .tool
            .as_deref()
            .map_or_else(String::new, |tool| format!(" ({})", clean(tool)));
        self.say(&format!(
            "permission requested by session {}{tool}\n  → {}",
            clean(&question.session_id),
            why.words()
        ));
    }

    /// Asks the human until a key decides, the request stops standing, or the
    /// command ends.
    async fn ask(
        &self,
        talk: &mut Talk<'_>,
        slot: &mut Option<Box<dyn KeySource>>,
        push: &PermissionRequestParams,
        entry: Option<PendingPermission>,
    ) -> PermissionOutcome {
        let streamed = talk.streamed(push);
        let mut question = Question::from_push(push, streamed.as_ref());
        let mut remember = rule_to_remember(&question, entry.as_ref(), talk.root.as_deref());
        if let Some(source) = slot.as_mut() {
            source.begin();
        }
        let mut strikes = 0;
        'show: loop {
            self.say(&question.render(&remember));
            loop {
                let key = match self.read(talk, slot, &question, push).await {
                    Reading::Key(key) => key,
                    Reading::Early => {
                        strikes += 1;
                        if strikes >= STRIKES {
                            self.say(UNATTENDED);
                            return denial(&question);
                        }
                        self.say(EARLY);
                        continue;
                    }
                    Reading::Burst => {
                        self.say(BURST);
                        continue;
                    }
                    Reading::Redraw(updated) => {
                        question = *updated;
                        remember =
                            rule_to_remember(&question, entry.as_ref(), talk.root.as_deref());
                        continue 'show;
                    }
                    Reading::Withdrawn(why) => {
                        self.say(&format!("  → {}", why.words()));
                        return PermissionOutcome::Cancelled;
                    }
                    Reading::Failed => {
                        *slot = None;
                        return self.failed(&question);
                    }
                };
                strikes = 0;
                let offered = matches!(remember, Remember::Offered(_));
                match answer_for(key, &question, offered) {
                    Answer::Nothing => {
                        if key == Key::Paste {
                            self.say(PASTE);
                        }
                    }
                    Answer::Choose(option_id) => {
                        self.say(&format!("  → {}", question.describe_option(&option_id)));
                        return selected(option_id);
                    }
                    Answer::ChooseConfirmed(option_id) => {
                        self.say(&format!(
                            "  {}: with it the agent stops asking about similar requests for the \
                             rest of this session, and those never reach Meltemi's permissions\n  \
                             y confirm   any other key goes back",
                            question.describe_option(&option_id)
                        ));
                        match self.confirm(talk, slot, &question, push).await {
                            Confirmed::Yes => {
                                self.say(&format!("  → {}", question.describe_option(&option_id)));
                                return selected(option_id);
                            }
                            Confirmed::Back => continue 'show,
                            Confirmed::Stop => return self.stop(talk, &question).await,
                            Confirmed::Over(outcome) => return outcome,
                        }
                    }
                    Answer::Deny(option_id) => {
                        match &option_id {
                            Some(option_id) => self.say(&format!(
                                "  → denied: {}",
                                question.describe_option(option_id)
                            )),
                            None => self.say(
                                "  → denied: the agent offered no reject option, so the request \
                                 was cancelled",
                            ),
                        }
                        return option_id.map_or(PermissionOutcome::Cancelled, selected);
                    }
                    Answer::Remember => {
                        let Remember::Offered(rule) = &remember else {
                            continue;
                        };
                        self.say(&format!(
                            "  remember: {}\n  it governs from the next session opened without \
                             --mode; this session keeps asking\n  y confirm   any other key goes \
                             back",
                            describe_rule(rule)
                        ));
                        match self.confirm(talk, slot, &question, push).await {
                            Confirmed::Yes => return self.remember(talk, &question, rule).await,
                            Confirmed::Back => continue 'show,
                            Confirmed::Stop => return self.stop(talk, &question).await,
                            Confirmed::Over(outcome) => return outcome,
                        }
                    }
                    Answer::Stop => return self.stop(talk, &question).await,
                }
            }
        }
    }

    /// Reads the key after a confirmation prompt.
    async fn confirm(
        &self,
        talk: &mut Talk<'_>,
        slot: &mut Option<Box<dyn KeySource>>,
        question: &Question,
        push: &PermissionRequestParams,
    ) -> Confirmed {
        let mut strikes = 0;
        loop {
            match self.read(talk, slot, question, push).await {
                Reading::Key(key) => {
                    return match confirmation_for(key) {
                        Confirmation::Confirm => Confirmed::Yes,
                        Confirmation::Stop => Confirmed::Stop,
                        Confirmation::Back => Confirmed::Back,
                    };
                }
                Reading::Early => {
                    strikes += 1;
                    if strikes >= STRIKES {
                        self.say(UNATTENDED);
                        return Confirmed::Over(denial(question));
                    }
                    self.say(EARLY);
                }
                Reading::Burst => self.say(BURST),
                // What the agent said changed: show the question again.
                Reading::Redraw(_) => return Confirmed::Back,
                Reading::Withdrawn(why) => {
                    self.say(&format!("  → {}", why.words()));
                    return Confirmed::Over(PermissionOutcome::Cancelled);
                }
                Reading::Failed => {
                    *slot = None;
                    return Confirmed::Over(self.failed(question));
                }
            }
        }
    }

    /// The terminal could not be read: this request and every later one are
    /// refused as without a terminal.
    fn failed(&self, question: &Question) -> PermissionOutcome {
        self.say(
            "  → denied: the terminal could not be read; later requests are denied as if there \
             were no terminal",
        );
        denial(question)
    }

    /// Ctrl+C (design D11): cancel the session first, so the daemon takes the
    /// request out of the queue and the log records it interrupted; answer
    /// only after that, or after a bound.
    async fn stop(&self, talk: &mut Talk<'_>, question: &Question) -> PermissionOutcome {
        self.inner.stopped.store(true, Ordering::SeqCst);
        self.say(
            "  → stopping: the session is being cancelled; press Ctrl+C again to leave without \
             waiting",
        );
        talk.peer.notify(
            methods::SESSION_CANCEL,
            &SessionCancelParams {
                session_id: question.session_id.clone(),
            },
        );
        if let Some(request_id) = &question.request_id {
            let deadline = Instant::now() + STOP_WAIT;
            while Instant::now() < deadline {
                match lookup(talk.peer, request_id).await {
                    Lookup::Entry(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                    Lookup::Absent | Lookup::Unknown => break,
                }
            }
        }
        PermissionOutcome::Cancelled
    }

    /// `r` + `y` (design D5): persist the rule the way the tray does, then
    /// approve once with the same option — the first resolution wins, and the
    /// push's answer after it changes nothing.
    async fn remember(
        &self,
        talk: &mut Talk<'_>,
        question: &Question,
        rule: &PermissionRule,
    ) -> PermissionOutcome {
        let (Some(option), Some(request_id)) = (
            allow_once_option(&question.options),
            question.request_id.clone(),
        ) else {
            // Offered only with both; kept total all the same.
            return PermissionOutcome::Cancelled;
        };
        let option_id = option.option_id.clone();
        let decided = talk
            .peer
            .request(
                methods::PERMISSION_DECIDE,
                &PermissionDecideParams {
                    request_id,
                    option_id: Some(option_id.clone()),
                    persist_rule: Some(rule.clone()),
                },
            )
            .await
            .ok()
            .and_then(|value| serde_json::from_value::<PermissionDecideResult>(value).ok());
        let chosen = question.describe_option(&option_id);
        match decided.map(|result| result.status) {
            Some(PermissionDecideStatus::Applied) => self.say(&format!(
                "  → {chosen}, and remembered: {} — from the next session opened without --mode",
                describe_rule(rule)
            )),
            Some(PermissionDecideStatus::AlreadyResolved) => self.say(
                "  → it had already been resolved elsewhere; the rule may not have been written — \
                 check .meltemi/permissions.toml",
            ),
            None => self.say(&format!(
                "  → {chosen}; the rule could not be sent to the daemon and was not remembered"
            )),
        }
        selected(option_id)
    }

    /// Reads one key while the question is open, reading the connection at
    /// the same time: a withdrawal or a redraw ends the read, and the source
    /// always comes back out of raw mode before this returns.
    async fn read(
        &self,
        talk: &mut Talk<'_>,
        slot: &mut Option<Box<dyn KeySource>>,
        question: &Question,
        push: &PermissionRequestParams,
    ) -> Reading {
        let Some(mut source) = slot.take() else {
            return Reading::Failed;
        };
        let stop = Arc::new(AtomicBool::new(false));
        *lock(&self.inner.current) = Some(Arc::clone(&stop));
        let arming = self.inner.arming;
        let reading = Arc::clone(&self.inner.reading);
        reading.store(true, Ordering::SeqCst);
        let mut reader = tokio::task::spawn_blocking({
            let stop = Arc::clone(&stop);
            move || {
                let read = read_answer(source.as_mut(), &stop, &arming);
                reading.store(false, Ordering::SeqCst);
                (source, read)
            }
        });
        let mut interruption: Option<Reading> = None;
        let joined = loop {
            tokio::select! {
                biased;
                joined = &mut reader => break joined,
                message = talk.incoming.recv(), if interruption.is_none() => {
                    interruption = self.during(talk, message, question, push).await;
                    if interruption.is_some() {
                        stop.store(true, Ordering::SeqCst);
                    }
                }
                () = talk.closing.notified(), if interruption.is_none() => {
                    talk.ended = true;
                    interruption = Some(Reading::Withdrawn(Withdrawal::CommandEnded));
                    stop.store(true, Ordering::SeqCst);
                }
            }
        };
        *lock(&self.inner.current) = None;
        let read = match joined {
            Ok((source, read)) => {
                *slot = Some(source);
                read
            }
            // A reader that panicked took the source with it.
            Err(_) => Read::Failed,
        };
        if let Some(interruption) = interruption {
            return interruption;
        }
        match read {
            Read::Key(key) => Reading::Key(key),
            Read::Early => Reading::Early,
            Read::Burst => Reading::Burst,
            Read::Stopped => Reading::Withdrawn(Withdrawal::CommandEnded),
            Read::Failed => Reading::Failed,
        }
    }

    /// What a message that arrives while a question is open means for it.
    async fn during(
        &self,
        talk: &mut Talk<'_>,
        message: Option<Incoming>,
        question: &Question,
        push: &PermissionRequestParams,
    ) -> Option<Reading> {
        let Some(message) = message else {
            talk.ended = true;
            return Some(Reading::Withdrawn(Withdrawal::ConnectionClosed));
        };
        let (method, params) = match message {
            Incoming::Request { id, method, params } if method == methods::PERMISSION_REQUEST => {
                talk.waiting.push_back((id, params));
                return None;
            }
            Incoming::Request { id, method, .. } => {
                talk.peer
                    .respond(id, Err(RpcError::method_not_found(&method)));
                return None;
            }
            Incoming::Notification { method, params } => (method, params),
        };
        match method.as_str() {
            // The call asked about, streamed late: show what it now says.
            methods::SESSION_EVENT => {
                let (session, id) = talk.calls.observe(&params)?;
                if session != question.session_id
                    || Some(id.as_str()) != question.tool_call_id.as_deref()
                {
                    return None;
                }
                let updated =
                    Question::from_push(push, talk.calls.get(&session, Some(id.as_str())));
                (updated != *question).then(|| Reading::Redraw(Box::new(updated)))
            }
            // A snapshot without the request, or with it expired, is checked:
            // it may predate the request (design D6).
            methods::PERMISSION_CHANGED => {
                let request_id = question.request_id.as_deref()?;
                let snapshot = serde_json::from_value::<PermissionChangedParams>(params).ok()?;
                if !calls_for_check(&snapshot.pending, request_id) {
                    return None;
                }
                check(talk.peer, request_id).await
            }
            methods::PERMISSION_TIMEOUT => {
                let timeout = serde_json::from_value::<PermissionTimeoutParams>(params).ok()?;
                if timeout.session_id != question.session_id {
                    return None;
                }
                if let (Some(expired), Some(asked)) =
                    (&timeout.tool_call_id, &question.tool_call_id)
                    && expired != asked
                {
                    return None;
                }
                match &question.request_id {
                    Some(request_id) => check(talk.peer, request_id).await,
                    // A daemon without queue ids: the timeout is all there is.
                    None => Some(Reading::Withdrawn(Withdrawal::Expired)),
                }
            }
            _ => None,
        }
    }
}

/// What the approver carries through one connection.
struct Talk<'a> {
    peer: &'a Peer,
    incoming: &'a mut UnboundedReceiver<Incoming>,
    closing: &'a Notify,
    /// Pushes that arrived while another question was open, in order.
    waiting: VecDeque<(Value, Value)>,
    /// The tool calls the sessions streamed, to complete a question with.
    calls: StreamedCalls,
    /// The project a remembered rule is written for, when the command's
    /// sessions work there.
    root: Option<String>,
    /// The command is ending, or the connection closed.
    ended: bool,
}

impl Talk<'_> {
    /// Waits for the next push, keeping the tool calls streamed meanwhile and
    /// refusing requests nobody here handles. `None` once the connection
    /// closes or the command ends.
    async fn next_push(&mut self) -> Option<(Value, Value)> {
        loop {
            let message = tokio::select! {
                biased;
                message = self.incoming.recv() => message,
                () = self.closing.notified() => None,
            };
            let Some(message) = message else {
                self.ended = true;
                return None;
            };
            match message {
                Incoming::Request { id, method, params }
                    if method == methods::PERMISSION_REQUEST =>
                {
                    return Some((id, params));
                }
                Incoming::Request { id, method, .. } => {
                    self.peer
                        .respond(id, Err(RpcError::method_not_found(&method)));
                }
                Incoming::Notification { method, params } => {
                    if method == methods::SESSION_EVENT {
                        self.calls.observe(&params);
                    }
                }
            }
        }
    }

    /// What the session streamed for the call a push asks about.
    fn streamed(&self, push: &PermissionRequestParams) -> Option<Value> {
        let id = push.tool_call.get("toolCallId").and_then(Value::as_str);
        self.calls.get(&push.session_id, id).cloned()
    }
}

/// What one read while a question is open comes to.
enum Reading {
    Key(Key),
    Early,
    Burst,
    /// What the agent said about the call changed: show it again.
    Redraw(Box<Question>),
    Withdrawn(Withdrawal),
    Failed,
}

/// What the key after a confirmation prompt comes to.
enum Confirmed {
    Yes,
    Back,
    Stop,
    /// The question ended another way, with this answer.
    Over(PermissionOutcome),
}

/// The request as the daemon's queue shows it now.
enum Lookup {
    Entry(Box<PendingPermission>),
    Absent,
    /// The queue could not be read.
    Unknown,
}

async fn lookup(peer: &Peer, request_id: &str) -> Lookup {
    let Ok(value) = peer.request(methods::PERMISSION_PENDING, &json!({})).await else {
        return Lookup::Unknown;
    };
    let Ok(result) = serde_json::from_value::<PermissionPendingResult>(value) else {
        return Lookup::Unknown;
    };
    result
        .pending
        .into_iter()
        .find(|entry| entry.request_id == request_id)
        .map_or(Lookup::Absent, |entry| Lookup::Entry(Box::new(entry)))
}

/// Whether a request that a snapshot called into question stopped standing.
async fn check(peer: &Peer, request_id: &str) -> Option<Reading> {
    match lookup(peer, request_id).await {
        Lookup::Absent => Some(Reading::Withdrawn(Withdrawal::Resolved)),
        Lookup::Entry(entry) if entry.expired => Some(Reading::Withdrawn(Withdrawal::Expired)),
        Lookup::Entry(_) | Lookup::Unknown => None,
    }
}

/// The denial the agent is told: its own rejection, or a cancellation when it
/// offered none.
fn denial(question: &Question) -> PermissionOutcome {
    reject_option(&question.options).map_or(PermissionOutcome::Cancelled, |option| {
        selected(option.option_id.clone())
    })
}

fn selected(option_id: String) -> PermissionOutcome {
    PermissionOutcome::Selected { option_id }
}

/// A lock that survives a writer that panicked: the approver's state stays
/// usable, since every write to it is a whole value.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

const EARLY: &str = "  (keys typed before the question could be read were ignored — press again)";
const BURST: &str =
    "  (keys that arrived together were ignored — a paste, or several keys at once; press one key)";
const PASTE: &str = "  (a paste was ignored — press one key)";
const UNATTENDED: &str = "  → denied: keys keep arriving before the question can be read, as if \
                          no person were reading it";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approver::keys::{RawCounts, Scripted, ScriptedKeys};
    use meltemi_proto::{
        PermissionOption, PermissionOptionKind, PermissionRuleEffect, PermissionRuleScope,
    };
    use tokio::io::{
        AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, Lines, ReadHalf, WriteHalf,
    };

    /// What the approver wrote, for the test to read.
    #[derive(Clone, Default)]
    struct Said(Arc<Mutex<Vec<u8>>>);

    impl Write for Said {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            lock(&self.0).extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Said {
        fn text(&self) -> String {
            String::from_utf8(lock(&self.0).clone()).expect("the approver writes text")
        }
    }

    /// The daemon's side of an in-memory connection, played by the test.
    struct Daemon {
        lines: Lines<BufReader<ReadHalf<DuplexStream>>>,
        write: WriteHalf<DuplexStream>,
    }

    impl Daemon {
        async fn send(&mut self, message: Value) {
            let mut line = serde_json::to_vec(&message).expect("a message serializes");
            line.push(b'\n');
            self.write
                .write_all(&line)
                .await
                .expect("the approver reads");
        }

        async fn push(&mut self, id: u64, params: Value) {
            self.send(json!({
                "jsonrpc": "2.0", "id": id, "method": methods::PERMISSION_REQUEST, "params": params
            }))
            .await;
        }

        async fn notify(&mut self, method: &str, params: Value) {
            self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
                .await;
        }

        async fn next(&mut self) -> Value {
            let line = tokio::time::timeout(Duration::from_secs(10), self.lines.next_line())
                .await
                .expect("the approver spoke in time")
                .expect("the connection reads")
                .expect("a line");
            serde_json::from_str(&line).expect("a JSON message")
        }

        /// Expects `permission/pending` and answers it with this queue.
        async fn queue(&mut self, pending: Vec<PendingPermission>) {
            let asked = self.next().await;
            assert_eq!(asked["method"], methods::PERMISSION_PENDING, "{asked}");
            self.send(json!({
                "jsonrpc": "2.0", "id": asked["id"], "result": { "pending": pending }
            }))
            .await;
        }
    }

    fn start(
        approver: &Approver,
        root: Option<&str>,
    ) -> (Daemon, Arc<Notify>, tokio::task::JoinHandle<()>) {
        let (ours, theirs) = tokio::io::duplex(64 * 1024);
        let (peer, incoming) = Peer::start(Box::new(ours));
        let closing = Arc::new(Notify::new());
        let task = tokio::spawn({
            let approver = approver.clone();
            let closing = Arc::clone(&closing);
            let root = root.map(str::to_string);
            async move { approver.serve(peer, incoming, closing, root).await }
        });
        let (read, write) = tokio::io::split(theirs);
        let daemon = Daemon {
            lines: BufReader::new(read).lines(),
            write,
        };
        (daemon, closing, task)
    }

    fn scripted(script: impl IntoIterator<Item = Scripted>) -> (Approver, Said, RawCounts) {
        let keys = ScriptedKeys::new(script);
        let counts = keys.counts();
        let said = Said::default();
        let approver = Approver::asking(Box::new(keys), Box::new(said.clone()), Arming::SCRIPTED);
        (approver, said, counts)
    }

    fn options() -> Vec<PermissionOption> {
        vec![
            PermissionOption {
                option_id: "allow".into(),
                name: "Allow".into(),
                kind: PermissionOptionKind::AllowOnce,
            },
            PermissionOption {
                option_id: "reject".into(),
                name: "Reject".into(),
                kind: PermissionOptionKind::RejectOnce,
            },
        ]
    }

    fn request(session: &str, request_id: &str, call: Value) -> Value {
        json!({
            "sessionId": session, "toolCall": call, "options": options(), "requestId": request_id
        })
    }

    fn entry(session: &str, request_id: &str) -> PendingPermission {
        PendingPermission {
            request_id: request_id.into(),
            session_id: session.into(),
            tool: "edit".into(),
            summary: "edit".into(),
            options: options(),
            waiting_seconds: 0,
            expires_in_seconds: None,
            expired: false,
            suggested_rule: None,
        }
    }

    fn outcome(answered: &Value) -> PermissionOutcome {
        serde_json::from_value(answered["result"]["outcome"].clone()).expect("an outcome")
    }

    fn chose(option: &str) -> PermissionOutcome {
        selected(option.to_string())
    }

    async fn ends(task: tokio::task::JoinHandle<()>) {
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .expect("the approver ends")
            .expect("the approver does not panic");
    }

    #[tokio::test]
    async fn a_snapshot_older_than_the_request_does_not_withdraw_it() {
        // Scenario: Una instantánea anterior a la petición no la retira
        let (approver, said, counts) = scripted([Scripted::Hold]);
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;

        // A snapshot taken before the request was registered, delivered after
        // the push: it is checked, and the request still stands.
        daemon
            .notify(methods::PERMISSION_CHANGED, json!({ "pending": [] }))
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;

        // Resolved on another surface: now the absence means it.
        daemon
            .notify(methods::PERMISSION_CHANGED, json!({ "pending": [] }))
            .await;
        daemon.queue(vec![]).await;
        let answered = daemon.next().await;
        assert_eq!(answered["id"], 100);
        assert_eq!(outcome(&answered), PermissionOutcome::Cancelled);
        let text = said.text();
        assert_eq!(text.matches("withdrawn").count(), 1, "{text}");
        assert!(text.contains("resolved elsewhere"), "{text}");

        drop(daemon);
        ends(task).await;
        assert_eq!(counts.entered(), counts.left());
    }

    #[tokio::test]
    async fn a_second_request_waits_for_the_first_answer() {
        let (approver, said, _) =
            scripted([Scripted::Key(Key::Char('1')), Scripted::Key(Key::Char('2'))]);
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon
            .push(
                101,
                request(
                    "s2",
                    "perm-2",
                    json!({ "toolCallId": "c2", "kind": "execute" }),
                ),
            )
            .await;
        daemon
            .queue(vec![entry("s1", "perm-1"), entry("s2", "perm-2")])
            .await;
        let first = daemon.next().await;
        assert_eq!(first["id"], 100);
        assert_eq!(outcome(&first), chose("allow"));
        daemon.queue(vec![entry("s2", "perm-2")]).await;
        let second = daemon.next().await;
        assert_eq!(second["id"], 101);
        assert_eq!(outcome(&second), chose("reject"));

        let text = said.text();
        let answered = text.find("→ Allow (allow once)").expect("the first answer");
        let asked = text.find("by session s2").expect("the second question");
        assert!(answered < asked, "one question at a time:\n{text}");
        drop(daemon);
        ends(task).await;
    }

    #[tokio::test]
    async fn ctrl_c_cancels_the_session_before_answering_and_asks_nothing_more() {
        let (approver, said, _) = scripted([Scripted::Key(Key::CtrlC)]);
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;

        // The cancellation goes first, as a notification…
        let cancel = daemon.next().await;
        assert_eq!(cancel["method"], methods::SESSION_CANCEL, "{cancel}");
        assert_eq!(cancel["params"]["sessionId"], "s1");
        assert!(cancel.get("id").is_none(), "a notification: {cancel}");
        // …and the push is answered only once the daemon took the request out.
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        daemon.queue(vec![]).await;
        let answered = daemon.next().await;
        assert_eq!(answered["id"], 100);
        assert_eq!(outcome(&answered), PermissionOutcome::Cancelled);
        assert!(approver.stopped());

        // A later request is refused without a question.
        daemon
            .push(
                101,
                request(
                    "s1",
                    "perm-2",
                    json!({ "toolCallId": "c2", "kind": "edit" }),
                ),
            )
            .await;
        let refused = daemon.next().await;
        assert_eq!(refused["id"], 101);
        assert_eq!(outcome(&refused), PermissionOutcome::Cancelled);
        assert!(
            said.text().contains("denied without asking"),
            "{}",
            said.text()
        );
        drop(daemon);
        ends(task).await;
    }

    #[tokio::test]
    async fn remembering_decides_by_id_with_the_rule_then_answers_the_push() {
        let (approver, said, _) =
            scripted([Scripted::Key(Key::Char('r')), Scripted::Key(Key::Char('y'))]);
        let (mut daemon, _closing, task) = start(&approver, Some("/repo"));
        let call = json!({
            "toolCallId": "c1", "kind": "edit", "rawInput": { "path": "/repo/notes/a.md" }
        });
        daemon.push(100, request("s1", "perm-1", call)).await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        let decide = daemon.next().await;
        assert_eq!(decide["method"], methods::PERMISSION_DECIDE, "{decide}");
        let rule = PermissionRule {
            effect: PermissionRuleEffect::Allow,
            tool: Some("edit".into()),
            command_prefix: None,
            path_prefix: Some("/repo/notes/a.md".into()),
            scope: PermissionRuleScope::Project,
        };
        assert_eq!(
            decide["params"],
            json!({ "requestId": "perm-1", "optionId": "allow", "persistRule": rule })
        );
        daemon
            .send(json!({
                "jsonrpc": "2.0", "id": decide["id"], "result": { "status": "applied" }
            }))
            .await;
        let answered = daemon.next().await;
        assert_eq!(answered["id"], 100);
        assert_eq!(outcome(&answered), chose("allow"));
        let text = said.text();
        assert!(
            text.contains("remember: allow tool edit, paths starting with /repo/notes/a.md"),
            "{text}"
        );
        assert!(
            text.contains("from the next session opened without --mode"),
            "{text}"
        );
        drop(daemon);
        ends(task).await;
    }

    #[tokio::test]
    async fn keys_before_the_question_can_be_read_answer_nothing() {
        // Scenario: Lo tecleado antes de poder leer la pregunta no la contesta
        let (approver, said, _) = scripted([
            Scripted::Early(Key::Char('1')),
            Scripted::Key(Key::Char('2')),
        ]);
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        let answered = daemon.next().await;
        assert_eq!(
            outcome(&answered),
            chose("reject"),
            "the early `1` chose nothing"
        );
        assert!(
            said.text()
                .contains("typed before the question could be read")
        );
        drop(daemon);
        ends(task).await;

        // Input that never stops is taken for no person at all: denied.
        let (approver, said, counts) = scripted([Scripted::Stream(Key::Char('1'))]);
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        let answered = daemon.next().await;
        assert_eq!(outcome(&answered), chose("reject"));
        assert!(
            said.text().contains("as if no person were reading it"),
            "{}",
            said.text()
        );
        drop(daemon);
        ends(task).await;
        assert_eq!(counts.entered(), counts.left());
    }

    #[tokio::test]
    async fn a_call_streamed_late_redraws_the_question() {
        let (approver, said, _) = scripted([Scripted::Hold, Scripted::Key(Key::Char('d'))]);
        let (mut daemon, _closing, task) = start(&approver, None);
        let call = json!({ "toolCallId": "item-7", "kind": "execute" });
        daemon.push(100, request("s1", "perm-1", call)).await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        let streamed = json!({
            "sessionId": "s1",
            "event": { "v": 1, "ts": "t", "type": "agent_update", "payload": { "update": {
                "sessionUpdate": "tool_call", "toolCallId": "item-7", "kind": "execute",
                "title": "rm -rf target", "rawInput": { "command": "rm -rf target" }
            } } }
        });
        daemon.notify(methods::SESSION_EVENT, streamed).await;
        let answered = daemon.next().await;
        assert_eq!(outcome(&answered), chose("reject"));
        let text = said.text();
        assert_eq!(
            text.matches("permission requested by session s1").count(),
            2,
            "{text}"
        );
        let redrawn = text
            .rfind("permission requested by session s1")
            .expect("redrawn");
        assert!(!text[..redrawn].contains("rm -rf target"), "{text}");
        assert!(
            text[redrawn..].contains("  command  rm -rf target"),
            "{text}"
        );
        drop(daemon);
        ends(task).await;
    }

    #[tokio::test]
    async fn a_question_open_when_the_command_ends_is_withdrawn_out_of_raw_mode() {
        let (approver, said, counts) = scripted([Scripted::Hold]);
        let (mut daemon, closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        daemon.queue(vec![entry("s1", "perm-1")]).await;
        while counts.entered() == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        closing.notify_one();
        let answered = daemon.next().await;
        assert_eq!(outcome(&answered), PermissionOutcome::Cancelled);
        ends(task).await;
        assert_eq!(
            counts.entered(),
            counts.left(),
            "raw mode left before the end"
        );
        assert!(
            said.text().contains("withdrawn: the command ended"),
            "{}",
            said.text()
        );
    }

    #[tokio::test]
    async fn without_a_terminal_the_agents_rejection_answers() {
        let said = Said::default();
        let approver = Approver::denying(Box::new(said.clone()));
        let (mut daemon, _closing, task) = start(&approver, None);
        daemon
            .push(
                100,
                request(
                    "s1",
                    "perm-1",
                    json!({ "toolCallId": "c1", "kind": "edit" }),
                ),
            )
            .await;
        // No queue lookup, no question: the agent's own rejection, at once.
        let answered = daemon.next().await;
        assert_eq!(answered["id"], 100);
        assert_eq!(outcome(&answered), chose("reject"));
        let text = said.text();
        assert!(text.contains("no terminal to ask in"), "{text}");
        assert!(text.contains(".meltemi/permissions.toml"), "{text}");
        drop(daemon);
        ends(task).await;
    }
}
