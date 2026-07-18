// SPDX-License-Identifier: Apache-2.0

//! Task-list parsing and ticking for the `implement` verb (comando-implement).
//!
//! `implement` deploys agents over a change's `tasks.md` task by task. This
//! module owns the pure parts — reading the checklist and marking a task done —
//! so the orchestration loop in the server composes checkpoint (#17), the agent
//! turn in the worktree (#16), the per-task commit (#18) and the tick here.
//! Ticking `tasks.md` is the resumable progress: a restart skips ticked tasks.

/// One task of a change's `tasks.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// The task id (e.g. `1.2`).
    pub id: String,
    /// The task description (the checklist line, minus id and marker).
    pub description: String,
    /// Whether the task is already ticked.
    pub done: bool,
}

/// Parses the tasks of a `tasks.md`: lines of the form `- [ ] <id> <desc>` or
/// `- [x] <id> <desc>`. Section headers and prose are ignored.
#[must_use]
pub fn parse_tasks(md: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    for line in md.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("- [") else {
            continue;
        };
        // rest is `<mark>] <id> <description>`.
        let Some((mark, after)) = rest.split_once(']') else {
            continue;
        };
        let done = mark.trim().eq_ignore_ascii_case("x");
        let after = after.trim();
        // The id is the first whitespace-delimited token (e.g. `1.2`).
        let (id, description) = match after.split_once(char::is_whitespace) {
            Some((id, desc)) => (id.to_string(), desc.trim().to_string()),
            None => (after.to_string(), String::new()),
        };
        if !id.is_empty() {
            tasks.push(Task {
                id,
                description,
                done,
            });
        }
    }
    tasks
}

/// The first not-yet-done task (the resume point), if any.
#[must_use]
pub fn next_pending(tasks: &[Task]) -> Option<&Task> {
    tasks.iter().find(|t| !t.done)
}

/// Returns `md` with the task `id` ticked (`- [ ]` → `- [x]`). Idempotent: an
/// already-ticked task is left as is. Only the first matching line is ticked.
#[must_use]
pub fn tick_task(md: &str, id: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut done = false;
    for line in md.lines() {
        if !done && line_is_task(line, id) {
            out.push_str(&line.replacen("- [ ]", "- [x]", 1));
            done = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Whether a line is the unticked task with the given id.
fn line_is_task(line: &str, id: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("- [ ]") else {
        return false;
    };
    rest.trim_start()
        .split_once(char::is_whitespace)
        .map(|(first, _)| first == id)
        .unwrap_or_else(|| rest.trim() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASKS: &str =
        "## 1. Work\n\n- [ ] 1.1 First task\n- [x] 1.2 Already done\n- [ ] 2.1 Second task\n";

    #[test]
    fn parses_tasks_with_ids_and_done_state() {
        let tasks = parse_tasks(TASKS);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, "1.1");
        assert_eq!(tasks[0].description, "First task");
        assert!(!tasks[0].done);
        assert!(tasks[1].done, "1.2 is ticked");
    }

    #[test]
    fn next_pending_is_the_resume_point() {
        // Scenario: Progreso sobrevive reinicio — resume at the first undone.
        let tasks = parse_tasks(TASKS);
        assert_eq!(next_pending(&tasks).unwrap().id, "1.1");
        // With 1.1 ticked, the resume point advances past the done 1.2.
        let ticked = tick_task(TASKS, "1.1");
        assert_eq!(next_pending(&parse_tasks(&ticked)).unwrap().id, "2.1");
    }

    #[test]
    fn tick_marks_only_the_target_and_is_idempotent() {
        let once = tick_task(TASKS, "1.1");
        assert!(once.contains("- [x] 1.1 First task"), "ticked: {once}");
        assert!(once.contains("- [ ] 2.1 Second task"), "others untouched");
        // Idempotent: ticking an already-done task changes nothing.
        assert_eq!(tick_task(&once, "1.1"), once);
    }
}
