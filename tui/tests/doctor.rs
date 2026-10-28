// SPDX-License-Identifier: Apache-2.0

//! `meltemi doctor` against the real binary (meltemi-doctor task 1.2).

use std::path::{Path, PathBuf};
use std::process::Command;

fn meltemi_bin() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(if cfg!(windows) {
        "meltemi.exe"
    } else {
        "meltemi"
    })
}

/// An isolated home for one run: data, configuration, and an endpoint where
/// nothing listens.
fn isolated(tag: &str) -> (PathBuf, String) {
    let base = std::env::temp_dir().join(format!("meltemi-doctor-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    #[cfg(windows)]
    let endpoint = format!(r"\\.\pipe\meltemi-doctor-{tag}-{}", std::process::id());
    #[cfg(unix)]
    let endpoint = base.join("nobody.sock").display().to_string();
    (base, endpoint)
}

fn doctor(base: &Path, endpoint: &str, args: &[&str]) -> std::process::Output {
    Command::new(meltemi_bin())
        .args(args)
        .env("MELTEMI_ENDPOINT", endpoint)
        .env("MELTEMI_DATA_DIR", base.join("data"))
        .env("MELTEMI_CONFIG_DIR", base.join("config"))
        // Even the one variable that lets a client start a daemon at a foreign
        // endpoint must not make the diagnosis start one.
        .env("MELTEMI_START_HERE", "1")
        .env("NO_COLOR", "1")
        .output()
        .expect("run meltemi doctor")
}

#[test]
fn the_diagnosis_runs_without_a_daemon_and_starts_none() {
    // Scenario: El diagnóstico funciona sin daemon y no arranca ninguno
    let (base, endpoint) = isolated("nodaemon");
    let out = doctor(&base, &endpoint, &["doctor"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(14),
        "a failing row exits 14: {text}"
    );
    assert!(
        text.contains("x failing       Daemon: nothing answers at"),
        "the daemon row fails, in its ASCII twin and its word: {text}"
    );
    assert!(
        text.contains("-> `meltemi status` starts the daemon"),
        "{text}"
    );
    for row in ["Client:", "Data folder:", "Configuration folder:", "git:"] {
        assert!(
            text.contains(row),
            "the rest of the report is there ({row}): {text}"
        );
    }
    #[cfg(unix)]
    assert!(
        !std::path::Path::new(&endpoint).exists(),
        "nothing was started at the endpoint"
    );
    assert!(
        !base.join("data").join("logs").exists(),
        "no daemon ran: it would have written its log"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_diagnosis_reads_as_json() {
    // Scenario: El diagnóstico se lee como JSON
    let (base, endpoint) = isolated("json");
    let out = doctor(&base, &endpoint, &["--json", "doctor"]);
    assert_eq!(out.status.code(), Some(14));
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout is one JSON object");
    assert_eq!(report["clean"], false);
    let rows = report["rows"].as_array().expect("rows");
    let daemon = rows
        .iter()
        .find(|row| row["id"] == "daemon")
        .expect("a daemon row");
    assert_eq!(daemon["status"], "fail");
    assert!(daemon["facts"]["endpoint"].is_string());
    assert!(daemon["remedy"].is_string());
    for row in rows {
        for key in ["id", "status", "facts", "title", "found"] {
            assert!(row.get(key).is_some(), "every row has `{key}`: {row}");
        }
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn doctor_takes_no_arguments() {
    let (base, endpoint) = isolated("args");
    let out = doctor(&base, &endpoint, &["doctor", "now"]);
    assert_eq!(out.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&base);
}
