// SPDX-License-Identifier: Apache-2.0

//! Surface-side file access for the utilitarian editor (edit-surface):
//! contained reads and project search. Reading is a surface concern — like
//! the TUI handing the terminal to `$EDITOR` — while every WRITE goes through
//! the daemon (`worktree/apply-edit`) for traceability. Nothing here leaves
//! the given tree, and nothing touches the network.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bridge::BridgeError;

/// Files beyond this size are refused for display (utilitarian editor, not a
/// data viewer).
const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;
/// Search caps: files larger than this are skipped; results stop at the cap.
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SEARCH_RESULTS: usize = 500;

fn refused(detail: impl Into<String>) -> BridgeError {
    BridgeError {
        code: None,
        message: "file operation refused".into(),
        kind: "fs_refused".into(),
        detail: Some(detail.into()),
        remedy: None,
    }
}

/// Joins `file` under `root`, refusing escapes (same discipline as the
/// daemon's apply-edit containment).
fn contained(root: &str, file: &str) -> Result<PathBuf, BridgeError> {
    if file.contains("..") || Path::new(file).is_absolute() {
        return Err(refused(format!("`{file}` escapes the tree")));
    }
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(refused(format!("`{}` is not a directory", root.display())));
    }
    Ok(root.join(file))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeFile {
    pub file: String,
    pub content: String,
}

/// Reads one UTF-8 file inside the tree.
#[tauri::command]
pub fn tree_read(root: String, file: String) -> Result<TreeFile, BridgeError> {
    let path = contained(&root, &file)?;
    let meta = std::fs::metadata(&path).map_err(|e| refused(format!("{file}: {e}")))?;
    if !meta.is_file() {
        return Err(refused(format!("`{file}` is not a file")));
    }
    if meta.len() > MAX_READ_BYTES {
        return Err(refused(format!(
            "`{file}` exceeds the editor's {MAX_READ_BYTES}-byte budget"
        )));
    }
    let bytes = std::fs::read(&path).map_err(|e| refused(format!("{file}: {e}")))?;
    let content = String::from_utf8(bytes)
        .map_err(|_| refused(format!("`{file}` is not UTF-8 text (binary?)")))?;
    Ok(TreeFile { file, content })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub file: String,
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
}

/// Case-insensitive substring search over the tree, honoring `.gitignore`
/// (the `ignore` crate, the same discipline as the daemon's repo map).
#[tauri::command]
pub fn tree_search(root: String, query: String) -> Result<SearchResult, BridgeError> {
    let needle = query.to_lowercase();
    if needle.trim().is_empty() {
        return Ok(SearchResult {
            matches: Vec::new(),
            truncated: false,
        });
    }
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(refused(format!("`{root}` is not a directory")));
    }

    let mut matches = Vec::new();
    let mut truncated = false;
    'walk: for entry in ignore::WalkBuilder::new(&root_path).hidden(false).build() {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.metadata().map(|m| m.len()).unwrap_or(u64::MAX) > MAX_SEARCH_FILE_BYTES {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue; // non-UTF-8: skipped, not an error
        };
        let relative = path
            .strip_prefix(&root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        for (index, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                if matches.len() >= MAX_SEARCH_RESULTS {
                    truncated = true;
                    break 'walk;
                }
                matches.push(SearchMatch {
                    file: relative.clone(),
                    line: (index + 1) as u32,
                    text: line.trim().chars().take(200).collect(),
                });
            }
        }
    }
    Ok(SearchResult { matches, truncated })
}

/// "Open with…": hands the file (and line) to the editor the user already
/// uses (edit-surface deep-link). Resolution: the `MELTEMI_OPEN_WITH`
/// template (`{file}`/`{line}` placeholders), then VS Code's `code -g` when
/// on PATH, then the OS opener (without line addressing).
#[tauri::command]
pub fn open_with(root: String, file: String, line: Option<u32>) -> Result<String, BridgeError> {
    let path = contained(&root, &file)?;
    let path_str = path.display().to_string();
    let line = line.unwrap_or(1);

    if let Ok(template) = std::env::var("MELTEMI_OPEN_WITH")
        && !template.trim().is_empty()
    {
        let rendered = template
            .replace("{file}", &path_str)
            .replace("{line}", &line.to_string());
        let mut parts = rendered.split_whitespace();
        let program = parts.next().ok_or_else(|| refused("empty template"))?;
        spawn_detached(program, &parts.map(String::from).collect::<Vec<_>>())?;
        return Ok(format!("template: {program}"));
    }

    if let Some(code) = find_on_path(if cfg!(windows) { "code.cmd" } else { "code" }) {
        spawn_detached(
            &code.display().to_string(),
            &["-g".into(), format!("{path_str}:{line}")],
        )?;
        return Ok("code -g".into());
    }

    // OS opener: no line addressing — honest fallback. `start` consumes the
    // first quoted argument as a window title, so one is always supplied or a
    // path with spaces would be eaten as the title.
    #[cfg(windows)]
    spawn_detached(
        "cmd",
        &["/C".into(), "start".into(), "meltemi".into(), path_str],
    )?;
    #[cfg(target_os = "macos")]
    spawn_detached("open", &[path_str])?;
    #[cfg(all(unix, not(target_os = "macos")))]
    spawn_detached("xdg-open", &[path_str])?;
    Ok("system opener".into())
}

fn spawn_detached(program: &str, args: &[String]) -> Result<(), BridgeError> {
    std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| refused(format!("{program}: {e}")))
}

/// Finds a program on PATH (the BYO detection primitive).
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_contained_to_the_tree() {
        let dir = std::env::temp_dir().join(format!("mel-fsops-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("inside.txt"), "ok").unwrap();
        let root = dir.display().to_string();

        let ok = tree_read(root.clone(), "inside.txt".into()).expect("inside is readable");
        assert_eq!(ok.content, "ok");
        assert!(tree_read(root.clone(), "../escape.txt".into()).is_err());
        assert!(tree_read(root, "C:/absolute.txt".into()).is_err());
    }

    #[test]
    fn search_finds_lines_and_reports_truncation_honestly() {
        let dir = std::env::temp_dir().join(format!("mel-fsearch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn alpha() {}\nfn beta() {}\n").unwrap();
        let result = tree_search(dir.display().to_string(), "ALPHA".into()).expect("search runs");
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].line, 1);
        assert!(!result.truncated);
    }
}
