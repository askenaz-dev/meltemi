// SPDX-License-Identifier: Apache-2.0

//! Local usage accounting (analitica-consumo-local design D1–D6).
//!
//! Everything here is computed on demand by folding records the daemon already
//! keeps: the session index (a prefilter — its start/end marks decide which
//! logs must be opened for the requested range) and the per-project JSONL
//! session logs (the facts), plus the project's human-edit log when its root is
//! reachable. There is no second store to drift from the logs, and no
//! incremental counter to desynchronize after a crash.
//!
//! Two honesty rules are load-bearing and are enforced by tests:
//!
//! 1. **Tokens are only ever measured.** They come exclusively from
//!    `usage_reported` events, which carry what an official structured output
//!    declared. Nothing is estimated, no provider account is consulted, and a
//!    counter the output did not declare stays *absent* — never zero.
//! 2. **Measured and unreported never share a figure.** Each cell declares its
//!    coverage: how many sessions backed the number and why the others have no
//!    data, with a stable reason.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use meltemi_proto::{
    AnalyticsUsageParams, AnalyticsUsageResult, SessionEvent, SessionEventKind, UsageActivity,
    UsageCell, UsageCoverage, UsageDisclosure, UsageGranularity, UsageTokens, UsageTotals,
    UsageUnattributed, UsageUnreportedKind, UsageUnreportedReason, error_codes,
};

use crate::rpc::RpcError;
use crate::session_index::{self, SessionRecord};

/// The disclosure the daemon always returns with the numbers (design D6): keys
/// only, so each surface renders its own ES/EN text.
pub const DISCLOSURE: &[UsageDisclosure] = &[
    UsageDisclosure::ActivityFromLocalRecords,
    UsageDisclosure::TokensOnlyWhenOfficialOutputReports,
    UsageDisclosure::NoQuotaBalanceOrBilling,
    UsageDisclosure::NothingIsEstimated,
    UsageDisclosure::NothingLeavesThisMachine,
];

/// The dimension key of a cell: project × agent × profile × period (design D2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CellKey {
    project_key: String,
    project_root: String,
    agent: String,
    profile: Option<String>,
    period: String,
}

/// The accumulator behind one cell.
#[derive(Debug, Default)]
struct Bucket {
    activity: UsageActivity,
    tokens: UsageTokens,
    agent_id: Option<String>,
    level: Option<u8>,
    measured_sessions: u32,
    unreported: BTreeMap<UsageUnreportedKind, u32>,
}

/// The facts one session log contributes.
#[derive(Debug, Default)]
struct SessionFacts {
    prompts: u32,
    turns: BTreeMap<String, u32>,
    permissions_requested: u32,
    permissions_approved: u32,
    permissions_denied: u32,
    permissions_expired: u32,
    human_edits: u32,
    commits: u32,
    checkpoints: u32,
    errors: u32,
    /// The effective binary, when the log recorded a resolution.
    binary: Option<String>,
    level: Option<u8>,
    /// Counters the official output reported, summed over the session's events.
    tokens: UsageTokens,
    measured: bool,
}

/// Adds `add` into `slot`, keeping absence absent: a counter nobody reported
/// stays `None` instead of becoming 0.
fn add_measured(slot: &mut Option<u64>, add: Option<u64>) {
    if let Some(value) = add {
        *slot = Some(slot.unwrap_or(0) + value);
    }
}

fn merge_tokens(into: &mut UsageTokens, from: &UsageTokens) {
    add_measured(&mut into.input, from.input);
    add_measured(&mut into.output, from.output);
    add_measured(&mut into.cached_input, from.cached_input);
    add_measured(&mut into.reasoning, from.reasoning);
    add_measured(&mut into.total, from.total);
}

fn merge_activity(into: &mut UsageActivity, from: &UsageActivity) {
    into.sessions += from.sessions;
    into.sessions_closed += from.sessions_closed;
    into.sessions_open += from.sessions_open;
    into.active_seconds += from.active_seconds;
    into.prompts += from.prompts;
    for (reason, count) in &from.turns_by_stop_reason {
        *into.turns_by_stop_reason.entry(reason.clone()).or_default() += count;
    }
    into.permissions_requested += from.permissions_requested;
    into.permissions_approved += from.permissions_approved;
    into.permissions_denied += from.permissions_denied;
    into.permissions_expired += from.permissions_expired;
    into.human_edits += from.human_edits;
    into.commits += from.commits;
    into.checkpoints += from.checkpoints;
    into.errors += from.errors;
}

/// The period label of a timestamp at the requested grain. RFC 3339 timestamps
/// sort and slice lexically, which is all the labels need.
fn period_of(ts: &str, grain: UsageGranularity) -> String {
    match grain {
        UsageGranularity::Total => "total".to_string(),
        UsageGranularity::Day => ts.get(0..10).unwrap_or(ts).to_string(),
        UsageGranularity::Month => ts.get(0..7).unwrap_or(ts).to_string(),
        UsageGranularity::Week => iso_week_label(ts),
    }
}

/// `YYYY-Www` for a timestamp, computed from the civil date so a week label
/// never depends on a date library.
fn iso_week_label(ts: &str) -> String {
    let Some((year, month, day)) = civil_date(ts) else {
        return ts.to_string();
    };
    // Days since 1970-01-01 (proleptic Gregorian), then ISO-8601 week number.
    let days = days_from_civil(year, month, day);
    // 1970-01-01 was a Thursday: shift so Monday is 0.
    let weekday = (days + 3).rem_euclid(7); // 0 = Monday
    let thursday = days - weekday + 3; // the Thursday of this ISO week
    let (iso_year, _, _) = civil_from_days(thursday);
    let jan1 = days_from_civil(iso_year, 1, 1);
    let week = (thursday - jan1) / 7 + 1;
    format!("{iso_year:04}-W{week:02}")
}

fn civil_date(ts: &str) -> Option<(i64, u32, u32)> {
    let date = ts.get(0..10)?;
    let mut parts = date.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

/// Howard Hinnant's `days_from_civil`: civil date → days since 1970-01-01.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Seconds between two RFC 3339 timestamps, or 0 when either is unparseable.
fn seconds_between(start: &str, end: &str) -> u64 {
    let secs = |ts: &str| -> Option<i64> {
        let (year, month, day) = civil_date(ts)?;
        let time = ts.get(11..19)?;
        let mut parts = time.split(':');
        let h: i64 = parts.next()?.parse().ok()?;
        let min: i64 = parts.next()?.parse().ok()?;
        let s: i64 = parts.next()?.parse().ok()?;
        Some(days_from_civil(year, month, day) * 86_400 + h * 3600 + min * 60 + s)
    };
    match (secs(start), secs(end)) {
        (Some(a), Some(b)) if b >= a => (b - a) as u64,
        // A clock that went backwards is not turned into a negative duration.
        _ => 0,
    }
}

/// The leaf of a path, without a Windows executable extension: the effective
/// binary as a dimension value.
fn binary_label(program: &str) -> String {
    let leaf = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .to_string();
    for ext in [".exe", ".cmd", ".bat"] {
        if let Some(stripped) = leaf.strip_suffix(ext) {
            return stripped.to_string();
        }
    }
    leaf
}

/// Folds one session's JSONL log into its facts. Unparseable lines are skipped
/// (the same tolerance the index fold has); a missing log yields no facts, not
/// an error.
fn facts_of_log(path: &Path) -> SessionFacts {
    let mut facts = SessionFacts::default();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return facts;
    };
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(event) = serde_json::from_str::<SessionEvent>(line) else {
            continue;
        };
        match event.kind {
            SessionEventKind::PromptSent { .. } => facts.prompts += 1,
            SessionEventKind::TurnCompleted { stop_reason } => {
                let key = serde_json::to_value(stop_reason)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| "unknown".into());
                *facts.turns.entry(key).or_default() += 1;
            }
            SessionEventKind::PermissionRequested { .. } => facts.permissions_requested += 1,
            SessionEventKind::PermissionDecided {
                denied, decided_by, ..
            } => {
                // The recorded fact first: the outcome SHAPE cannot tell an
                // approval from a denial, because selecting a reject option
                // looks exactly like selecting an allow one.
                // Logs written before the field: only the decider that always
                // denies can be read with certainty.
                let denial = denied.unwrap_or(matches!(
                    decided_by,
                    meltemi_proto::PermissionDecidedBy::DefaultDeny
                        | meltemi_proto::PermissionDecidedBy::Timeout
                ));
                if denial {
                    facts.permissions_denied += 1;
                } else {
                    facts.permissions_approved += 1;
                }
                // A timeout is a denial AND an expiry: it is counted in both,
                // which is what the metric set declares.
                if matches!(decided_by, meltemi_proto::PermissionDecidedBy::Timeout) {
                    facts.permissions_expired += 1;
                }
            }
            SessionEventKind::HumanEdit { .. } => facts.human_edits += 1,
            SessionEventKind::TaskCommitted { .. } => facts.commits += 1,
            SessionEventKind::CheckpointCreated { .. } => facts.checkpoints += 1,
            SessionEventKind::Error { .. } => facts.errors += 1,
            SessionEventKind::AgentResolved { binary, level, .. } => {
                facts.binary = Some(binary_label(&binary));
                facts.level = Some(level);
            }
            SessionEventKind::UsageReported {
                input_tokens,
                output_tokens,
                cached_input_tokens,
                reasoning_tokens,
                total_tokens,
                ..
            } => {
                // Only what the official output declared, and only summed with
                // itself: absence never becomes zero.
                facts.measured = true;
                merge_tokens(
                    &mut facts.tokens,
                    &UsageTokens {
                        input: input_tokens,
                        output: output_tokens,
                        cached_input: cached_input_tokens,
                        reasoning: reasoning_tokens,
                        total: total_tokens,
                    },
                );
            }
            _ => {}
        }
    }
    facts
}

/// Why a session with no `usage_reported` event has no tokens (design D4).
fn unreported_kind(level: u8) -> UsageUnreportedKind {
    match level {
        // Level 4 produces artifacts; no process runs, so nothing can report.
        4 => UsageUnreportedKind::LevelRunsNoProcess,
        // Level 3 runs a process whose structured output declared no counters.
        3 => UsageUnreportedKind::StreamDeclaredNoCounters,
        // Levels 1 and 2 are piloted over ACP, which does not carry usage.
        _ => UsageUnreportedKind::ProtocolCarriesNoUsage,
    }
}

/// Whether a timestamp falls inside `[since, until)`. RFC 3339 compares
/// lexically for the same offset, which is what the daemon writes (UTC `Z`).
fn in_range(ts: &str, since: Option<&str>, until: Option<&str>) -> bool {
    if let Some(since) = since
        && ts < since
    {
        return false;
    }
    if let Some(until) = until
        && ts >= until
    {
        return false;
    }
    true
}

/// Human edits recorded with no session active on the tree — the real
/// unattributable fact (design D2). Read from the project's own edit log, and
/// only when its root is reachable.
fn unattributed_edits(project_root: &str, since: Option<&str>, until: Option<&str>) -> Vec<String> {
    let path = PathBuf::from(project_root)
        .join(".meltemi")
        .join("edits")
        .join("events.jsonl");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut stamps = Vec::new();
    for line in contents.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        // Only the edits nobody can attribute: those with no session.
        if event.get("sessionId").and_then(|v| v.as_str()).is_some() {
            continue;
        }
        let Some(ts) = event.get("ts").and_then(|v| v.as_str()) else {
            continue;
        };
        if in_range(ts, since, until) {
            stamps.push(ts.to_string());
        }
    }
    stamps
}

/// Validates the parameters, refusing rather than silently defaulting (D5).
fn validate(params: &AnalyticsUsageParams) -> Result<(), RpcError> {
    let invalid = |detail: String, remedy: &str| {
        RpcError::application(
            error_codes::USAGE_QUERY_INVALID,
            "invalid usage query",
            "invalid_usage_query",
            detail,
            Some(remedy.to_string()),
        )
    };
    if let (Some(since), Some(until)) = (&params.since, &params.until)
        && since >= until
    {
        return Err(invalid(
            format!("the range is inverted: since `{since}` is not before until `{until}`"),
            "Swap the bounds: `since` is inclusive and `until` exclusive.",
        ));
    }
    for (name, value) in [("since", &params.since), ("until", &params.until)] {
        if let Some(ts) = value
            && civil_date(ts).is_none()
        {
            return Err(invalid(
                format!("`{name}` is not an RFC 3339 timestamp: `{ts}`"),
                "Use a timestamp like 2026-07-01T00:00:00Z.",
            ));
        }
    }
    if params.limit == Some(0) {
        return Err(invalid(
            "`limit` must be at least 1".into(),
            "Omit the limit or ask for at least one cell.",
        ));
    }
    Ok(())
}

/// Aggregates usage over the local records. Pure over the data directory: it
/// opens no socket and writes nothing (design D6).
pub fn aggregate(
    data_dir: &Path,
    params: &AnalyticsUsageParams,
) -> Result<AnalyticsUsageResult, RpcError> {
    validate(params)?;
    let grain = params.granularity.unwrap_or_default();
    let since = params.since.as_deref();
    let until = params.until.as_deref();

    let keys: Vec<String> = match &params.project_root {
        Some(root) => vec![crate::paths::project_key(Path::new(root))],
        None => session_index::all_project_keys(data_dir),
    };

    let mut buckets: BTreeMap<CellKey, Bucket> = BTreeMap::new();
    let mut unattributed: BTreeMap<(String, String, String), u32> = BTreeMap::new();

    for key in keys {
        let records = session_index::records_for_project(data_dir, &key);
        let dir = session_index::sessions_dir(data_dir, &key);
        // Any project root the records mention (they all share the tree, but a
        // worktree session records its own root).
        let mut project_root = params.project_root.clone().unwrap_or_default();

        for record in &records {
            // The index is the prefilter: a session outside the range never
            // has its log opened.
            if !in_range(&record.started_at, since, until) {
                continue;
            }
            if project_root.is_empty() {
                project_root = record.project_root.clone();
            }
            let facts = facts_of_log(&dir.join(format!("{}.jsonl", record.session_id)));
            let agent = facts.binary.clone().unwrap_or_else(|| {
                binary_label(record.agent_command.first().map_or("", String::as_str))
            });
            if let Some(filter) = &params.agent
                && *filter != agent
            {
                continue;
            }
            if let Some(filter) = &params.profile
                && record.profile.as_deref() != Some(filter.as_str())
            {
                continue;
            }

            let cell_key = CellKey {
                project_key: key.clone(),
                project_root: project_root.clone(),
                agent,
                profile: record.profile.clone(),
                period: period_of(&record.started_at, grain),
            };
            let bucket = buckets.entry(cell_key).or_default();
            accumulate(bucket, record, &facts);
        }

        // The unattributable bucket, per project and period.
        if !project_root.is_empty() {
            for ts in unattributed_edits(&project_root, since, until) {
                let entry = (key.clone(), project_root.clone(), period_of(&ts, grain));
                *unattributed.entry(entry).or_default() += 1;
            }
        }
    }

    let mut cells: Vec<UsageCell> = buckets
        .into_iter()
        .map(|(key, bucket)| UsageCell {
            project_key: key.project_key,
            project_root: key.project_root,
            agent: key.agent,
            agent_id: bucket.agent_id,
            profile: key.profile,
            level: bucket.level,
            period: key.period,
            activity: bucket.activity,
            tokens: (!bucket.tokens.is_empty()).then_some(bucket.tokens),
            coverage: UsageCoverage {
                measured_sessions: bucket.measured_sessions,
                unreported_sessions: bucket.unreported.values().sum(),
                reasons: bucket
                    .unreported
                    .into_iter()
                    .map(|(kind, sessions)| UsageUnreportedReason { kind, sessions })
                    .collect(),
            },
        })
        .collect();
    // Most recent period first, then project and agent: a stable order.
    cells.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.project_root.cmp(&b.project_root))
            .then_with(|| a.agent.cmp(&b.agent))
            .then_with(|| a.profile.cmp(&b.profile))
    });

    let truncated = params
        .limit
        .is_some_and(|limit| cells.len() > limit as usize);
    if let Some(limit) = params.limit {
        cells.truncate(limit as usize);
    }

    // Totals sum exactly the cells returned, so the figure and the list agree.
    let mut totals = UsageTotals {
        cells: cells.len() as u32,
        ..UsageTotals::default()
    };
    let mut reasons: BTreeMap<UsageUnreportedKind, u32> = BTreeMap::new();
    let mut measured_tokens = UsageTokens::default();
    for cell in &cells {
        merge_activity(&mut totals.activity, &cell.activity);
        if let Some(tokens) = &cell.tokens {
            merge_tokens(&mut measured_tokens, tokens);
        }
        totals.coverage.measured_sessions += cell.coverage.measured_sessions;
        totals.coverage.unreported_sessions += cell.coverage.unreported_sessions;
        for reason in &cell.coverage.reasons {
            *reasons.entry(reason.kind).or_default() += reason.sessions;
        }
    }
    totals.coverage.reasons = reasons
        .into_iter()
        .map(|(kind, sessions)| UsageUnreportedReason { kind, sessions })
        .collect();
    totals.tokens = (!measured_tokens.is_empty()).then_some(measured_tokens);

    Ok(AnalyticsUsageResult {
        cells,
        unattributed: unattributed
            .into_iter()
            .map(
                |((project_key, project_root, period), human_edits)| UsageUnattributed {
                    project_key,
                    project_root,
                    period,
                    human_edits,
                },
            )
            .collect(),
        totals,
        truncated,
        disclosure: DISCLOSURE.to_vec(),
    })
}

/// Folds one session's record and facts into its cell.
fn accumulate(bucket: &mut Bucket, record: &SessionRecord, facts: &SessionFacts) {
    let activity = &mut bucket.activity;
    activity.sessions += 1;
    match &record.ended_at {
        // Duration only for closed sessions: an open one is never extrapolated.
        Some(ended_at) => {
            activity.sessions_closed += 1;
            activity.active_seconds += seconds_between(&record.started_at, ended_at);
        }
        None => activity.sessions_open += 1,
    }
    activity.prompts += facts.prompts;
    for (reason, count) in &facts.turns {
        *activity
            .turns_by_stop_reason
            .entry(reason.clone())
            .or_default() += count;
    }
    activity.permissions_requested += facts.permissions_requested;
    activity.permissions_approved += facts.permissions_approved;
    activity.permissions_denied += facts.permissions_denied;
    activity.permissions_expired += facts.permissions_expired;
    activity.human_edits += facts.human_edits;
    activity.commits += facts.commits;
    activity.checkpoints += facts.checkpoints;
    activity.errors += facts.errors;

    if bucket.agent_id.is_none() {
        bucket.agent_id = record.agent_id.clone();
    }
    if bucket.level.is_none() {
        bucket.level = facts.level.or(Some(record.level));
    }

    if facts.measured {
        bucket.measured_sessions += 1;
        merge_tokens(&mut bucket.tokens, &facts.tokens);
    } else {
        let level = facts.level.unwrap_or(record.level);
        *bucket.unreported.entry(unreported_kind(level)).or_default() += 1;
    }
}

/// Handles `analytics/usage`.
pub fn handle_analytics_usage(
    params: serde_json::Value,
    state: &std::sync::Arc<crate::server::DaemonState>,
) -> Result<serde_json::Value, RpcError> {
    let params: AnalyticsUsageParams = if params.is_null() {
        AnalyticsUsageParams::default()
    } else {
        serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("analytics/usage: {e}")))?
    };
    let result = aggregate(&state.data_dir, &params)?;
    serde_json::to_value(result).map_err(RpcError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use meltemi_proto::TurnStatus;

    /// The remedy the refusal carries in its structured error data.
    fn remedy_of(error: &RpcError) -> Option<String> {
        error
            .data
            .as_ref()
            .and_then(|data| data.get("remedy"))
            .and_then(|remedy| remedy.as_str())
            .map(String::from)
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mel-analytics-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn record(
        id: &str,
        started: &str,
        ended: Option<&str>,
        profile: Option<&str>,
    ) -> SessionRecord {
        SessionRecord {
            session_id: id.into(),
            agent_command: vec!["claude.exe".into()],
            project_root: "/repo".into(),
            level: 1,
            started_at: started.into(),
            ended_at: ended.map(String::from),
            final_status: Some(TurnStatus::Completed),
            agent_session_id: None,
            supports_load: false,
            resumed_from: None,
            agent_id: Some("claude-code".into()),
            profile: profile.map(String::from),
            source: None,
            title: None,
        }
    }

    /// Writes one session log with the given event lines (payload JSON).
    fn log(dir: &Path, id: &str, lines: &[(&str, &str, &str)]) {
        std::fs::create_dir_all(dir).unwrap();
        let body: String = lines
            .iter()
            .map(|(kind, ts, payload)| {
                format!(r#"{{"v":1,"ts":"{ts}","type":"{kind}","payload":{payload}}}"#) + "\n"
            })
            .collect();
        std::fs::write(dir.join(format!("{id}.jsonl")), body).unwrap();
    }

    fn write(data: &Path, key: &str, records: &[SessionRecord]) {
        for record in records {
            session_index::append(data, key, record).unwrap();
        }
    }

    #[test]
    fn cells_split_by_agent_and_profile_within_a_period() {
        // Scenario: Celdas por proyecto, agente, perfil y período
        let data = temp("cells");
        let key = "k";
        write(
            &data,
            key,
            &[
                record(
                    "s1",
                    "2026-07-10T10:00:00Z",
                    Some("2026-07-10T10:10:00Z"),
                    Some("work"),
                ),
                record(
                    "s2",
                    "2026-07-11T10:00:00Z",
                    Some("2026-07-11T10:05:00Z"),
                    Some("personal"),
                ),
            ],
        );
        let dir = session_index::sessions_dir(&data, key);
        log(
            &dir,
            "s1",
            &[("prompt_sent", "2026-07-10T10:00:01Z", r#"{"text":"hi"}"#)],
        );
        log(
            &dir,
            "s2",
            &[("prompt_sent", "2026-07-11T10:00:01Z", r#"{"text":"ho"}"#)],
        );

        let result = aggregate(
            &data,
            &AnalyticsUsageParams {
                granularity: Some(UsageGranularity::Month),
                ..Default::default()
            },
        )
        .expect("aggregate");
        assert_eq!(
            result.cells.len(),
            2,
            "one cell per subscription: {result:#?}"
        );
        assert!(result.cells.iter().all(|c| c.period == "2026-07"));
        assert!(result.cells.iter().all(|c| c.activity.prompts == 1));
        assert_eq!(result.totals.activity.prompts, 2);
        assert_eq!(result.totals.cells, 2);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn duration_counts_only_closed_sessions() {
        // Scenario: Duración medida solo en sesiones cerradas
        let data = temp("duration");
        write(
            &data,
            "k",
            &[
                record(
                    "closed",
                    "2026-07-10T10:00:00Z",
                    Some("2026-07-10T10:02:00Z"),
                    None,
                ),
                record("open", "2026-07-10T11:00:00Z", None, None),
            ],
        );
        let result = aggregate(&data, &AnalyticsUsageParams::default()).expect("aggregate");
        let cell = &result.cells[0];
        assert_eq!(cell.activity.sessions, 2);
        assert_eq!(cell.activity.sessions_closed, 1);
        assert_eq!(cell.activity.sessions_open, 1);
        assert_eq!(cell.activity.active_seconds, 120, "only the closed session");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn measured_counters_are_summed_and_absences_stay_absent() {
        // Scenario: Contadores del stream oficial agregados
        // Scenario: Contador no declarado permanece ausente
        let data = temp("tokens");
        write(
            &data,
            "k",
            &[record(
                "s1",
                "2026-07-10T10:00:00Z",
                Some("2026-07-10T10:01:00Z"),
                None,
            )],
        );
        let dir = session_index::sessions_dir(&data, "k");
        log(
            &dir,
            "s1",
            &[
                (
                    "usage_reported",
                    "2026-07-10T10:00:30Z",
                    r#"{"source":"mock-headless","inputTokens":100,"outputTokens":20}"#,
                ),
                (
                    "usage_reported",
                    "2026-07-10T10:00:40Z",
                    r#"{"source":"mock-headless","inputTokens":5}"#,
                ),
            ],
        );
        let result = aggregate(&data, &AnalyticsUsageParams::default()).expect("aggregate");
        let tokens = result.cells[0].tokens.as_ref().expect("measured tokens");
        assert_eq!(tokens.input, Some(105), "measured counters are summed");
        assert_eq!(tokens.output, Some(20));
        assert_eq!(
            tokens.cached_input, None,
            "a counter the output never declared stays absent, not zero"
        );
        assert_eq!(result.cells[0].coverage.measured_sessions, 1);
        assert_eq!(result.cells[0].coverage.unreported_sessions, 0);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_acp_session_is_unreported_by_the_protocol_and_keeps_its_activity() {
        // Scenario: Sesión ACP marcada como no reportada por el protocolo
        // Scenario: Medido y no reportado nunca comparten cifra
        let data = temp("acp");
        write(
            &data,
            "k",
            &[
                record(
                    "acp",
                    "2026-07-10T10:00:00Z",
                    Some("2026-07-10T10:01:00Z"),
                    None,
                ),
                record(
                    "measured",
                    "2026-07-10T12:00:00Z",
                    Some("2026-07-10T12:01:00Z"),
                    None,
                ),
            ],
        );
        let dir = session_index::sessions_dir(&data, "k");
        log(
            &dir,
            "acp",
            &[("prompt_sent", "2026-07-10T10:00:01Z", r#"{"text":"hi"}"#)],
        );
        log(
            &dir,
            "measured",
            &[(
                "usage_reported",
                "2026-07-10T12:00:30Z",
                r#"{"source":"mock-headless","inputTokens":7}"#,
            )],
        );
        let result = aggregate(&data, &AnalyticsUsageParams::default()).expect("aggregate");
        let cell = &result.cells[0];
        assert_eq!(cell.activity.sessions, 2);
        assert_eq!(
            cell.activity.prompts, 1,
            "the ACP session keeps its activity"
        );
        // The measured figure covers only the measured session.
        assert_eq!(cell.tokens.as_ref().unwrap().input, Some(7));
        assert_eq!(cell.coverage.measured_sessions, 1);
        assert_eq!(cell.coverage.unreported_sessions, 1);
        assert_eq!(
            cell.coverage.reasons,
            vec![UsageUnreportedReason {
                kind: UsageUnreportedKind::ProtocolCarriesNoUsage,
                sessions: 1
            }]
        );
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_human_edit_without_a_session_lands_in_the_unattributed_bucket() {
        // Scenario: Hecho sin atribución en cubeta explícita
        let data = temp("unattributed");
        let project = temp("repo-unattributed");
        let edits = project.join(".meltemi").join("edits");
        std::fs::create_dir_all(&edits).unwrap();
        std::fs::write(
            edits.join("events.jsonl"),
            concat!(
                r#"{"ts":"2026-07-10T09:00:00Z","file":"src/a.rs"}"#,
                "\n",
                r#"{"ts":"2026-07-10T09:05:00Z","file":"src/b.rs","sessionId":"s1"}"#,
                "\n",
            ),
        )
        .unwrap();
        let key = crate::paths::project_key(&project);
        let mut record = record(
            "s1",
            "2026-07-10T09:00:00Z",
            Some("2026-07-10T09:10:00Z"),
            None,
        );
        record.project_root = project.display().to_string();
        write(&data, &key, &[record]);

        let result = aggregate(
            &data,
            &AnalyticsUsageParams {
                project_root: Some(project.display().to_string()),
                ..Default::default()
            },
        )
        .expect("aggregate");
        assert_eq!(result.unattributed.len(), 1, "got: {result:#?}");
        assert_eq!(
            result.unattributed[0].human_edits, 1,
            "only the edit with no session is unattributed"
        );
        // And it is not folded into the attributed cell.
        assert_eq!(result.cells[0].activity.human_edits, 0);
        let _ = std::fs::remove_dir_all(&data);
        let _ = std::fs::remove_dir_all(&project);
    }

    #[test]
    fn aggregation_works_from_the_logs_when_the_index_is_missing() {
        // Scenario: Agregación sin índice de sesiones
        let data = temp("no-index");
        let dir = session_index::sessions_dir(&data, "k");
        log(
            &dir,
            "s1",
            &[
                (
                    "session_started",
                    "2026-07-10T10:00:00Z",
                    r#"{"sessionId":"s1","agentCommand":["claude"],"projectRoot":"/repo"}"#,
                ),
                ("prompt_sent", "2026-07-10T10:00:01Z", r#"{"text":"hi"}"#),
                (
                    "session_ended",
                    "2026-07-10T10:01:00Z",
                    r#"{"reason":"completed"}"#,
                ),
            ],
        );
        let result = aggregate(&data, &AnalyticsUsageParams::default()).expect("aggregate");
        assert_eq!(result.cells.len(), 1, "the log-only session is accounted");
        assert_eq!(result.cells[0].activity.prompts, 1);
        assert_eq!(result.cells[0].agent, "claude");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_project_without_records_answers_empty_without_zero_rows() {
        // Scenario: Proyecto sin registros responde vacío honesto
        let data = temp("empty");
        let result = aggregate(
            &data,
            &AnalyticsUsageParams {
                project_root: Some("/nowhere".into()),
                ..Default::default()
            },
        )
        .expect("aggregate");
        assert!(result.cells.is_empty(), "no fabricated zero rows");
        assert_eq!(result.totals.cells, 0);
        assert!(result.totals.tokens.is_none(), "no zeroed token total");
        assert!(
            !result.disclosure.is_empty(),
            "the disclosure always travels"
        );
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_invalid_query_refuses_instead_of_defaulting() {
        // Scenario: Parámetro inválido rehúsa
        let data = temp("invalid");
        let inverted = aggregate(
            &data,
            &AnalyticsUsageParams {
                since: Some("2026-07-10T00:00:00Z".into()),
                until: Some("2026-07-01T00:00:00Z".into()),
                ..Default::default()
            },
        )
        .expect_err("an inverted range refuses");
        assert_eq!(inverted.code, error_codes::USAGE_QUERY_INVALID);
        assert!(
            remedy_of(&inverted).is_some(),
            "the refusal carries a remedy"
        );

        let bad_stamp = aggregate(
            &data,
            &AnalyticsUsageParams {
                since: Some("last tuesday".into()),
                ..Default::default()
            },
        )
        .expect_err("an unparseable bound refuses");
        assert!(remedy_of(&bad_stamp).is_some());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn the_range_prefilters_and_the_limit_declares_truncation() {
        let data = temp("range");
        write(
            &data,
            "k",
            &[
                record(
                    "old",
                    "2026-06-01T10:00:00Z",
                    Some("2026-06-01T10:01:00Z"),
                    Some("a"),
                ),
                record(
                    "new",
                    "2026-07-10T10:00:00Z",
                    Some("2026-07-10T10:01:00Z"),
                    Some("b"),
                ),
                record(
                    "newer",
                    "2026-07-20T10:00:00Z",
                    Some("2026-07-20T10:01:00Z"),
                    Some("c"),
                ),
            ],
        );
        let ranged = aggregate(
            &data,
            &AnalyticsUsageParams {
                since: Some("2026-07-01T00:00:00Z".into()),
                granularity: Some(UsageGranularity::Day),
                ..Default::default()
            },
        )
        .expect("aggregate");
        assert_eq!(ranged.cells.len(), 2, "June is outside the range");
        assert_eq!(ranged.cells[0].period, "2026-07-20", "most recent first");

        let limited = aggregate(
            &data,
            &AnalyticsUsageParams {
                granularity: Some(UsageGranularity::Day),
                limit: Some(1),
                ..Default::default()
            },
        )
        .expect("aggregate");
        assert_eq!(limited.cells.len(), 1);
        assert!(limited.truncated, "truncation is declared, never silent");
        assert_eq!(limited.totals.cells, 1, "totals sum what was returned");
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn period_labels_cover_every_grain() {
        assert_eq!(
            period_of("2026-07-24T10:00:00Z", UsageGranularity::Day),
            "2026-07-24"
        );
        assert_eq!(
            period_of("2026-07-24T10:00:00Z", UsageGranularity::Month),
            "2026-07"
        );
        assert_eq!(
            period_of("2026-07-24T10:00:00Z", UsageGranularity::Total),
            "total"
        );
        // 2026-07-24 is a Friday, in ISO week 30.
        assert_eq!(
            period_of("2026-07-24T10:00:00Z", UsageGranularity::Week),
            "2026-W30"
        );
        // A Thursday-anchored year boundary: 2027-01-01 falls in week 53 of 2026.
        assert_eq!(
            period_of("2027-01-01T00:00:00Z", UsageGranularity::Week),
            "2026-W53"
        );
    }

    #[test]
    fn the_disclosure_keys_are_stable_and_carry_no_prose() {
        // Scenario: Declaración estructurada, texto de la superficie
        let json = serde_json::to_value(DISCLOSURE).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                "activity-from-local-records",
                "tokens-only-when-official-output-reports",
                "no-quota-balance-or-billing",
                "nothing-is-estimated",
                "nothing-leaves-this-machine"
            ])
        );
    }
}
