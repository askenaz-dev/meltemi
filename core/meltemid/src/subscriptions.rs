// SPDX-License-Identifier: Apache-2.0

//! The daemon-owned subscription store (vincular-suscripciones design D2/D4).
//!
//! Linked subscriptions are ordinary launch profiles persisted in
//! `<config_dir>/subscriptions.toml` — a machine-managed file of
//! `[[fleet.profile]]` blocks that the existing config parser reads verbatim.
//! It is loaded BEFORE the user's `config.toml`, so with the merge-by-name
//! rule already in force, anything written by hand wins over a homonymous
//! link: the explicit user always outranks the machine.
//!
//! The file is rewritten whole on every change. That is safe because it is
//! OURS: the header says so, and hand-written profiles belong in
//! `config.toml`, where nothing here ever writes.

use std::path::{Path, PathBuf};

use crate::config::FleetProfile;

/// The managed file, beside the user's `config.toml`.
#[must_use]
pub fn store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("subscriptions.toml")
}

/// The header that declares the file machine-managed. Rewritten every time,
/// so a hand edit here is a note left on a whiteboard about to be wiped —
/// the header says where hand edits belong instead.
const HEADER: &str = "\
# Managed by Meltemi (subscription/link). Do not edit: this file is rewritten\n\
# whole on every link/unlink. Hand-written profiles belong in config.toml,\n\
# where they win over links by name.\n";

/// Whether a link name is a safe path component: kebab-case, no separators,
/// no dots, no case surprises on Windows. The name becomes a directory.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The linked profiles, in file order. A missing file is an empty store; an
/// unreadable one is reported by the caller-facing operations, not here.
#[must_use]
pub fn load(config_dir: &Path) -> Vec<FleetProfile> {
    let Ok(contents) = std::fs::read_to_string(store_path(config_dir)) else {
        return Vec::new();
    };
    parse(&contents)
}

/// Parses the store's `[[fleet.profile]]` blocks (same shape the config
/// parser reads; kept lean here to avoid exposing config internals).
fn parse(contents: &str) -> Vec<FleetProfile> {
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        fleet: RawFleet,
    }
    #[derive(Default, serde::Deserialize)]
    struct RawFleet {
        #[serde(default)]
        profile: Vec<RawProfile>,
    }
    #[derive(serde::Deserialize)]
    struct RawProfile {
        name: String,
        agent: String,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    }
    match toml::from_str::<Raw>(contents) {
        Ok(raw) => raw
            .fleet
            .profile
            .into_iter()
            .map(|p| FleetProfile {
                model: None,
                effort: None,
                name: p.name,
                agent: p.agent,
                env: p.env.into_iter().collect(),
            })
            .collect(),
        Err(e) => {
            tracing::warn!(error = %e, "ignoring unreadable subscriptions store");
            Vec::new()
        }
    }
}

/// Serializes the whole store. Values ride in TOML literal strings so Windows
/// paths keep their backslashes; a value that cannot (a single quote) falls
/// back to a basic string with escaping.
fn render(profiles: &[FleetProfile]) -> String {
    let mut out = String::from(HEADER);
    for profile in profiles {
        out.push('\n');
        out.push_str("[[fleet.profile]]\n");
        out.push_str(&format!("name = \"{}\"\n", profile.name));
        out.push_str(&format!("agent = \"{}\"\n", profile.agent));
        let pairs: Vec<String> = profile
            .env
            .iter()
            .map(|(k, v)| {
                if v.contains('\'') {
                    format!("{k} = \"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
                } else {
                    format!("{k} = '{v}'")
                }
            })
            .collect();
        out.push_str(&format!("env = {{ {} }}\n", pairs.join(", ")));
    }
    out
}

/// Why a link could not be added to the store.
#[derive(Debug, PartialEq, Eq)]
pub enum AddError {
    /// The name is not a safe kebab-case path component.
    InvalidName,
    /// The store already holds a link by that name.
    AlreadyLinked,
}

/// Adds a link, rewriting the store. Refuses before touching anything: an
/// invalid name never names a directory, and a taken name never overwrites a
/// context another link is using.
pub fn add(config_dir: &Path, profile: FleetProfile) -> Result<(), AddError> {
    if !is_valid_name(&profile.name) {
        return Err(AddError::InvalidName);
    }
    let mut profiles = load(config_dir);
    if profiles.iter().any(|p| p.name == profile.name) {
        return Err(AddError::AlreadyLinked);
    }
    profiles.push(profile);
    write(config_dir, &profiles);
    Ok(())
}

/// Removes a link by name, rewriting the store. Returns the removed profile —
/// its env names the context directory the caller must report as left behind —
/// or `None` when the store holds no such link (a hand-written profile lives
/// elsewhere and is not this store's to remove).
pub fn remove(config_dir: &Path, name: &str) -> Option<FleetProfile> {
    let mut profiles = load(config_dir);
    let index = profiles.iter().position(|p| p.name == name)?;
    let removed = profiles.remove(index);
    write(config_dir, &profiles);
    Some(removed)
}

fn write(config_dir: &Path, profiles: &[FleetProfile]) {
    let path = store_path(config_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, render(profiles)) {
        tracing::warn!(path = %path.display(), error = %e, "could not write subscriptions store");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("meltemi-subs-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn profile(name: &str) -> FleetProfile {
        FleetProfile {
            model: None,
            effort: None,
            name: name.into(),
            agent: "provider-a".into(),
            env: vec![(
                "PROVIDER_CONTEXT_DIR".into(),
                r"C:\Users\u\meltemi\subscriptions\work".into(),
            )],
        }
    }

    #[test]
    fn the_store_roundtrips_profiles_with_windows_paths_intact() {
        let d = dir("roundtrip");
        add(&d, profile("work")).unwrap();
        add(&d, profile("personal")).unwrap();
        let loaded = load(&d);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "work");
        assert_eq!(
            loaded[0].env[0].1, r"C:\Users\u\meltemi\subscriptions\work",
            "the literal-string quoting keeps backslashes whole"
        );
        // The header declares the file machine-managed.
        let raw = std::fs::read_to_string(store_path(&d)).unwrap();
        assert!(raw.starts_with("# Managed by Meltemi"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_invalid_name_is_refused_before_anything_exists() {
        // Scenario: El nombre inválido como ruta rehúsa
        let d = dir("badname");
        for bad in [
            "Work", "wo rk", "wo/rk", r"wo\rk", "..", "-work", "work-", "wo--rk", "",
        ] {
            assert_eq!(
                add(&d, profile(bad)),
                Err(AddError::InvalidName),
                "`{bad}` must not name a directory"
            );
        }
        assert!(!store_path(&d).exists(), "a refusal writes nothing at all");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_taken_name_is_refused_and_removal_reports_the_profile() {
        let d = dir("taken");
        add(&d, profile("work")).unwrap();
        assert_eq!(add(&d, profile("work")), Err(AddError::AlreadyLinked));
        let removed = remove(&d, "work").expect("linked, so removable");
        assert_eq!(removed.env[0].0, "PROVIDER_CONTEXT_DIR");
        assert!(load(&d).is_empty());
        assert!(remove(&d, "work").is_none(), "already gone");
        let _ = std::fs::remove_dir_all(&d);
    }
}
