// SPDX-License-Identifier: Apache-2.0

//! The release tooling of this repository (release-con-el-push D1, D2).
//!
//! A release is decided by code, not by somebody reading YAML: which files
//! carry the version and whether they agree, and later whether a commit opens
//! the release path and which notes it carries. Everything here is plain text
//! work over the files of a tree, so it behaves the same on the three
//! platforms the project supports and needs no shell.
//!
//! The version is written in five files, one of them twice. They are the
//! *carriers*, and [`CARRIERS`] is their complete list: a carrier added to the
//! tree and not here is a version that drifts silently, which is what the
//! repository test over [`agreed_version`] exists to catch.

use std::fmt;
use std::path::{Path, PathBuf};

/// A version of the form `x.y.z`, compared as SemVer without pre-releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Parses `x.y.z`: three decimal numbers without leading zeros.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let (major, minor, patch) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major: number(major)?,
            minor: number(minor)?,
            patch: number(patch)?,
        })
    }

    /// Parses a release tag, `v` followed by a version.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        tag.strip_prefix('v').and_then(Self::parse)
    }

    /// The release tag of this version.
    #[must_use]
    pub fn tag(self) -> String {
        format!("v{self}")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn number(text: &str) -> Option<u64> {
    let digits = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    let canonical = text == "0" || !text.starts_with('0');
    if digits && canonical {
        text.parse().ok()
    } else {
        None
    }
}

/// How a carrier's version line is found inside its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locator {
    /// `version = "…"` inside the `[workspace.package]` table of `Cargo.toml`.
    WorkspacePackage,
    /// The top-level `"version": "…"` of a JSON file indented by two spaces.
    JsonTopLevel,
    /// The `"version"` of the root package, `packages[""]`, in an npm lockfile.
    LockfileRootPackage,
}

/// One place where the product version is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carrier {
    /// Path from the repository root, with forward slashes.
    pub path: &'static str,
    pub locator: Locator,
}

impl Carrier {
    /// How the carrier is named in a message: its path, plus which entry when
    /// one file carries the version twice.
    #[must_use]
    pub fn label(self) -> String {
        match self.locator {
            Locator::WorkspacePackage => format!("{} [workspace.package]", self.path),
            Locator::JsonTopLevel => self.path.to_string(),
            Locator::LockfileRootPackage => format!("{} (packages[\"\"])", self.path),
        }
    }
}

/// Every carrier of the version, in the order a reader meets them.
pub const CARRIERS: [Carrier; 6] = [
    Carrier {
        path: "Cargo.toml",
        locator: Locator::WorkspacePackage,
    },
    Carrier {
        path: "desktop/tauri.conf.json",
        locator: Locator::JsonTopLevel,
    },
    Carrier {
        path: "mobile/tauri.conf.json",
        locator: Locator::JsonTopLevel,
    },
    Carrier {
        path: "desktop/ui/package.json",
        locator: Locator::JsonTopLevel,
    },
    Carrier {
        path: "desktop/ui/package-lock.json",
        locator: Locator::JsonTopLevel,
    },
    Carrier {
        path: "desktop/ui/package-lock.json",
        locator: Locator::LockfileRootPackage,
    },
];

/// The line that holds a carrier's version, and the version as written.
#[must_use]
pub fn locate(text: &str, locator: Locator) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let index = match locator {
        Locator::WorkspacePackage => {
            let table = lines
                .iter()
                .position(|line| line.trim() == "[workspace.package]")?;
            (table + 1..lines.len())
                .take_while(|&i| !lines[i].trim_start().starts_with('['))
                .find(|&i| lines[i].trim_start().starts_with("version = \""))?
        }
        Locator::JsonTopLevel => lines
            .iter()
            .position(|line| line.starts_with("  \"version\": \"") && !line.starts_with("   "))?,
        Locator::LockfileRootPackage => {
            let root = lines
                .iter()
                .position(|line| line.trim_end() == "    \"\": {")?;
            (root + 1..lines.len())
                .take_while(|&i| !lines[i].starts_with("    }"))
                .find(|&i| lines[i].starts_with("      \"version\": \""))?
        }
    };
    let line = lines[index];
    let start = line.find('"')?;
    let value = match locator {
        // `version = "x"`: the first quoted string is the value.
        Locator::WorkspacePackage => &line[start + 1..],
        // `"version": "x"`: skip the key, take the second quoted string.
        Locator::JsonTopLevel | Locator::LockfileRootPackage => {
            let after_key = line[start + 1..].find('"')? + start + 2;
            let open = line[after_key..].find('"')? + after_key;
            &line[open + 1..]
        }
    };
    let end = value.find('"')?;
    Some((index, value[..end].to_string()))
}

/// What one carrier says, or why it says nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub carrier: Carrier,
    /// The version as written, when the carrier's line was found.
    pub written: Option<String>,
}

/// Reads every carrier through `read`, which maps a carrier path to the
/// file's text, or to `None` when the file does not exist.
pub fn read_carriers(read: impl Fn(&str) -> Option<String>) -> Vec<Reading> {
    CARRIERS
        .iter()
        .map(|&carrier| Reading {
            carrier,
            written: read(carrier.path)
                .and_then(|text| locate(&text, carrier.locator).map(|(_, value)| value)),
        })
        .collect()
}

/// The version every carrier agrees on.
///
/// # Errors
///
/// When a carrier cannot be read, does not hold a version of the form
/// `x.y.z`, or disagrees with the first one: the message names each carrier
/// and what it says, so the fix is a read away.
pub fn agreed_version(read: impl Fn(&str) -> Option<String>) -> Result<Version, String> {
    let readings = read_carriers(read);
    let parsed: Vec<Option<Version>> = readings
        .iter()
        .map(|reading| reading.written.as_deref().and_then(Version::parse))
        .collect();
    if let Some(first) = parsed[0]
        && parsed.iter().all(|version| *version == Some(first))
    {
        return Ok(first);
    }
    let detail: Vec<String> = readings
        .iter()
        .map(|reading| match &reading.written {
            Some(written) => format!("{} says {written}", reading.carrier.label()),
            None => format!("{} holds no version line", reading.carrier.label()),
        })
        .collect();
    Err(format!(
        "the carriers of the version disagree or are unreadable: {}",
        detail.join("; ")
    ))
}

/// Reads a carrier from a working tree.
pub fn read_from_tree(root: &Path) -> impl Fn(&str) -> Option<String> + '_ {
    move |path| std::fs::read_to_string(root.join(path)).ok()
}

/// Writes `new` into every carrier of the tree at `root`.
///
/// The current version is the newest any carrier holds, so a tree whose
/// carriers drifted can be brought back together, but never moved backwards.
///
/// # Errors
///
/// When a carrier cannot be read or written, or when `new` is not greater
/// than the current version. Nothing is written unless every carrier was
/// located first.
pub fn set_version(root: &Path, new: Version) -> Result<Vec<PathBuf>, String> {
    let readings = read_carriers(read_from_tree(root));
    let mut current: Option<Version> = None;
    for reading in &readings {
        let written = reading
            .written
            .as_deref()
            .ok_or_else(|| format!("{} holds no version line", reading.carrier.label()))?;
        let version = Version::parse(written).ok_or_else(|| {
            format!(
                "{} says {written}, which is not x.y.z",
                reading.carrier.label()
            )
        })?;
        current = current.max(Some(version));
    }
    let current = current.expect("there is at least one carrier");
    if new <= current {
        return Err(format!(
            "{new} is not greater than the current version {current}: a version only moves forward"
        ));
    }

    // Every carrier located; now rewrite, one file at a time, each line once.
    let mut changed: Vec<PathBuf> = Vec::new();
    let mut paths: Vec<&str> = CARRIERS.iter().map(|carrier| carrier.path).collect();
    paths.dedup();
    for path in paths {
        let file = root.join(path);
        let text = std::fs::read_to_string(&file).map_err(|e| format!("{path}: {e}"))?;
        let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        for carrier in CARRIERS.iter().filter(|carrier| carrier.path == path) {
            let (index, old) = locate(&text, carrier.locator)
                .ok_or_else(|| format!("{} holds no version line", carrier.label()))?;
            lines[index] = lines[index].replacen(&format!("\"{old}\""), &format!("\"{new}\""), 1);
        }
        let next = lines.join("\n");
        if next != text {
            std::fs::write(&file, next).map_err(|e| format!("{path}: {e}"))?;
            changed.push(PathBuf::from(path));
        }
    }
    Ok(changed)
}

/// What the release gate decided about one commit (release-con-el-push D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Whether the commit opens the release path.
    pub release: bool,
    /// The version the commit's carriers agree on.
    pub version: Version,
    /// Why, in one sentence a person can act on.
    pub reason: String,
}

/// Runs `git` inside `repo`.
fn git(repo: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))
}

/// The text of `path` as it is in `commit`, or `None` when the commit has no
/// such file.
fn file_at(repo: &Path, commit: &str, path: &str) -> Option<String> {
    let output = git(repo, &["show", &format!("{commit}:{path}")]).ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Decides whether `commit` opens the release path.
///
/// The commit, not the working tree, is what is judged: its carriers are read
/// from the commit itself. In order, the commit is refused when its version
/// already has a tag, when it is not greater than the newest version tag, and
/// when it is not reachable from `main_ref`. Tags count whether or not their
/// release was published: a draft's tag is a version that was claimed.
///
/// # Errors
///
/// When the carriers of the commit disagree or cannot be read, or `git` cannot
/// answer. Those are not a "no": they fail the pipeline loudly, so that a red
/// run says what to fix instead of a green one that published nothing.
pub fn gate(repo: &Path, commit: &str, main_ref: &str) -> Result<Decision, String> {
    let version = agreed_version(|path| file_at(repo, commit, path))?;
    let no = |reason: String| {
        Ok(Decision {
            release: false,
            version,
            reason,
        })
    };

    let listed = git(repo, &["tag", "--list", "v*"])?;
    if !listed.status.success() {
        return Err(format!(
            "git tag --list failed: {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    let tags: Vec<Version> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| Version::from_tag(line.trim()))
        .collect();
    if tags.contains(&version) {
        return no(format!(
            "{} already exists: this version was released, or claimed by a draft",
            version.tag()
        ));
    }
    let newest = tags.iter().max().copied();
    if let Some(newest) = newest
        && version < newest
    {
        return no(format!(
            "{version} is not greater than the newest version tag {}: a release only moves forward",
            newest.tag()
        ));
    }

    let reachable = git(repo, &["merge-base", "--is-ancestor", commit, main_ref])?;
    match reachable.status.code() {
        Some(0) => {}
        Some(1) => return no(format!("{commit} is not reachable from {main_ref}")),
        _ => {
            return Err(format!(
                "git merge-base --is-ancestor {commit} {main_ref} failed: {}",
                String::from_utf8_lossy(&reachable.stderr).trim()
            ));
        }
    }

    let over = newest.map_or_else(
        || "it is the first version tag".to_string(),
        |newest| format!("it is newer than {}", newest.tag()),
    );
    Ok(Decision {
        release: true,
        version,
        reason: format!(
            "{} has no tag, {over}, and the commit is on {main_ref}",
            version.tag()
        ),
    })
}

/// One `[[package]]` entry of a `Cargo.lock`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LockEntry {
    name: String,
    version: String,
    source: Option<String>,
}

fn lock_entries(lock: &str) -> Vec<LockEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LockEntry> = None;
    let value = |line: &str, key: &str| {
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(" = \""))
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_string)
    };
    for line in lock.lines() {
        let line = line.trim_end();
        if line == "[[package]]" {
            entries.extend(current.take());
            current = Some(LockEntry {
                name: String::new(),
                version: String::new(),
                source: None,
            });
        } else if let Some(entry) = current.as_mut() {
            if let Some(name) = value(line, "name") {
                entry.name = name;
            } else if let Some(version) = value(line, "version") {
                entry.version = version;
            } else if let Some(source) = value(line, "source") {
                entry.source = Some(source);
            }
        }
    }
    entries.extend(current);
    entries
}

/// Checks that a `Cargo.lock` refreshed after a version change moved only the
/// workspace's own crates: the entries with no `source`. A third-party crate
/// that changed would be a dependency update riding inside a version commit.
///
/// # Errors
///
/// Names every third-party entry that appeared, disappeared or moved.
pub fn lock_moved_only_the_workspace(before: &str, after: &str) -> Result<(), String> {
    let third_party = |lock: &str| -> Vec<LockEntry> {
        let mut entries: Vec<LockEntry> = lock_entries(lock)
            .into_iter()
            .filter(|entry| entry.source.is_some())
            .collect();
        entries.sort();
        entries
    };
    let (old, new) = (third_party(before), third_party(after));
    if old == new {
        return Ok(());
    }
    let gone = old.iter().filter(|entry| !new.contains(entry));
    let came = new.iter().filter(|entry| !old.contains(entry));
    let detail: Vec<String> = gone
        .map(|entry| format!("-{} {}", entry.name, entry.version))
        .chain(came.map(|entry| format!("+{} {}", entry.name, entry.version)))
        .collect();
    Err(format!(
        "Cargo.lock moved third-party crates, not only the workspace: {}",
        detail.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_three_canonical_numbers() {
        assert_eq!(
            Version::parse("0.2.0"),
            Some(Version {
                major: 0,
                minor: 2,
                patch: 0
            })
        );
        for bad in ["0.2", "0.2.0.1", "v0.2.0", "0.02.0", "0.2.x", "", "0..1"] {
            assert_eq!(Version::parse(bad), None, "{bad} is not x.y.z");
        }
        assert_eq!(
            Version::from_tag("v1.10.3")
                .map(|v| v.to_string())
                .as_deref(),
            Some("1.10.3")
        );
        assert_eq!(Version::from_tag("1.10.3"), None, "a tag starts with v");
    }

    #[test]
    fn versions_order_by_number_not_by_text() {
        let v = |s| Version::parse(s).expect("valid");
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("0.2.0") > v("0.1.1"));
    }

    #[test]
    fn each_locator_finds_its_line_and_only_its_line() {
        let cargo = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.1.1\"\nedition = \"2024\"\n\n[workspace.dependencies]\nversion = \"9.9.9\"\n";
        assert_eq!(
            locate(cargo, Locator::WorkspacePackage),
            Some((4, "0.1.1".into()))
        );

        let json = "{\n  \"$schema\": \"x\",\n  \"productName\": \"Meltemi\",\n  \"version\": \"0.1.1\",\n  \"nested\": {\n    \"version\": \"9.9.9\"\n  }\n}\n";
        assert_eq!(
            locate(json, Locator::JsonTopLevel),
            Some((3, "0.1.1".into()))
        );

        let lock = "{\n  \"name\": \"ui\",\n  \"version\": \"0.1.1\",\n  \"packages\": {\n    \"\": {\n      \"name\": \"ui\",\n      \"version\": \"0.1.1\"\n    },\n    \"node_modules/x\": {\n      \"version\": \"9.9.9\"\n    }\n  }\n}\n";
        assert_eq!(
            locate(lock, Locator::JsonTopLevel),
            Some((2, "0.1.1".into()))
        );
        assert_eq!(
            locate(lock, Locator::LockfileRootPackage),
            Some((6, "0.1.1".into()))
        );
    }

    #[test]
    fn a_lockfile_that_moved_a_third_party_crate_is_refused() {
        let before = "[[package]]\nname = \"meltemi\"\nversion = \"0.1.1\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.228\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
        let workspace_only = before.replace("version = \"0.1.1\"", "version = \"0.2.0\"");
        assert_eq!(
            lock_moved_only_the_workspace(before, &workspace_only),
            Ok(())
        );

        let dependency_too = workspace_only.replace("1.0.228", "1.0.229");
        let refused = lock_moved_only_the_workspace(before, &dependency_too).expect_err("refused");
        assert!(
            refused.contains("serde 1.0.228") && refused.contains("serde 1.0.229"),
            "{refused}"
        );
    }
}
