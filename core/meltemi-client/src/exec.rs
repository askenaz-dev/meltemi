// SPDX-License-Identifier: Apache-2.0

//! Speaking the protocol through a process the user named
//! (via-ssh-del-cliente).
//!
//! `meltemi bridge` already puts a daemon's local endpoint on its own stdio —
//! including the Windows named pipe that no SSH forwarding reaches — so
//! `ssh pc meltemi bridge` is a complete JSON-RPC channel. What was missing was
//! a client that could speak through a process instead of through a socket.
//! This module is that, and nothing more: the crossing is the user's own `ssh`,
//! and Meltemi neither sees nor carries any key material (§2).

use std::io;

/// The prefix that marks an endpoint as a command to run rather than a socket
/// to dial.
///
/// The form lives in the endpoint because the endpoint is already the single
/// place all three surfaces choose who to talk to, and it already travels
/// through the environment, the user's configuration and the arguments. A
/// parallel option would have made a second place to decide the same thing,
/// with the inevitable question of which one wins (design D1).
pub const EXEC_PREFIX: &str = "exec:";

/// Whether this endpoint names a command.
#[must_use]
pub fn is_exec(endpoint: &str) -> bool {
    endpoint.starts_with(EXEC_PREFIX)
}

/// Splits a command line into program and arguments.
///
/// Whitespace separates, and double quotes group — the one rule anybody writing
/// a path with a space needs. Deliberately NOT a shell: no expansion, no
/// globbing, no pipelines, and no second interpreter between what the user
/// wrote and what runs (design D2). A command that needs a pipeline names a
/// script, which is what a shell would have done with less control.
///
/// # Errors
///
/// An empty command names nothing to run, and an unclosed quote is a command
/// whose author meant something the splitter cannot guess. Both refuse rather
/// than run half of what was written.
pub fn split(command: &str) -> Result<(String, Vec<String>), String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;

    for ch in command.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                // An empty pair of quotes is a real, empty argument: `-o ""`
                // must not silently vanish.
                started = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if started {
                    parts.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if in_quotes {
        return Err(format!(
            "the command `{command}` has an unclosed quote; close it, or name a script \
             instead of writing the whole line here"
        ));
    }
    if started {
        parts.push(current);
    }

    let mut parts = parts.into_iter();
    let program = parts
        .next()
        .filter(|program| !program.is_empty())
        .ok_or_else(|| {
            "the endpoint names no command to run; write it as `exec:<program> <arguments>`, \
             for example `exec:ssh my-pc meltemi bridge`"
                .to_string()
        })?;
    Ok((program, parts.collect()))
}

/// The command an `exec:` endpoint names, split and ready to launch.
///
/// # Errors
///
/// Returns the splitter's refusal as an invalid-input error, so a caller that
/// only handles `io::Error` still gets the diagnosis.
pub fn command_of(endpoint: &str) -> io::Result<(String, Vec<String>)> {
    let command = endpoint.strip_prefix(EXEC_PREFIX).unwrap_or(endpoint);
    split(command).map_err(|reason| io::Error::new(io::ErrorKind::InvalidInput, reason))
}

/// How this endpoint is worth showing in a surface's chrome: the program and
/// its first argument, which for the case this exists for is `ssh <host>`.
///
/// A surface that said nothing about talking to another machine would make an
/// action on the wrong one look like an action on your own (design D8).
#[must_use]
pub fn shown(endpoint: &str) -> Option<String> {
    if !is_exec(endpoint) {
        return None;
    }
    let (program, args) = command_of(endpoint).ok()?;
    // Just the program's own name, not the path somebody may have spelled out.
    let program = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&program)
        .trim_end_matches(".exe")
        .to_string();
    Some(match args.first() {
        Some(first) => format!("{program} {first}"),
        None => program,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: El comando se lanza sin intérprete intermedio
    #[test]
    fn a_command_splits_into_a_program_and_its_arguments() {
        let (program, args) = split("ssh my-pc meltemi bridge").expect("a plain command");
        assert_eq!(program, "ssh");
        assert_eq!(args, ["my-pc", "meltemi", "bridge"]);
    }

    #[test]
    fn double_quotes_group_an_argument_that_has_spaces() {
        let (program, args) = split(r#""C:/Program Files/OpenSSH/ssh.exe" my-pc "meltemi bridge""#)
            .expect("quotes group");
        assert_eq!(program, "C:/Program Files/OpenSSH/ssh.exe");
        assert_eq!(args, ["my-pc", "meltemi bridge"]);
    }

    #[test]
    fn an_empty_pair_of_quotes_is_a_real_empty_argument() {
        // `-o ""` must not vanish: an argument the user wrote is an argument
        // the program receives, even when it is empty.
        let (_, args) = split(r#"ssh -o "" host"#).expect("an empty argument");
        assert_eq!(args, ["-o", "", "host"]);
    }

    #[test]
    fn an_unclosed_quote_refuses_instead_of_guessing() {
        let refusal = split(r#"ssh "my pc meltemi bridge"#).expect_err("unclosed");
        assert!(refusal.contains("unclosed quote"), "{refusal}");
        assert!(refusal.contains("name a script"), "the remedy: {refusal}");
    }

    #[test]
    fn an_empty_command_names_nothing_and_says_so() {
        for empty in ["", "   ", "\t"] {
            let refusal = split(empty).expect_err("nothing to run");
            assert!(refusal.contains("names no command"), "{refusal}");
            assert!(
                refusal.contains("exec:ssh my-pc meltemi bridge"),
                "the shape is shown: {refusal}"
            );
        }
    }

    #[test]
    fn the_prefix_is_what_tells_a_command_from_a_socket() {
        assert!(is_exec("exec:ssh pc meltemi bridge"));
        assert!(!is_exec(r"\\.\pipe\meltemid-guill"));
        assert!(!is_exec("/tmp/meltemid.sock"));

        let (program, args) = command_of("exec:ssh pc meltemi bridge").expect("split");
        assert_eq!(program, "ssh");
        assert_eq!(args, ["pc", "meltemi", "bridge"]);
    }

    #[test]
    fn the_chrome_shows_the_program_and_the_host_and_nothing_of_a_socket() {
        assert_eq!(
            shown("exec:ssh my-pc meltemi bridge").as_deref(),
            Some("ssh my-pc")
        );
        // A spelled-out path shows its name, not the whole path.
        assert_eq!(
            shown(r#"exec:"C:/Program Files/OpenSSH/ssh.exe" my-pc meltemi bridge"#).as_deref(),
            Some("ssh my-pc")
        );
        assert_eq!(shown("exec:local-helper").as_deref(), Some("local-helper"));
        // A socket has no via to show.
        assert_eq!(shown(r"\\.\pipe\meltemid-guill"), None);
    }
}
