// SPDX-License-Identifier: Apache-2.0

//! Dependency audit of the own adapters (adaptadores-propios-acp task 1.1).
//!
//! The claim these adapters rest on is architectural, not rhetorical: every
//! byte that reaches a network leaves from the provider's official binary, so
//! the adapter itself links no HTTP client and no TLS stack. A claim like that
//! rots unless something checks it, so two things do — and this test asserts
//! both are in place:
//!
//! 1. `deny.toml` bans those crates, and CI runs `cargo deny check bans` on
//!    every push (that is the audit the spec asks CI to perform);
//! 2. the adapters' own dependency closure, read straight from `Cargo.lock`,
//!    contains none of them today.
//!
//! The closure is read from the lockfile rather than resolved per platform, so
//! it *over*-approximates: it merges normal, build and dev dependencies of all
//! targets. A stricter check than the binary links is the right error to make.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Crate names that would mean an HTTP client or a TLS stack inside a Meltemi
/// process. Not exhaustive of the ecosystem — exhaustive of what could
/// plausibly arrive here, and matched by the same list `deny.toml` bans.
const NETWORK_STACK: &[&str] = &[
    "reqwest",
    "hyper",
    "rustls",
    "native-tls",
    "openssl",
    "openssl-sys",
    "ureq",
    "curl",
    "isahc",
];

/// The workspace root (this crate lives at `<root>/core/meltemi-adapters`).
fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // core
    dir.pop(); // <root>
    dir
}

/// Every package in `Cargo.lock`, mapped to the names it depends on.
fn lockfile_graph() -> BTreeMap<String, Vec<String>> {
    let lock = std::fs::read_to_string(workspace_root().join("Cargo.lock"))
        .expect("the workspace lockfile is committed");
    let lock: toml::Value = toml::from_str(&lock).expect("Cargo.lock is TOML");
    let mut graph = BTreeMap::new();
    for package in lock["package"].as_array().expect("packages") {
        let name = package["name"].as_str().expect("package name").to_string();
        let deps = package
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|deps| {
                deps.iter()
                    .filter_map(|d| d.as_str())
                    // Entries are `name` or `name version` (and optionally a
                    // source); only the name matters for reachability.
                    .map(|d| d.split_whitespace().next().unwrap_or(d).to_string())
                    .collect()
            })
            .unwrap_or_default();
        graph.insert(name, deps);
    }
    graph
}

/// Everything reachable from `root` in the lockfile graph.
fn closure_of(root: &str) -> BTreeSet<String> {
    let graph = lockfile_graph();
    assert!(
        graph.contains_key(root),
        "`{root}` is missing from Cargo.lock; run `cargo check` to refresh it"
    );
    let mut seen = BTreeSet::new();
    let mut stack = vec![root.to_string()];
    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        for dep in graph.get(&name).into_iter().flatten() {
            stack.push(dep.clone());
        }
    }
    seen
}

/// The crates `deny.toml` bans, each with the wrappers allowed to pull it.
fn banned_crates() -> BTreeMap<String, Vec<String>> {
    let deny = std::fs::read_to_string(workspace_root().join("deny.toml"))
        .expect("the dependency audit configuration is committed");
    let deny: toml::Value = toml::from_str(&deny).expect("deny.toml is TOML");
    let mut banned = BTreeMap::new();
    for entry in deny["bans"]["deny"].as_array().expect("bans.deny entries") {
        let name = entry["crate"].as_str().expect("a banned crate name");
        let wrappers = entry
            .get("wrappers")
            .and_then(|w| w.as_array())
            .map(|w| {
                w.iter()
                    .filter_map(|c| c.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        banned.insert(name.to_string(), wrappers);
    }
    banned
}

#[test]
fn the_adapters_link_no_http_client_and_no_tls_stack() {
    // Scenario: Adaptador sin pila de red
    let closure = closure_of("meltemi-adapters");
    let found: Vec<&&str> = NETWORK_STACK
        .iter()
        .filter(|c| closure.contains(**c))
        .collect();
    assert!(
        found.is_empty(),
        "the adapters' dependency closure reached a network stack: {found:?}. \
         All the network belongs to the provider's official binary (design D2)."
    );

    // And the audit CI runs must be the thing that keeps it that way: every
    // name above is banned, and the exceptions granted (wrappers) are crates
    // the adapters cannot reach.
    let banned = banned_crates();
    for name in NETWORK_STACK {
        let wrappers = banned
            .get(*name)
            .unwrap_or_else(|| panic!("deny.toml must ban `{name}` so CI verifies its absence"));
        for wrapper in wrappers {
            assert!(
                !closure.contains(wrapper),
                "`{name}` is allowed under `{wrapper}`, which the adapters reach: \
                 the exception would let a network stack in through the back door"
            );
        }
    }
}
