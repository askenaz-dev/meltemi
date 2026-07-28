// SPDX-License-Identifier: Apache-2.0

//! The permission side of the scripted headless-session wire
//! (adaptadores-propios-acp task 3.6).
//!
//! The adapter hands the piloted CLI two things it must actually use before it
//! runs a tool: a hook to run first, and an MCP server to ask when nothing else
//! decided. A fixture that ignored both would let a broken adapter pass every
//! test it has — the configuration would be written, and nothing would ever
//! read it.
//!
//! So this module does what the CLI does with them: it runs the hook command
//! through a shell with the documented event on its input, and it speaks MCP
//! over stdio to the configured server. Both are launched exactly as the real
//! CLI launches them, which is what makes this the place where the adapter's
//! own quoting, argument order and protocol get tested rather than assumed.
//!
//! The order is the provider's documented one, and this fixture keeps it: the
//! hook decides first and its answer is final; only a call it did not decide
//! reaches the prompt tool. Two script directives, one per channel, so a test
//! can exercise either half on its own.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// What a permission channel answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Whether the call may proceed.
    pub allowed: bool,
    /// Why, in the words the channel used.
    pub reason: String,
}

impl Decision {
    fn denied(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
        }
    }
}

/// The permission configuration the adapter handed this wire on the command
/// line.
#[derive(Debug, Clone, Default)]
pub struct Handed {
    /// The MCP configuration file.
    pub mcp_config: Option<PathBuf>,
    /// The settings file, which is where the hook lives.
    pub settings: Option<PathBuf>,
    /// The tool the CLI is told to ask, as `mcp__<server>__<tool>`.
    pub prompt_tool: Option<String>,
}

impl Handed {
    /// Reads what the adapter passed.
    #[must_use]
    pub fn from_args(args: &[String]) -> Self {
        Self {
            mcp_config: valued(args, "--mcp-config").map(PathBuf::from),
            settings: valued(args, "--settings").map(PathBuf::from),
            prompt_tool: valued(args, "--permission-prompt-tool"),
        }
    }

    /// Runs the hook the adapter installed, as the CLI runs it: through a
    /// shell, with the documented event on its input.
    #[must_use]
    pub fn ask_hook(&self, tool: &str, tool_use_id: &str, input: &Value) -> Decision {
        let Some(settings) = &self.settings else {
            return Decision::denied("no settings were handed to this wire");
        };
        let Some(command) = hook_command(settings) else {
            return Decision::denied("the settings carry no PreToolUse hook");
        };
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "mock-claude-session",
            "cwd": ".",
            "tool_name": tool,
            "tool_use_id": tool_use_id,
            "tool_input": input,
        });
        match run_through_shell(&command, &event.to_string()) {
            Ok(said) => read_hook_answer(&said),
            Err(error) => Decision::denied(format!("the hook could not be run: {error}")),
        }
    }

    /// Calls the permission tool the adapter registered, speaking MCP over the
    /// server's stdio exactly as the CLI does.
    #[must_use]
    pub fn ask_prompt_tool(&self, tool: &str, tool_use_id: &str, input: &Value) -> Decision {
        match self.call_prompt_tool(tool, tool_use_id, input) {
            Ok(decision) => decision,
            Err(error) => Decision::denied(error),
        }
    }

    fn call_prompt_tool(
        &self,
        tool: &str,
        tool_use_id: &str,
        input: &Value,
    ) -> Result<Decision, String> {
        let config = self
            .mcp_config
            .as_ref()
            .ok_or("no MCP configuration was handed to this wire")?;
        let reference = self
            .prompt_tool
            .as_ref()
            .ok_or("no permission prompt tool was named")?;
        let (server_name, tool_name) = split_reference(reference)
            .ok_or_else(|| format!("`{reference}` is not an `mcp__server__tool` reference"))?;

        let document: Value = serde_json::from_str(
            &std::fs::read_to_string(config)
                .map_err(|error| format!("the MCP configuration cannot be read: {error}"))?,
        )
        .map_err(|error| format!("the MCP configuration is not JSON: {error}"))?;
        let server = &document["mcpServers"][&server_name];
        let command = server["command"]
            .as_str()
            .ok_or_else(|| format!("no server `{server_name}` in the configuration"))?;
        let args: Vec<String> = server["args"]
            .as_array()
            .map(|args| {
                args.iter()
                    .filter_map(|arg| arg.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let mut child = Command::new(command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("the permission server could not be launched: {error}"))?;
        let mut writing = child.stdin.take().ok_or("no input to the server")?;
        let mut reading = BufReader::new(child.stdout.take().ok_or("no output from the server")?);

        // The handshake the protocol requires, then the call itself.
        say(
            &mut writing,
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                           "clientInfo": {"name": "mock-claude-wire", "version": "0"}},
            }),
        )?;
        let _ = hear(&mut reading)?;
        say(
            &mut writing,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )?;
        say(
            &mut writing,
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": {"name": tool_name, "arguments": {
                    "tool_name": tool, "tool_use_id": tool_use_id, "input": input,
                }},
            }),
        )?;
        let answered = hear(&mut reading)?;

        drop(writing);
        let _ = child.wait();

        let said = answered["result"]["content"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("the tool answered nothing readable: {answered}"))?;
        let decision: Value = serde_json::from_str(said)
            .map_err(|error| format!("the tool's answer is not JSON ({error}): {said}"))?;
        Ok(read_prompt_tool_answer(&decision))
    }
}

/// The events the CLI emits for a tool call it decided about: the call itself,
/// and its result — which is the denial when it was denied, because that is
/// what the model would see.
#[must_use]
pub fn tool_events(
    tool: &str,
    tool_use_id: &str,
    input: &Value,
    decision: &Decision,
) -> Vec<String> {
    let call = json!({
        "type": "assistant",
        "session_id": "mock-claude-session",
        "message": {"role": "assistant", "content": [
            {"type": "tool_use", "id": tool_use_id, "name": tool, "input": input}
        ]},
    });
    let result = json!({
        "type": "user",
        "session_id": "mock-claude-session",
        "message": {"role": "user", "content": [{
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": if decision.allowed { "ok".to_string() } else { decision.reason.clone() },
            "is_error": !decision.allowed,
        }]},
    });
    vec![call.to_string(), result.to_string()]
}

/// The hook's answer, in the shape the provider documents.
fn read_hook_answer(said: &str) -> Decision {
    let Some(answer) = said
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .next_back()
    else {
        return Decision::denied(format!("the hook said nothing readable: {said}"));
    };
    let output = &answer["hookSpecificOutput"];
    let reason = output["permissionDecisionReason"]
        .as_str()
        .unwrap_or("the hook gave no reason")
        .to_string();
    Decision {
        allowed: output["permissionDecision"] == "allow",
        reason,
    }
}

/// The prompt tool's answer, in the shape the provider documents.
fn read_prompt_tool_answer(decision: &Value) -> Decision {
    Decision {
        allowed: decision["behavior"] == "allow",
        reason: decision["message"]
            .as_str()
            .unwrap_or("the tool gave no reason")
            .to_string(),
    }
}

/// The `PreToolUse` command a settings file installs.
fn hook_command(settings: &Path) -> Option<String> {
    let document: Value = serde_json::from_str(&std::fs::read_to_string(settings).ok()?).ok()?;
    document["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .map(str::to_string)
}

/// Runs a command line the way a CLI runs a hook: through the platform's shell,
/// with the event on its input.
///
/// On Windows the command line is handed to `cmd` **raw**, with the switches a
/// language runtime uses for the same job (`/d /s /c "<command>"`), because
/// Rust's own argument escaping is not the escaping `cmd` reads and a quoted
/// path would arrive mangled. Getting this right is the point: it is what makes
/// the adapter's own quoting of a path with a space in it a thing this fixture
/// actually tests rather than assumes.
fn run_through_shell(command: &str, input: &str) -> Result<String, String> {
    let mut shell = Command::new(if cfg!(windows) { "cmd" } else { "sh" });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        shell.raw_arg(format!("/d /s /c \"{command}\""));
    }
    #[cfg(not(windows))]
    {
        shell.arg("-c").arg(command);
    }
    let mut child = shell
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Captured rather than inherited so that a hook which fails says why
        // *here*, in the decision this fixture returns, instead of scrolling
        // past in somebody else's log: a fixture failure has to be loud.
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("no input to the hook")?
        .write_all(input.as_bytes())
        .map_err(|error| error.to_string())?;
    let done = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    let said = String::from_utf8_lossy(&done.stdout).into_owned();
    if said.trim().is_empty() {
        return Err(format!(
            "the hook `{command}` printed nothing (exit {:?}), and said this on stderr: {}",
            done.status.code(),
            String::from_utf8_lossy(&done.stderr).trim()
        ));
    }
    Ok(said)
}

/// `mcp__server__tool` split into its two names.
fn split_reference(reference: &str) -> Option<(String, String)> {
    let rest = reference.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    Some((server.to_string(), tool.to_string()))
}

fn valued(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

fn say(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    writeln!(writer, "{message}").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn hear(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err("the permission server closed its output".to_string());
        }
        if line.trim().is_empty() {
            continue;
        }
        return serde_json::from_str(line.trim())
            .map_err(|error| format!("the server said something that is not JSON: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_adapter_handed_this_wire_is_read_off_the_command_line() {
        let args: Vec<String> = [
            "mock-claude-wire",
            "-p",
            "--mcp-config",
            "/tmp/mcp.json",
            "--permission-prompt-tool",
            "mcp__meltemi_permissions__approve",
            "--settings",
            "/tmp/settings.json",
        ]
        .iter()
        .map(|arg| (*arg).to_string())
        .collect();
        let handed = Handed::from_args(&args);
        assert_eq!(handed.mcp_config, Some(PathBuf::from("/tmp/mcp.json")));
        assert_eq!(handed.settings, Some(PathBuf::from("/tmp/settings.json")));
        assert_eq!(
            split_reference(&handed.prompt_tool.expect("a tool was named")),
            Some(("meltemi_permissions".into(), "approve".into()))
        );
    }

    #[test]
    fn a_channel_that_answers_nothing_is_read_as_a_denial() {
        // The fixture must never turn silence into permission either: a wire
        // that did would let a broken adapter pass.
        assert!(!read_hook_answer("").allowed);
        assert!(!read_hook_answer("not json").allowed);
        assert!(!read_prompt_tool_answer(&json!({})).allowed);
        assert!(
            read_hook_answer(
                r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}"#
            )
            .allowed
        );
        assert!(read_prompt_tool_answer(&json!({"behavior": "allow"})).allowed);
    }

    #[test]
    fn a_denied_call_is_answered_with_the_denial_the_model_would_see() {
        let events = tool_events(
            "Write",
            "toolu_1",
            &json!({"file_path": "NOTES.md"}),
            &Decision::denied("no"),
        );
        let result: Value = serde_json::from_str(&events[1]).expect("the result is JSON");
        assert_eq!(result["message"]["content"][0]["is_error"], true);
        assert_eq!(result["message"]["content"][0]["content"], "no");
    }
}
