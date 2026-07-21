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
    default_endpoint()
}

#[cfg(unix)]
fn default_endpoint() -> String {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(data_dir);
    base.join("meltemi")
        .join("meltemid.sock")
        .to_string_lossy()
        .into_owned()
}

#[cfg(windows)]
fn default_endpoint() -> String {
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
