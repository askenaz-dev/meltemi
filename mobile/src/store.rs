// SPDX-License-Identifier: Apache-2.0

//! What the companion keeps between runs that is not a secret
//! (companero-que-conecta D6): the hosts it has met, by key fingerprint, and
//! the destination it last connected to. Plain files in the app's own data
//! directory — the same facts a user can read in `~/.ssh` on any machine.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ssh::HostKeys;

/// The hosts file: `host → fingerprint`, fixed on first use.
pub const HOSTS_FILE: &str = "known_hosts.json";
/// The destination file: `user@host[:port]`, as the user typed it.
pub const DESTINATION_FILE: &str = "destination";

/// Host fingerprints, fixed on first use and checked after.
///
/// Persisted on every change. A file that exists and cannot be read is an
/// error and not an empty list: an empty list would mean trusting whatever
/// key comes next, which is exactly the silence a changed host key must not
/// get.
pub struct KnownHosts {
    path: PathBuf,
    hosts: BTreeMap<String, String>,
}

impl KnownHosts {
    /// Reads the hosts file in `dir`, or starts empty when there is none.
    ///
    /// # Errors
    ///
    /// When the file exists and is not readable as the map it should be.
    pub fn open(dir: &Path) -> Result<Self, String> {
        let path = dir.join(HOSTS_FILE);
        let hosts = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| format!("{} is not a hosts file: {e}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(format!("could not read {}: {error}", path.display())),
        };
        Ok(Self { path, hosts })
    }

    /// Forgets `host`, which is what a reinstalled machine needs. Says whether
    /// there was anything to forget.
    ///
    /// # Errors
    ///
    /// When the change cannot be written down.
    pub fn forget(&mut self, host: &str) -> Result<bool, String> {
        let known = self.hosts.remove(host).is_some();
        if known {
            self.persist()?;
        }
        Ok(known)
    }

    /// The hosts on file, for a surface that lists them.
    #[must_use]
    pub fn hosts(&self) -> Vec<String> {
        self.hosts.keys().cloned().collect()
    }

    fn persist(&self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.hosts)
            .map_err(|e| format!("could not serialise the hosts: {e}"))?;
        write_whole(&self.path, text.as_bytes())
    }
}

impl HostKeys for KnownHosts {
    fn remembered(&self, host: &str) -> Option<String> {
        self.hosts.get(host).cloned()
    }

    fn remember(&mut self, host: &str, fingerprint: &str) {
        self.hosts.insert(host.to_string(), fingerprint.to_string());
        // Best effort, and the failure mode is the safe one: the fingerprint
        // is held for this run, so this connection is checked; a next run that
        // finds no file is a first use again, which asks nothing of the user
        // and accepts nothing it would not have accepted the first time.
        let _ = self.persist();
    }
}

/// The destination last connected to, as the user typed it.
pub struct Remembered {
    path: PathBuf,
}

impl Remembered {
    #[must_use]
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            path: dir.join(DESTINATION_FILE),
        }
    }

    /// The remembered destination, if any; whitespace is not a destination.
    #[must_use]
    pub fn read(&self) -> Option<String> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    /// Remembers `target` for the next run.
    ///
    /// # Errors
    ///
    /// When it cannot be written down.
    pub fn write(&self, target: &str) -> Result<(), String> {
        write_whole(&self.path, target.trim().as_bytes())
    }

    /// Forgets the destination. Forgetting what was never remembered is fine.
    ///
    /// # Errors
    ///
    /// When the file exists and cannot be removed.
    pub fn clear(&self) -> Result<(), String> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("could not forget {}: {error}", self.path.display())),
        }
    }
}

/// Writes `bytes` to `path` whole: a neighbour first, then a rename, so a
/// crash mid-write leaves the old file and never half of the new one.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let staged = path.with_extension("tmp");
    std::fs::write(&staged, bytes)
        .map_err(|e| format!("could not write {}: {e}", staged.display()))?;
    std::fs::rename(&staged, path).map_err(|e| format!("could not write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory per test, under the system's temp dir and this
    /// process's id, removed when the guard drops.
    struct Dir(PathBuf);

    impl Dir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "meltemi-companion-store-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a test directory");
            Self(path)
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Scenario: La clave del host cambia
    #[test]
    fn a_remembered_host_is_remembered_across_runs_and_forgotten_on_purpose() {
        let dir = Dir::new("hosts");
        let mut first = KnownHosts::open(&dir.0).expect("no file yet is fine");
        assert_eq!(first.remembered("mac-studio"), None);
        first.remember("mac-studio", "SHA256:abc");

        // A second run reads what the first wrote down.
        let mut second = KnownHosts::open(&dir.0).expect("the file is readable");
        assert_eq!(
            second.remembered("mac-studio").as_deref(),
            Some("SHA256:abc")
        );
        assert_eq!(second.hosts(), vec!["mac-studio".to_string()]);

        // Forgetting is explicit, and it sticks.
        assert!(second.forget("mac-studio").expect("written"));
        assert!(!second.forget("mac-studio").expect("nothing to forget"));
        let third = KnownHosts::open(&dir.0).expect("readable");
        assert_eq!(third.remembered("mac-studio"), None);
    }

    #[test]
    fn a_hosts_file_that_cannot_be_read_is_an_error_and_not_an_empty_list() {
        let dir = Dir::new("hosts-rubbish");
        std::fs::write(dir.0.join(HOSTS_FILE), b"not json at all").expect("written");
        let error = match KnownHosts::open(&dir.0) {
            Ok(_) => panic!("rubbish must not read as 'no hosts known'"),
            Err(error) => error,
        };
        assert!(error.contains("is not a hosts file"), "{error}");
    }

    // Scenario: El destino se escribe una vez
    #[test]
    fn the_destination_is_remembered_as_typed_and_cleared_on_purpose() {
        let dir = Dir::new("destination");
        let remembered = Remembered::in_dir(&dir.0);
        assert_eq!(remembered.read(), None);
        remembered
            .write("  ana@mac-studio:2222 \n")
            .expect("written");
        assert_eq!(remembered.read().as_deref(), Some("ana@mac-studio:2222"));
        // Whitespace is not a destination.
        remembered.write("   ").expect("written");
        assert_eq!(remembered.read(), None);
        remembered.write("ana@mac-studio").expect("written");
        remembered.clear().expect("cleared");
        remembered.clear().expect("clearing twice is fine");
        assert_eq!(remembered.read(), None);
    }

    #[test]
    fn a_write_never_leaves_half_a_file_behind() {
        let dir = Dir::new("whole");
        let remembered = Remembered::in_dir(&dir.0);
        remembered.write("ana@mac-studio").expect("written");
        let left: Vec<_> = std::fs::read_dir(&dir.0)
            .expect("listable")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            left,
            vec![DESTINATION_FILE.to_string()],
            "no staging file remains"
        );
    }
}
