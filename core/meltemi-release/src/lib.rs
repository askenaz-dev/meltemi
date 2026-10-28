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

/// The section of a changelog for `version`: what follows its `## ` heading up
/// to the next `## ` heading, trimmed (release-con-el-push D4).
///
/// The heading names the version first, with or without a leading `v`, and
/// may carry anything after it, such as a date: `## 0.2.0`, `## v0.2.0`,
/// `## 0.2.0 (2026-10-23)`. `None` when no heading names the version or its
/// section is empty, so the pipeline knows to generate notes instead.
#[must_use]
pub fn changelog_section(changelog: &str, version: Version) -> Option<String> {
    let wanted = version.to_string();
    let mut inside = false;
    let mut body: Vec<&str> = Vec::new();
    for line in changelog.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if inside {
                break;
            }
            let first = heading.split_whitespace().next().unwrap_or("");
            inside = first.strip_prefix('v').unwrap_or(first) == wanted;
            continue;
        }
        if inside {
            body.push(line);
        }
    }
    let text = body.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// One change in the archive, as the changelog lists it (antes-de-anunciar D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archived {
    /// The date its folder carries, `YYYY-MM-DD`.
    pub date: String,
    /// The change's name.
    pub name: String,
    /// Its folder under `.meltemi/changes/archive/`.
    pub folder: String,
    /// The line its proposal opens with: what it changed.
    pub summary: String,
}

/// The changes of `<root>/.meltemi/changes/archive/`, oldest first. A folder
/// whose name does not start with a date is not an archived change and is
/// skipped.
///
/// # Errors
///
/// The archive cannot be read.
pub fn archived_changes(root: &Path) -> Result<Vec<Archived>, String> {
    let dir = root.join(".meltemi").join("changes").join("archive");
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        if !entry.path().is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let Some((date, name)) = dated(&folder) else {
            continue;
        };
        let proposal =
            std::fs::read_to_string(entry.path().join("proposal.md")).unwrap_or_default();
        found.push(Archived {
            date: date.to_string(),
            name: name.to_string(),
            folder: folder.clone(),
            summary: summary_of(&proposal),
        });
    }
    found.sort_by(|a, b| (&a.date, &a.name).cmp(&(&b.date, &b.name)));
    Ok(found)
}

/// Splits `YYYY-MM-DD-name` into its date and its name.
fn dated(folder: &str) -> Option<(&str, &str)> {
    let (date, name) = (folder.get(..10)?, folder.get(11..)?);
    is_date(date)
        .then_some((date, name))
        .filter(|_| folder.as_bytes().get(10) == Some(&b'-') && !name.is_empty())
}

/// Whether `text` is a date of the form `YYYY-MM-DD`.
#[must_use]
pub fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(at, byte)| at == 4 || at == 7 || byte.is_ascii_digit())
}

/// The line a proposal opens its changes with: the bold lead of the first
/// bullet of «What Changes», or that bullet's first sentence, or else the first
/// sentence of «Why». Empty when the proposal says none of them.
#[must_use]
pub fn summary_of(proposal: &str) -> String {
    // A heading on the very first line is a heading too.
    let proposal = format!("\n{proposal}");
    let section = |title: &str| -> Option<String> {
        let start = proposal.find(&format!("\n## {title}"))?;
        let body = &proposal[start + 1..];
        let body = &body[body.find('\n').map_or(body.len(), |at| at + 1)..];
        let end = body.find("\n## ").unwrap_or(body.len());
        Some(body[..end].to_string())
    };
    let first_sentence = |text: &str| -> String {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let cut = flat
            .char_indices()
            .find(|(at, c)| {
                matches!(c, '.' | '!' | '?') && flat[at + c.len_utf8()..].starts_with(' ')
            })
            .map_or(flat.len(), |(at, c)| at + c.len_utf8());
        flat[..cut].to_string()
    };
    let tidy = |text: &str| -> String {
        let text = text.trim().trim_end_matches([':', ' ']).trim();
        let mut out: String = text.chars().take(240).collect();
        if text.chars().count() > 240 {
            out.push('…');
        }
        out
    };
    if let Some(changes) = section("What Changes") {
        let mut bullet = String::new();
        for line in changes.lines() {
            let line = line.trim_end();
            if let Some(item) = line.strip_prefix("- ") {
                if !bullet.is_empty() {
                    break;
                }
                bullet.push_str(item);
            } else if !bullet.is_empty() {
                if line.trim().is_empty() {
                    break;
                }
                bullet.push(' ');
                bullet.push_str(line.trim());
            }
        }
        let bullet = bullet.trim();
        if let Some(rest) = bullet.strip_prefix("**")
            && let Some(end) = rest.find("**")
        {
            return tidy(&rest[..end]);
        }
        if !bullet.is_empty() {
            return tidy(&first_sentence(bullet));
        }
    }
    if let Some(why) = section("Why") {
        let paragraph = why.trim().split("\n\n").next().unwrap_or("").to_string();
        return tidy(&first_sentence(&paragraph));
    }
    String::new()
}

/// What a changelog says before its first section, when it has none yet.
const CHANGELOG_HEADER: &str = "# Changelog\n\nEvery release lists the changes it archives: a change enters the first\nrelease written after it was archived, and its entry links its proposal in\n`.meltemi/changes/archive/`. Each line is the one its proposal opens its\nchanges with, in Spanish, the language of the method (constitution §11).\nWritten by `cargo run -p meltemi-release -- changelog <version>`; the section\nof a version is the release's notes (`meltemi-release notes <version>`).\n";

/// The changelog with the section of `version` written (antes-de-anunciar D3).
///
/// The section lists every archived change that no other section lists yet —
/// with `until`, only those archived up to that date, which becomes the
/// section's date. A section for `version` already present is replaced, so
/// writing the same version twice gives the same changelog. Sections are kept
/// newest version first.
#[must_use]
pub fn write_changelog(
    changelog: &str,
    version: Version,
    until: Option<&str>,
    archived: &[Archived],
) -> String {
    let (header, sections) = changelog_sections(changelog);
    let mut sections: Vec<(Option<Version>, String)> = sections
        .into_iter()
        .filter(|(heading, _)| *heading != Some(version))
        .collect();
    let listed: String = sections.iter().map(|(_, text)| text.as_str()).collect();
    let entries: Vec<&Archived> = archived
        .iter()
        .filter(|change| !listed.contains(&entry_key(change)))
        .filter(|change| until.is_none_or(|until| change.date.as_str() <= until))
        .collect();

    let mut section = match until {
        Some(date) => format!("## {version} — {date}\n\n"),
        None => format!("## {version}\n\n"),
    };
    if entries.is_empty() {
        section.push_str("No change was archived for this release.\n");
    } else {
        for change in entries {
            section.push_str(&entry_key(change));
            section.push_str(&format!(
                "(.meltemi/changes/archive/{}/proposal.md)",
                change.folder
            ));
            if !change.summary.is_empty() {
                section.push_str(&format!(" — {}", change.summary));
            }
            section.push('\n');
        }
    }
    sections.push((Some(version), section));
    sections.sort_by_key(|section| std::cmp::Reverse(section.0));

    let header = if header.trim().is_empty() {
        CHANGELOG_HEADER.to_string()
    } else {
        header
    };
    let mut out = header.trim_end().to_string();
    out.push('\n');
    for (_, text) in sections {
        out.push('\n');
        out.push_str(text.trim_end());
        out.push('\n');
    }
    out
}

/// How an archived change is named in a changelog entry, and what tells that
/// it is already listed.
fn entry_key(change: &Archived) -> String {
    format!("- [`{}`]", change.name)
}

/// A changelog's text before its first `## ` heading, and each section with the
/// version its heading names, if it names one.
fn changelog_sections(changelog: &str) -> (String, Vec<(Option<Version>, String)>) {
    let mut header = String::new();
    let mut sections: Vec<(Option<Version>, String)> = Vec::new();
    for line in changelog.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let first = heading.split_whitespace().next().unwrap_or("");
            let version = Version::parse(first.strip_prefix('v').unwrap_or(first));
            sections.push((version, format!("{line}\n")));
        } else if let Some((_, text)) = sections.last_mut() {
            text.push_str(line);
            text.push('\n');
        } else {
            header.push_str(line);
            header.push('\n');
        }
    }
    (header, sections)
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
    fn the_notes_are_the_changelog_section_of_the_version() {
        // Scenario: Notas desde el changelog
        let changelog = "# Changelog\n\n## 0.2.0 (2026-10-23)\n\n### Added\n\n- The release leaves on the push.\n\n## 0.1.1\n\n- Older.\n";
        let v = |s| Version::parse(s).expect("valid");
        assert_eq!(
            changelog_section(changelog, v("0.2.0")).as_deref(),
            Some("### Added\n\n- The release leaves on the push.")
        );
        assert_eq!(
            changelog_section(changelog, v("0.1.1")).as_deref(),
            Some("- Older.")
        );
        assert_eq!(
            changelog_section("## v0.3.0\n\nWith a v.\n", v("0.3.0")).as_deref(),
            Some("With a v.")
        );
        assert_eq!(
            changelog_section(changelog, v("0.9.0")),
            None,
            "no heading, no notes"
        );
        assert_eq!(
            changelog_section("## 0.4.0\n\n## 0.3.0\n- x\n", v("0.4.0")),
            None,
            "an empty section is no notes"
        );
        assert_eq!(
            changelog_section("## 0.2.00\n- x\n", v("0.2.0")),
            None,
            "the heading names this version, not one that looks like it"
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
