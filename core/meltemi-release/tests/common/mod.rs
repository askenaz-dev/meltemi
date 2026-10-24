// SPDX-License-Identifier: Apache-2.0

//! Fixture trees for the release tool's tests: the five carrier files, written
//! the way this repository writes them.

use std::path::Path;

/// Writes the five carriers into `root`, every one at `version`.
pub fn write_carriers(root: &Path, version: &str) {
    let write = |path: &str, text: String| {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("has a parent")).expect("fixture dir");
        std::fs::write(file, text).expect("fixture file");
    };
    write(
        "Cargo.toml",
        format!(
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\nedition = \"2024\"\n"
        ),
    );
    for json in [
        "desktop/tauri.conf.json",
        "mobile/tauri.conf.json",
        "desktop/ui/package.json",
    ] {
        write(
            json,
            format!("{{\n  \"name\": \"x\",\n  \"version\": \"{version}\"\n}}\n"),
        );
    }
    write(
        "desktop/ui/package-lock.json",
        format!(
            "{{\n  \"name\": \"ui\",\n  \"version\": \"{version}\",\n  \"packages\": {{\n    \"\": {{\n      \"name\": \"ui\",\n      \"version\": \"{version}\"\n    }},\n    \"node_modules/x\": {{\n      \"version\": \"9.9.9\"\n    }}\n  }}\n}}\n"
        ),
    );
}

/// A fresh, empty directory for one test.
pub fn scratch(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("meltemi-release-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    root
}
