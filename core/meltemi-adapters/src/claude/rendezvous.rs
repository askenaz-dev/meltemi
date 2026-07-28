// SPDX-License-Identifier: Apache-2.0

//! The private channel between the adapter and its own permission shim
//! (adaptadores-propios-acp task 3.3, design D5 as amended).
//!
//! The shim is this same binary, launched **by the piloted CLI** as one of its
//! MCP servers — which is what makes the channel a problem worth a module. The
//! design named an anonymous pipe inherited at launch; that cannot work here,
//! because the shim is not this process's child. It is a grandchild, spawned by
//! a runtime Meltemi does not control, and no handle of ours survives that trip
//! on Windows at all. The amendment is written up in the change's design; what
//! replaces the pipe keeps every property the pipe was there for:
//!
//! - **Private to the user.** A directory the adapter creates for one session,
//!   `0700` where the platform has modes, and inside the per-user temporary
//!   directory where it does not.
//! - **No port, no listener.** Nothing accepts connections; nothing is
//!   reachable by anything that cannot already read the user's own files.
//! - **No transport in the daemon.** The daemon never learns this exists: what
//!   reaches it is `session/request_permission`, exactly as from any agent.
//!
//! The cost is a poll, and it is paid where nobody can notice it: the thing
//! being waited on is a human deciding.
//!
//! One rule governs everything here. **A request that cannot be answered is
//! denied.** The shim denies when the adapter is gone, when the wait runs out,
//! when anything at all fails; there is no path through this module on which
//! silence becomes consent.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::Value;

/// How often each side looks for the other's file.
///
/// Short enough to be invisible next to a human deciding, long enough that a
/// session waiting on an approval costs nothing measurable.
const POLL: Duration = Duration::from_millis(20);

/// How long the shim waits for a decision before denying.
///
/// Generous, because the answer may be a human who stepped away, and the CLI is
/// blocked on it anyway; finite, because a tool call that waits forever holds
/// the session open forever. The adapter going away is detected long before
/// this: its directory disappears.
const SHIM_PATIENCE: Duration = Duration::from_secs(60 * 60);

/// The prefix of a file the shim writes to ask.
const ASK: &str = "ask-";
/// The prefix of a file the adapter writes to answer.
const ANSWER: &str = "answer-";
/// The suffix of a file that is still being written. Nothing reads one: both
/// sides write to it and rename, and a rename inside one directory is atomic on
/// every platform Meltemi supports.
const PARTIAL: &str = ".partial";

/// One question the shim is waiting on an answer for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The file the question arrived in, which is deleted when it is answered.
    path: PathBuf,
    /// The tool the CLI wants to run.
    pub tool: String,
    /// The id of the call, when the CLI names one.
    pub tool_use_id: Option<String>,
    /// The arguments the tool would run with.
    pub input: Value,
}

/// The adapter's side of the channel: the directory it owns for one session.
#[derive(Debug)]
pub struct Rendezvous {
    dir: PathBuf,
}

impl Rendezvous {
    /// Creates the directory this session's shim will find.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the directory cannot be created or hardened.
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        harden(&dir)?;
        Ok(Self { dir })
    }

    /// The address the shim is told to find, as a string a command line can
    /// carry.
    #[must_use]
    pub fn address(&self) -> &Path {
        &self.dir
    }

    /// Waits for the next question, however long that takes.
    ///
    /// Cancellation safe, and deliberately so: this is a branch of the turn
    /// loop, and losing the race to another branch must not lose a question. A
    /// question is only removed when it is answered, so a poll that was
    /// interrupted finds it again.
    pub async fn next_ask(&self) -> Ask {
        loop {
            if let Some(ask) = self.pending() {
                return ask;
            }
            tokio::time::sleep(POLL).await;
        }
    }

    /// Answers a question and removes it.
    pub fn answer(&self, ask: &Ask, payload: &Value) {
        let name = ask
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(ASK))
            .unwrap_or_default()
            .to_string();
        write_atomically(&self.dir.join(format!("{ANSWER}{name}")), payload);
        // Removed only now: a question that vanished before its answer existed
        // would be a question nobody ever answers.
        let _ = std::fs::remove_file(&ask.path);
    }

    /// Removes the channel. The shim reads that as the adapter being gone, and
    /// denies whatever it was waiting on rather than waiting out its patience.
    pub fn close(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }

    /// The oldest question nobody has answered yet.
    fn pending(&self) -> Option<Ask> {
        let mut asks: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(ASK) && !name.ends_with(PARTIAL))
            })
            .collect();
        asks.sort();
        asks.into_iter().find_map(|path| {
            let asked: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
            Some(Ask {
                tool: asked
                    .get("tool")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tool_use_id: asked
                    .get("toolUseId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .map(str::to_string),
                input: asked.get("input").cloned().unwrap_or(Value::Null),
                path,
            })
        })
    }
}

/// The shim's side: asks the adapter and waits for what it decides.
///
/// Returns `None` when no decision could be obtained — the adapter is gone, the
/// wait ran out, the channel could not be written. Every one of those is a
/// denial at the caller, and none of them is a maybe.
#[must_use]
pub fn ask(dir: &Path, tool: &str, tool_use_id: Option<&str>, input: &Value) -> Option<Value> {
    let id = format!("{}-{}", std::process::id(), now_nanos());
    let question = serde_json::json!({
        "tool": tool,
        "toolUseId": tool_use_id,
        "input": input,
    });
    write_atomically(&dir.join(format!("{ASK}{id}.json")), &question);

    let answer = dir.join(format!("{ANSWER}{id}.json"));
    let deadline = Instant::now() + SHIM_PATIENCE;
    while Instant::now() < deadline {
        if let Ok(said) = std::fs::read_to_string(&answer) {
            let _ = std::fs::remove_file(&answer);
            return serde_json::from_str(&said).ok();
        }
        if !dir.exists() {
            // The session that governs this call is over. Nothing here may
            // decide on its behalf.
            return None;
        }
        std::thread::sleep(POLL);
    }
    None
}

/// Writes a file whole or not at all: written beside itself, then renamed.
fn write_atomically(path: &Path, value: &Value) {
    let partial = path.with_extension(format!(
        "{}{PARTIAL}",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    if std::fs::write(&partial, value.to_string()).is_ok() {
        let _ = std::fs::rename(&partial, path);
    }
}

/// A monotonic-enough tag so two questions from one process never collide.
fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default()
}

/// Makes the directory the user's own, where the platform says so with modes.
#[cfg(unix)]
fn harden(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// On Windows the per-user temporary directory is already the user's own, and
/// so is everything created inside it: the inherited ACL grants the owner and
/// the system, and nobody else. There is nothing to tighten that is not already
/// tight.
#[cfg(windows)]
fn harden(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "meltemi-rendezvous-test-{}-{tag}-{}",
            std::process::id(),
            now_nanos()
        ))
    }

    #[tokio::test]
    async fn a_question_reaches_the_adapter_and_the_answer_reaches_the_shim() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The whole channel, in the direction it actually runs: the shim (here,
        // a blocking thread, as it is a process of its own in life) asks, the
        // adapter's loop picks the question up, and what the adapter decided is
        // what the shim hands back to the CLI.
        let dir = scratch("round-trip");
        let rendezvous = Rendezvous::open(dir.clone()).expect("the channel opens");

        let asking = std::thread::spawn({
            let dir = dir.clone();
            move || {
                ask(
                    &dir,
                    "Write",
                    Some("toolu_1"),
                    &json!({"file_path": "NOTES.md"}),
                )
            }
        });

        let question = rendezvous.next_ask().await;
        assert_eq!(question.tool, "Write");
        assert_eq!(question.tool_use_id.as_deref(), Some("toolu_1"));
        assert_eq!(question.input["file_path"], "NOTES.md");
        rendezvous.answer(&question, &json!({"behavior": "allow"}));

        let answered = asking.join().expect("the shim finished");
        assert_eq!(answered.expect("a decision came back")["behavior"], "allow");
        rendezvous.close();
        assert!(!dir.exists(), "the channel leaves nothing behind");
    }

    #[test]
    fn a_shim_whose_adapter_is_gone_gets_no_decision_at_all() {
        // Scenario: Permiso decidido por el proxy vigente
        //
        // The one failure mode this channel must have: no answer, never a
        // guess. The caller turns that into a denial, which is the only thing
        // it may be turned into.
        let dir = scratch("gone");
        assert!(
            ask(&dir, "Bash", None, &json!({})).is_none(),
            "a channel that does not exist decides nothing"
        );
    }

    #[tokio::test]
    async fn a_question_stays_until_it_is_answered() {
        // The adapter's side is a branch of the turn loop and can lose the race
        // to another branch at any moment. A question that were removed when it
        // was read would be a question nobody ever answers.
        let dir = scratch("kept");
        let rendezvous = Rendezvous::open(dir.clone()).expect("the channel opens");
        write_atomically(
            &dir.join(format!("{ASK}1.json")),
            &json!({"tool": "Read", "input": {}}),
        );

        let first = rendezvous.next_ask().await;
        let again = rendezvous.next_ask().await;
        assert_eq!(first, again, "reading a question does not consume it");

        rendezvous.answer(&first, &json!({"behavior": "deny", "message": "no"}));
        assert!(
            !first.path.exists(),
            "answering it is what consumes it: {}",
            first.path.display()
        );
        rendezvous.close();
    }
}
