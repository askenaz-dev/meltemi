// SPDX-License-Identifier: Apache-2.0

//! The core-parity gate (constitution §4; gui-tauri-paridad design D3): every
//! client-invocable method of the daemon contract must have a home in the TUI
//! palette registry, in the GUI method registry and in the living matrix
//! `docs/paridad-nucleo.md`. A method without a home fails the workspace test
//! suite — parity is a blocking gate, not a promise.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use meltemi::shell::palette;

/// The connection handshake: performed by every surface, not user-invocable.
const INFRASTRUCTURE: &[&str] = &["initialize"];

/// Daemon → client traffic: delivered to surfaces, never invoked by them.
const DAEMON_INITIATED: &[&str] = &[
    "session/event",
    "permission/request",
    "permission/timeout",
    "permission/changed",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tui/ lives under the repo root")
        .to_path_buf()
}

/// Parses the method-name constants out of the contract's `methods` module.
fn contract_methods() -> BTreeSet<String> {
    let source = fs::read_to_string(repo_root().join("proto/meltemi-proto/src/lib.rs"))
        .expect("read proto lib.rs");
    let module = source
        .split("pub mod methods {")
        .nth(1)
        .expect("methods module present")
        .split("\n}")
        .next()
        .expect("methods module closes");

    let mut methods = BTreeSet::new();
    for line in module.lines() {
        if let Some(rest) = line.split("= \"").nth(1)
            && let Some(name) = rest.split('"').next()
        {
            methods.insert(name.to_string());
        }
    }
    assert!(
        methods.len() >= 40,
        "the contract lists its methods ({} found)",
        methods.len()
    );
    methods
}

fn client_invocable() -> BTreeSet<String> {
    let mut methods = contract_methods();
    for excluded in INFRASTRUCTURE.iter().chain(DAEMON_INITIATED) {
        assert!(
            methods.remove(*excluded),
            "exclusion {excluded} no longer exists in the contract"
        );
    }
    methods
}

// Scenario: Registro obligatorio de método nuevo
#[test]
fn the_tui_palette_registers_every_contract_method_and_nothing_else() {
    let contract = client_invocable();
    let registered: BTreeSet<String> = palette::registered_methods()
        .into_iter()
        .map(str::to_string)
        .collect();

    let missing: Vec<_> = contract.difference(&registered).collect();
    let unknown: Vec<_> = registered.difference(&contract).collect();
    assert!(
        missing.is_empty(),
        "contract methods without a TUI palette home: {missing:?}"
    );
    assert!(
        unknown.is_empty(),
        "TUI palette declares methods outside the contract: {unknown:?}"
    );
}

// Scenario: Método sin casa rompe la CI
#[test]
fn the_gui_registry_registers_every_contract_method_and_nothing_else() {
    let contract = client_invocable();
    let source = fs::read_to_string(repo_root().join("desktop/ui/src/lib/registry.ts"))
        .expect("read the GUI registry");

    let mut registered = BTreeSet::new();
    for chunk in source.split("R(\"").skip(1) {
        if let Some(method) = chunk.split('"').next() {
            registered.insert(method.to_string());
        }
    }

    let missing: Vec<_> = contract.difference(&registered).collect();
    let unknown: Vec<_> = registered.difference(&contract).collect();
    assert!(
        missing.is_empty(),
        "contract methods without a GUI registry home: {missing:?}"
    );
    assert!(
        unknown.is_empty(),
        "GUI registry declares methods outside the contract: {unknown:?}"
    );
}

#[test]
fn the_parity_matrix_documents_every_method() {
    let source = fs::read_to_string(repo_root().join("docs/paridad-nucleo.md"))
        .expect("read the parity matrix");

    for method in contract_methods() {
        assert!(
            source.contains(&format!("`{method}`")),
            "docs/paridad-nucleo.md is missing `{method}`"
        );
    }
}
