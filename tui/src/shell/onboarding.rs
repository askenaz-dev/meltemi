// SPDX-License-Identifier: Apache-2.0

//! First-run detection for the onboarding overlay (design D4).
//!
//! First run is detected by the absence of a marker file in the user data
//! directory — no account, no network, no telemetry. If the data directory is
//! not writable the marker cannot persist; the onboarding then shows again next
//! time rather than failing, which is acceptable.

use std::path::PathBuf;

fn marker_path() -> PathBuf {
    meltemid::paths::data_dir().join("onboarded")
}

/// Whether this is the first interactive run (no marker present).
#[must_use]
pub fn is_first_run() -> bool {
    !marker_path().exists()
}

/// Records that onboarding has been shown. Best-effort; a failure just means it
/// may show again next time.
pub fn mark_seen() {
    let path = marker_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, b"meltemi onboarding seen\n");
}
