// SPDX-License-Identifier: Apache-2.0

//! The project registry (multiproyecto-suscripciones design D1/D2/D3): the
//! repositories this user has actually pointed Meltemi at, so the surfaces can
//! offer a tree of projects instead of being caged by the working directory.
//!
//! `<data_dir>/projects/index.jsonl` is append-only, one line per touch, folded
//! last-wins by project key while keeping the first sighting. A file is needed
//! rather than a computation because `project_key` is a truncated SHA-256:
//! the key directories can be enumerated but never inverted back to a path.
//! The invariant that keeps it honest: the registry MUST be rebuildable from
//! the session records themselves — they remain the source of truth — so the
//! file only adds recency and the memory of a project whose history was
//! emptied. Nothing is ever discovered by walking the disk (D3).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One line of the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    /// Record version, so the format can evolve without guessing.
    pub v: u32,
    pub project_key: String,
    pub root: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

const RECORD_VERSION: u32 = 1;

fn index_path(data_dir: &Path) -> PathBuf {
    data_dir.join("projects").join("index.jsonl")
}

/// Records a project as seen now: appended, never rewritten (append-only is
/// what makes a half-written line survivable). Best-effort — a failure here
/// never fails the operation that triggered it.
pub fn touch(data_dir: &Path, root: &Path) {
    let _ = record_seen(data_dir, root);
}

/// Appends "seen now" for `root` and returns the record it wrote: the first
/// sighting is the earliest one the registry ever held, so a repeat is
/// idempotent.
fn record_seen(data_dir: &Path, root: &Path) -> ProjectRecord {
    let key = crate::paths::project_key(root);
    let now = crate::clock::now_rfc3339();
    let first_seen_at = list(data_dir)
        .into_iter()
        .find(|record| record.project_key == key)
        .map(|record| record.first_seen_at)
        .unwrap_or_else(|| now.clone());
    let record = ProjectRecord {
        v: RECORD_VERSION,
        project_key: key,
        root: root.display().to_string(),
        first_seen_at,
        last_seen_at: now,
    };
    if let Ok(line) = serde_json::to_string(&record) {
        append_line(data_dir, &line);
    }
    record
}

/// Appends one raw line to the registry, creating its directory. Best-effort by
/// design: the registry is a convenience over the session records, which are the
/// source of truth, so a write failure must never fail the caller.
fn append_line(data_dir: &Path, line: &str) {
    let path = index_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// The registered projects, most recently seen first. Unparsable lines are
/// skipped: a corrupt tail must not hide the rest of the history.
pub fn list(data_dir: &Path) -> Vec<ProjectRecord> {
    let Ok(text) = std::fs::read_to_string(index_path(data_dir)) else {
        return Vec::new();
    };
    // The append order IS the recency order, and the timestamps have one-second
    // resolution: two touches inside the same second tie, so the line position
    // breaks the tie. Without it, "most recently used first" would be luck.
    let mut order: BTreeMap<String, usize> = BTreeMap::new();
    let mut folded: BTreeMap<String, ProjectRecord> = BTreeMap::new();
    for (position, line) in text.lines().enumerate() {
        let Ok(record) = serde_json::from_str::<ProjectRecord>(line) else {
            continue;
        };
        order.insert(record.project_key.clone(), position);
        match folded.get_mut(&record.project_key) {
            // Last write wins for the root and the recency; the first sighting
            // is the earliest one ever recorded.
            Some(existing) => {
                if record.first_seen_at < existing.first_seen_at {
                    existing.first_seen_at = record.first_seen_at.clone();
                }
                existing.root = record.root;
                existing.last_seen_at = record.last_seen_at;
            }
            None => {
                folded.insert(record.project_key.clone(), record);
            }
        }
    }
    let mut records: Vec<ProjectRecord> = folded.into_values().collect();
    records.sort_by(|a, b| {
        b.last_seen_at.cmp(&a.last_seen_at).then_with(|| {
            let (left, right) = (
                order.get(&a.project_key).copied().unwrap_or(0),
                order.get(&b.project_key).copied().unwrap_or(0),
            );
            right.cmp(&left)
        })
    });
    records
}

/// Rebuilds the registry from the session records, which are the source of
/// truth (D2). Used when the index is missing or was lost: every project with
/// a session history reappears, with the recency its sessions declare.
pub fn rebuild_from_sessions(data_dir: &Path) -> Vec<ProjectRecord> {
    let mut records = Vec::new();
    for key in crate::session_index::all_project_keys(data_dir) {
        let sessions = crate::session_index::records_for_project(data_dir, &key);
        let Some(first) = sessions.first() else {
            continue;
        };
        let root = first.project_root.clone();
        let started: Vec<String> = sessions.iter().map(|s| s.started_at.clone()).collect();
        let first_seen_at = started.iter().min().cloned().unwrap_or_default();
        let last_seen_at = sessions
            .iter()
            .map(|s| s.ended_at.clone().unwrap_or_else(|| s.started_at.clone()))
            .max()
            .unwrap_or_else(|| first_seen_at.clone());
        records.push(ProjectRecord {
            v: RECORD_VERSION,
            project_key: key,
            root,
            first_seen_at,
            last_seen_at,
        });
    }
    records.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
    records
}

/// Handles `project/list`: the registered projects, rebuilt from the session
/// history when the index is absent, each with whether its root still exists
/// and how many sessions it holds.
pub async fn handle_project_list(
    params: serde_json::Value,
    state: &std::sync::Arc<crate::server::DaemonState>,
) -> Result<serde_json::Value, crate::rpc::RpcError> {
    use meltemi_proto::{ProjectListParams, ProjectListResult};

    let params: ProjectListParams = if params.is_null() {
        ProjectListParams::default()
    } else {
        serde_json::from_value(params)
            .map_err(|e| crate::rpc::RpcError::invalid_params(format!("project/list: {e}")))?
    };

    let mut records = list(&state.data_dir);
    if records.is_empty() {
        records = rebuild_from_sessions(&state.data_dir);
    }

    let live = live_sessions(state).await;
    let mut projects = Vec::with_capacity(records.len());
    for record in records {
        let info = to_info(&state.data_dir, record, &live);
        if params.existing_only.unwrap_or(false) && !info.exists {
            continue;
        }
        projects.push(info);
    }

    serde_json::to_value(ProjectListResult { projects }).map_err(crate::rpc::RpcError::internal)
}

/// Which sessions are running RIGHT NOW, by id: the registry is history, the
/// live state belongs to the session registry.
async fn live_sessions(
    state: &std::sync::Arc<crate::server::DaemonState>,
) -> std::collections::HashSet<String> {
    state
        .sessions
        .summaries()
        .await
        .into_iter()
        .filter(|summary| {
            matches!(
                summary.state,
                meltemi_proto::SessionState::Starting
                    | meltemi_proto::SessionState::Active
                    | meltemi_proto::SessionState::WaitingPermission
            )
        })
        .map(|summary| summary.session_id)
        .collect()
}

/// One registry record as the contract reports it. Shared by `project/list` and
/// `project/register` so a freshly registered project is described in exactly
/// the shape the listing would describe it in — one row, one composition.
fn to_info(
    data_dir: &Path,
    record: ProjectRecord,
    live: &std::collections::HashSet<String>,
) -> meltemi_proto::ProjectInfo {
    let sessions = crate::session_index::records_for_project(data_dir, &record.project_key);
    // Live counts are the client's job: `session/list` already reports every
    // session with its real state and its project root, so a single response
    // aggregates the tree (design D7). Here we only report what the history
    // knows, which is stable and cheap.
    meltemi_proto::ProjectInfo {
        exists: PathBuf::from(&record.root).is_dir(),
        project_key: record.project_key,
        root: record.root,
        first_seen_at: record.first_seen_at,
        last_seen_at: record.last_seen_at,
        sessions_total: sessions.len() as u32,
        active_sessions: sessions
            .iter()
            .filter(|session| live.contains(&session.session_id))
            .count() as u32,
        resumable_sessions: sessions
            .iter()
            .filter(|session| session.resumable())
            .count() as u32,
    }
}

/// The canonical form of a path, for the registry to store and show. Canonical
/// because `project/list` resolves the root last-wins and the surfaces print
/// whatever it holds: without this the registry would show whichever spelling
/// was typed last, and a later comparison by path would have to argue with it.
///
/// On Windows `canonicalize` answers in the `\\?\` extended-length form, which
/// is canonical and unusable — no user recognizes their own repository in it and
/// no shell they paste it into likes it. The prefix is stripped back off for a
/// plain drive path (UNC forms are left exactly as they came), and that costs
/// nothing downstream: `project_key` canonicalizes again before hashing, so both
/// spellings key to the same project.
fn canonical(root: &Path) -> PathBuf {
    let Ok(resolved) = root.canonicalize() else {
        return root.to_path_buf();
    };
    #[cfg(windows)]
    {
        let shown = resolved.to_string_lossy();
        if let Some(plain) = shown.strip_prefix(r"\\?\")
            && !plain.starts_with("UNC\\")
        {
            return PathBuf::from(plain);
        }
    }
    resolved
}

/// Handles `project/register`: an explicit entry, with the path the client hands
/// over. Validated and canonicalized, idempotent, and inert — nothing is created
/// inside the root, nothing outside it is read, and `.meltemi/` is not required,
/// because registering is aiming the tool at a directory rather than initializing
/// it as a project (design D6).
pub async fn handle_project_register(
    params: serde_json::Value,
    state: &std::sync::Arc<crate::server::DaemonState>,
) -> Result<serde_json::Value, crate::rpc::RpcError> {
    use meltemi_proto::{ProjectRegisterParams, ProjectRegisterResult, error_codes};

    let params: ProjectRegisterParams = serde_json::from_value(params)
        .map_err(|e| crate::rpc::RpcError::invalid_params(format!("project/register: {e}")))?;
    let root = PathBuf::from(&params.root);
    if !root.is_dir() {
        return Err(crate::rpc::RpcError::application(
            error_codes::PROJECT_ROOT_INVALID,
            "invalid project root",
            "project_root_invalid",
            format!("`{}` is not an existing directory", root.display()),
            Some("Pass the absolute path to an existing directory.".into()),
        ));
    }
    let record = record_seen(&state.data_dir, &canonical(&root));
    let live = live_sessions(state).await;
    let project = to_info(&state.data_dir, record, &live);
    serde_json::to_value(ProjectRegisterResult { project }).map_err(crate::rpc::RpcError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mel-projects-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    // Scenario: Alta repetida no duplica el proyecto
    // Scenario: Una sesión estrena el proyecto en el registro
    #[test]
    fn touching_a_project_registers_it_once_with_its_recency() {
        let data = temp("touch");
        let project = temp("repo-a");
        touch(&data, &project);
        touch(&data, &project);

        let records = list(&data);
        assert_eq!(records.len(), 1, "one entry per project, folded last-wins");
        assert_eq!(records[0].root, project.display().to_string());
        assert!(records[0].first_seen_at <= records[0].last_seen_at);

        // A second project appears beside it, most recent first.
        let other = temp("repo-b");
        touch(&data, &other);
        let records = list(&data);
        assert_eq!(records.len(), 2);
        assert!(records[0].last_seen_at >= records[1].last_seen_at);

        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_corrupt_line_never_hides_the_rest() {
        let data = temp("corrupt");
        let project = temp("repo-c");
        touch(&data, &project);
        let path = index_path(&data);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append");
        writeln!(file, "{{not json").expect("write");
        drop(file);
        touch(&data, &project);
        assert_eq!(list(&data).len(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_absent_index_yields_no_projects_rather_than_an_error() {
        let data = temp("absent");
        assert!(list(&data).is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }

    // Scenario: Ningún proyecto aparece sin haberse usado
    #[test]
    fn reading_the_registry_never_registers_anything() {
        let data = temp("read-only");
        // A query on an empty registry answers empty and writes nothing: the
        // registry grows from real use (a session start), never from a query,
        // and nothing is ever discovered by walking the user's disk (D3).
        assert!(list(&data).is_empty());
        assert!(rebuild_from_sessions(&data).is_empty());
        assert!(
            !index_path(&data).exists(),
            "a read created the registry file"
        );

        // With one real use recorded, repeated reads leave the file untouched.
        let project = temp("repo-read");
        touch(&data, &project);
        let before = std::fs::read_to_string(index_path(&data)).expect("index");
        for _ in 0..3 {
            let _ = list(&data);
            let _ = rebuild_from_sessions(&data);
        }
        let after = std::fs::read_to_string(index_path(&data)).expect("index");
        assert_eq!(before, after, "reads must not append");
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }
}
