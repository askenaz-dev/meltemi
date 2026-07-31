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
//!
//! Two verbs put the user in charge of the list (lanzador-conversacional D6).
//! `project/register` adds a directory the client hands over — validated,
//! canonicalized, idempotent — so a folder can be pointed at before anything has
//! ever run in it; it requires no `.meltemi/`, creates nothing inside the root
//! and reads nothing outside it. `project/forget` appends a forget line, and
//! **rules over the listing and nothing else**: it deletes no file, ends no
//! session, hides no session log and removes nothing from the analytics, and the
//! project reappears the moment it is used or registered again. It is not a
//! promise of permanence either — a registry that has to be rebuilt from the
//! session records comes back whole, tombstones included in what is lost,
//! because those records outrank this file by design.

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
/// idempotent, and a project that had been forgotten reappears — a later line
/// wins the fold, which is the whole mechanism.
fn record_seen(data_dir: &Path, root: &Path) -> ProjectRecord {
    // Canonicalized here rather than at each caller, because the promise is
    // about the registry and not about one verb: whichever door a project comes
    // in by, the row it leaves behind spells the root the same way. `canonical`
    // is idempotent, so the callers that already resolved a path pay nothing.
    let root = &canonical(root);
    let key = crate::paths::project_key(root);
    let now = crate::clock::now_rfc3339();
    // Looked up in the FOLD rather than the listing: a forgotten project keeps
    // the day it was first seen when it comes back, because forgetting hides a
    // row, it does not erase a history.
    let first_seen_at = fold(data_dir)
        .all
        .get(&key)
        .map(|record| record.first_seen_at.clone())
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

/// The prefix of a forget line. The exact precedent is next door:
/// `worktrees::list` has folded `REMOVED <path>` lines since
/// orquestacion-worktrees, for the same reason — an append-only file cannot
/// delete, so it tombstones (design D6).
const FORGOTTEN: &str = "FORGOTTEN ";

/// The folded registry: what is listed, what the registry has ever known, and
/// whether anything at all was readable.
struct Fold {
    /// Folded records that no later line forgot, most recently seen first.
    visible: Vec<ProjectRecord>,
    /// Every folded record, forgotten ones included: forgetting hides a row, it
    /// does not erase what the registry knew about it.
    all: BTreeMap<String, ProjectRecord>,
    /// Whether ANY line of the file parsed — as a record or as a tombstone.
    /// This is the distinction the rebuild hangs on: an empty listing because
    /// everything was forgotten is an answer, and an empty listing because there
    /// is nothing to read is a missing file (design D6).
    readable: bool,
}

/// Folds the append-only registry last-wins by project key. Unparsable lines are
/// skipped: a corrupt tail must not hide the rest of the history.
fn fold(data_dir: &Path) -> Fold {
    let Ok(text) = std::fs::read_to_string(index_path(data_dir)) else {
        return Fold {
            visible: Vec::new(),
            all: BTreeMap::new(),
            readable: false,
        };
    };
    // The append order IS the recency order, and the timestamps have one-second
    // resolution: two touches inside the same second tie, so the line position
    // breaks the tie. Without it, "most recently used first" would be luck.
    let mut order: BTreeMap<String, usize> = BTreeMap::new();
    let mut folded: BTreeMap<String, ProjectRecord> = BTreeMap::new();
    let mut forgotten: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut readable = false;
    for (position, line) in text.lines().enumerate() {
        if let Some(key) = line.strip_prefix(FORGOTTEN) {
            let key = key.trim();
            if !key.is_empty() {
                readable = true;
                forgotten.insert(key.to_string());
            }
            continue;
        }
        let Ok(record) = serde_json::from_str::<ProjectRecord>(line) else {
            continue;
        };
        readable = true;
        // A record after a tombstone un-forgets the project: last line wins, and
        // that is exactly how a forgotten project reappears when it is used or
        // registered again.
        forgotten.remove(&record.project_key);
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
    let mut visible: Vec<ProjectRecord> = folded
        .values()
        .filter(|record| !forgotten.contains(&record.project_key))
        .cloned()
        .collect();
    visible.sort_by(|a, b| {
        b.last_seen_at.cmp(&a.last_seen_at).then_with(|| {
            let (left, right) = (
                order.get(&a.project_key).copied().unwrap_or(0),
                order.get(&b.project_key).copied().unwrap_or(0),
            );
            right.cmp(&left)
        })
    });
    Fold {
        visible,
        all: folded,
        readable,
    }
}

/// The registered projects, most recently seen first, without the ones a forget
/// line dropped from the listing.
pub fn list(data_dir: &Path) -> Vec<ProjectRecord> {
    fold(data_dir).visible
}

/// Compares two roots the way a human means them: a trailing separator is not a
/// different directory, and on Windows neither is a different case or a forward
/// slash. Used only to find a registered root that can no longer be
/// canonicalized — which is precisely the root somebody wants to forget.
fn same_root(left: &str, right: &str) -> bool {
    fn normalize(path: &str) -> String {
        let unified = path.replace('\\', "/");
        let trimmed = unified.trim_end_matches('/');
        if cfg!(windows) {
            trimmed.to_lowercase()
        } else {
            trimmed.to_string()
        }
    }
    normalize(left) == normalize(right)
}

/// Drops a project from the listing by appending a forget line. Returns whether
/// the registry was listing that root and no longer is; `false` says it was not
/// listed to begin with, which is not an error — the caller asked for a state,
/// and that state already held.
///
/// Nothing on disk is deleted, and nothing but the listing changes: the sessions
/// keep listing, their logs keep reading and the analytics keep counting them.
pub fn forget(data_dir: &Path, root: &Path) -> bool {
    let listed = list(data_dir);
    // The key when the path still resolves, which is the exact same key the
    // registry derived when it stored the project.
    let by_key = root
        .canonicalize()
        .ok()
        .map(|_| crate::paths::project_key(root))
        .filter(|key| listed.iter().any(|record| &record.project_key == key));
    // Otherwise match the registered roots as text: a root that no longer exists
    // cannot be canonicalized, and it is the likeliest thing to be forgetting.
    let key = by_key.or_else(|| {
        let target = root.display().to_string();
        listed
            .into_iter()
            .find(|record| same_root(&record.root, &target))
            .map(|record| record.project_key)
    });
    match key {
        Some(key) => {
            append_line(data_dir, &format!("{FORGOTTEN}{key}"));
            true
        }
        None => false,
    }
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

    let folded = fold(&state.data_dir);
    // The rebuild fires for a registry that cannot be READ, never for one that
    // reads as empty. With tombstones those stopped being the same thing: a user
    // who forgot every project would otherwise watch them all march back on the
    // next listing, and the forget would be a no-op wearing a result (D6).
    let records = if folded.readable {
        folded.visible
    } else {
        rebuild_from_sessions(&state.data_dir)
    };

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
    let record = record_seen(&state.data_dir, &root);
    let live = live_sessions(state).await;
    let project = to_info(&state.data_dir, record, &live);
    serde_json::to_value(ProjectRegisterResult { project }).map_err(crate::rpc::RpcError::internal)
}

/// Handles `project/forget`: one line over the listing, and nothing else. No
/// file, no session, no session log and no analytics record is touched, and the
/// root need not exist — a directory that vanished is precisely the one worth
/// forgetting, so demanding a canonicalizable path would make it unforgettable
/// (design D6).
pub async fn handle_project_forget(
    params: serde_json::Value,
    state: &std::sync::Arc<crate::server::DaemonState>,
) -> Result<serde_json::Value, crate::rpc::RpcError> {
    use meltemi_proto::{ProjectForgetParams, ProjectForgetResult};

    let params: ProjectForgetParams = serde_json::from_value(params)
        .map_err(|e| crate::rpc::RpcError::invalid_params(format!("project/forget: {e}")))?;
    let forgotten = forget(&state.data_dir, Path::new(&params.root));
    serde_json::to_value(ProjectForgetResult { forgotten }).map_err(crate::rpc::RpcError::internal)
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

    /// The entries of a directory, sorted: enough to see whether anything was
    /// created inside a root that was only supposed to be pointed at.
    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// A daemon state whose data directory is `data`, for the handlers that need
    /// one. Nothing else about it is exercised here.
    fn state_over(data: &Path) -> std::sync::Arc<crate::server::DaemonState> {
        let (shutdown, _rx) = tokio::sync::mpsc::channel(1);
        crate::server::DaemonState::new(data.to_path_buf(), data.join("config"), shutdown)
    }

    /// Writes one session record for `root`, so the rebuild has something to
    /// rebuild FROM — without it this test would pass for the wrong reason.
    fn record_a_session(data: &Path, root: &Path) {
        let key = crate::paths::project_key(root);
        crate::session_index::append(
            data,
            &key,
            &crate::session_index::SessionRecord {
                session_id: "sess-forgotten".into(),
                agent_command: vec!["mock-agent".into()],
                project_root: root.display().to_string(),
                level: 1,
                started_at: "2026-07-31T10:00:00Z".into(),
                ended_at: Some("2026-07-31T10:01:00Z".into()),
                final_status: Some(meltemi_proto::TurnStatus::Completed),
                agent_session_id: None,
                supports_load: false,
                resumed_from: None,
                agent_id: None,
                profile: None,
            },
        )
        .expect("session record");
    }

    // Scenario: Todo olvidado no dispara la reconstrucción
    #[tokio::test]
    async fn forgetting_everything_leaves_an_empty_listing_rather_than_a_rebuild() {
        let data = temp("all-forgotten");
        let project = temp("repo-forgotten");
        record_a_session(&data, &project);
        touch(&data, &project);
        assert_eq!(list(&data).len(), 1);

        assert!(forget(&data, &project), "the project was listed");
        assert!(list(&data).is_empty(), "and now it is not");

        // The rebuild reads the session records, which still describe this
        // project — and would hand it straight back if the empty listing were
        // read as an unreadable registry.
        assert_eq!(
            rebuild_from_sessions(&data).len(),
            1,
            "the session records still know it: that is the trap"
        );
        let state = state_over(&data);
        let listed = handle_project_list(serde_json::Value::Null, &state)
            .await
            .expect("project/list ok");
        assert_eq!(
            listed["projects"].as_array().map(Vec::len),
            Some(0),
            "a forget that the next listing undoes is not a forget: {listed:#}"
        );

        // And the rebuild still fires where it is meant to: no registry file at
        // all is a different thing from a registry that says "nothing".
        let fresh = temp("no-registry");
        record_a_session(&fresh, &project);
        let state = state_over(&fresh);
        let rebuilt = handle_project_list(serde_json::Value::Null, &state)
            .await
            .expect("project/list ok");
        assert_eq!(
            rebuilt["projects"].as_array().map(Vec::len),
            Some(1),
            "a missing registry is still rebuilt from the session records: {rebuilt:#}"
        );

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&fresh);
        let _ = std::fs::remove_dir_all(&project);
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

    // Scenario: Alta de un directorio que nunca corrió una sesión
    // Scenario: Un alta explícita estrena el proyecto sin correr nada
    // Scenario: El alta no toca el disco del usuario
    #[tokio::test]
    async fn registering_a_directory_lists_it_without_touching_it() {
        let data = temp("register-fresh");
        let project = temp("repo-fresh");
        // A plain directory: no `.meltemi/`, no session, nothing ever run here.
        std::fs::write(project.join("README.md"), "hello\n").expect("write");
        let before = entries(&project);

        let state = state_over(&data);
        let registered = handle_project_register(
            serde_json::json!({ "root": project.display().to_string() }),
            &state,
        )
        .await
        .expect("project/register ok");

        assert_eq!(
            registered["project"]["sessionsTotal"], 0,
            "nothing ran to get it here: {registered:#}"
        );
        assert_eq!(registered["project"]["exists"], true);
        assert!(
            list(&data).iter().any(|record| record.project_key
                == registered["project"]["projectKey"].as_str().unwrap()),
            "the registry lists it"
        );

        // The root is exactly as it was: registering is aiming the tool at a
        // folder, not initializing one.
        assert_eq!(
            entries(&project),
            before,
            "the daemon wrote inside the root"
        );
        assert!(!project.join(".meltemi").exists());

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }

    // Scenario: Alta repetida no duplica ni pierde la primera vez
    #[tokio::test]
    async fn a_repeated_registration_folds_to_one_entry_and_keeps_the_first_sighting() {
        let data = temp("register-twice");
        let project = temp("repo-twice");
        let nested = project.join("inner");
        std::fs::create_dir_all(&nested).expect("nested");

        // Seed a sighting from long ago, so "keeps the first" is an assertion
        // about a real date rather than about two calls in the same second.
        let key = crate::paths::project_key(&project);
        append_line(
            &data,
            &serde_json::to_string(&ProjectRecord {
                v: RECORD_VERSION,
                project_key: key.clone(),
                root: project.display().to_string(),
                first_seen_at: "2020-01-01T00:00:00Z".into(),
                last_seen_at: "2020-01-01T00:00:00Z".into(),
            })
            .expect("record"),
        );

        let state = state_over(&data);
        // The same folder, spelled the long way round.
        let equivalent = nested.join("..");
        let registered = handle_project_register(
            serde_json::json!({ "root": equivalent.display().to_string() }),
            &state,
        )
        .await
        .expect("project/register ok");

        let records = list(&data);
        assert_eq!(records.len(), 1, "two spellings, one project: {records:#?}");
        assert_eq!(
            records[0].first_seen_at, "2020-01-01T00:00:00Z",
            "the first sighting is the earliest the registry ever held"
        );
        assert!(
            records[0].last_seen_at > records[0].first_seen_at,
            "and the last moved"
        );
        assert_eq!(
            records[0].root,
            canonical(&project).display().to_string(),
            "the listing shows the canonical root, not the spelling last typed"
        );
        assert_eq!(registered["project"]["projectKey"], key.as_str());

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }

    // Scenario: Ruta inexistente rehusada con remedio
    #[tokio::test]
    async fn registering_a_path_that_is_not_a_directory_is_refused_with_a_remedy() {
        let data = temp("register-invalid");
        let project = temp("repo-invalid");
        let file = project.join("a-file.txt");
        std::fs::write(&file, "not a directory\n").expect("write");
        let state = state_over(&data);

        for candidate in [project.join("nowhere"), file] {
            let refused = handle_project_register(
                serde_json::json!({ "root": candidate.display().to_string() }),
                &state,
            )
            .await
            .expect_err("only an existing directory can be registered");
            assert_eq!(
                refused.code,
                meltemi_proto::error_codes::PROJECT_ROOT_INVALID
            );
            let data_field = refused.data.clone().expect("a refusal carries data");
            assert!(
                !data_field["remedy"].as_str().unwrap_or_default().is_empty(),
                "with something to do about it: {refused}"
            );
        }
        assert!(list(&data).is_empty(), "the registry is unchanged");

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }

    // Scenario: Olvidar oculta del listado y conserva todo lo demás
    // Scenario: Un proyecto olvidado reaparece al volver a usarse
    #[tokio::test]
    async fn forgetting_hides_the_row_and_using_it_again_brings_it_back() {
        let data = temp("forget-hide");
        let project = temp("repo-hide");
        record_a_session(&data, &project);
        touch(&data, &project);
        let first_seen = list(&data)[0].first_seen_at.clone();
        let key = list(&data)[0].project_key.clone();

        let state = state_over(&data);
        let forgotten = handle_project_forget(
            serde_json::json!({ "root": project.display().to_string() }),
            &state,
        )
        .await
        .expect("project/forget ok");
        assert_eq!(forgotten["forgotten"], true);
        assert!(list(&data).is_empty(), "gone from the listing");

        // Everything else survives: the session records are untouched, the root
        // is untouched, and forgetting twice is not an error.
        assert_eq!(
            crate::session_index::records_for_project(&data, &key).len(),
            1,
            "its sessions still list"
        );
        assert!(project.is_dir(), "nothing on disk was deleted");
        let again = handle_project_forget(
            serde_json::json!({ "root": project.display().to_string() }),
            &state,
        )
        .await
        .expect("project/forget ok");
        assert_eq!(
            again["forgotten"], false,
            "it was not listed to begin with, which is a state and not an error"
        );

        // Using it again brings it back, with the day it was first seen.
        touch(&data, &project);
        let records = list(&data);
        assert_eq!(records.len(), 1, "a later line wins the fold");
        assert_eq!(
            records[0].first_seen_at, first_seen,
            "forgetting hid a row; it did not erase a history"
        );

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }

    // Scenario: Olvidar una raíz que ya no existe en disco
    #[tokio::test]
    async fn a_root_that_vanished_is_still_forgettable() {
        let data = temp("forget-gone");
        let project = temp("repo-gone");
        touch(&data, &project);
        let listed = list(&data)[0].root.clone();
        std::fs::remove_dir_all(&project).expect("the directory disappears");
        assert!(!project.exists());

        // A path that cannot be canonicalized is precisely the one worth
        // forgetting, so the match falls back to comparing the registered roots
        // as text — trailing separator and all.
        let state = state_over(&data);
        let forgotten = handle_project_forget(
            serde_json::json!({ "root": format!("{listed}{}", std::path::MAIN_SEPARATOR) }),
            &state,
        )
        .await
        .expect("project/forget ok");
        assert_eq!(
            forgotten["forgotten"], true,
            "an absent root is forgettable"
        );
        assert!(list(&data).is_empty());

        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    async fn a_corrupt_line_hides_neither_a_project_nor_a_tombstone() {
        let data = temp("forget-corrupt");
        let kept = temp("repo-kept");
        let dropped = temp("repo-dropped");
        touch(&data, &kept);
        touch(&data, &dropped);
        assert!(forget(&data, &dropped));
        append_line(&data, "{ this is not json");
        append_line(&data, "FORGOTTEN ");

        let records = list(&data);
        assert_eq!(
            records.len(),
            1,
            "the readable lines still fold: {records:#?}"
        );
        assert_eq!(records[0].root, kept.display().to_string());

        // A garbage tail does not resurrect what a tombstone dropped, and an
        // empty tombstone forgets nobody.
        let state = state_over(&data);
        let listed = handle_project_list(serde_json::Value::Null, &state)
            .await
            .expect("project/list ok");
        assert_eq!(listed["projects"].as_array().map(Vec::len), Some(1));

        // And reading changed nothing on disk.
        let before = std::fs::read_to_string(index_path(&data)).expect("index");
        let _ = handle_project_list(serde_json::Value::Null, &state).await;
        let _ = list(&data);
        assert_eq!(
            std::fs::read_to_string(index_path(&data)).expect("index"),
            before,
            "a listing must never write"
        );

        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&kept);
        let _ = std::fs::remove_dir_all(&dropped);
    }
}
