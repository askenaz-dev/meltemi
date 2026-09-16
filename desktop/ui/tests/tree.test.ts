// SPDX-License-Identifier: Apache-2.0
// Project → Sessions aggregation (multiproyecto-suscripciones 8.5). Pure
// functions, so the tree is testable without a window: node --test with Node's
// native type stripping, no test framework dependency.

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  BUCKET_ORDER,
  STOPPED_SHOWN,
  agentLabelOf,
  bucketSessions,
  groupSessions,
  projectName,
} from "../src/lib/tree.ts";
import type { ProjectInfo, SessionInfo } from "../src/lib/stores.ts";

const TS = "2026-07-24T10:00:00Z";

function project(root: string, extra: Partial<ProjectInfo> = {}): ProjectInfo {
  return {
    projectKey: root.toLowerCase(),
    root,
    exists: true,
    firstSeenAt: TS,
    lastSeenAt: TS,
    sessionsTotal: 0,
    resumableSessions: 0,
    ...extra,
  };
}

function session(id: string, root: string, extra: Partial<SessionInfo> = {}): SessionInfo {
  return {
    sessionId: id,
    agentCommand: ["claude.exe", "--acp"],
    projectRoot: root,
    state: "ended",
    level: 1,
    startedAt: TS,
    resumable: false,
    ...extra,
  };
}

test("sessions group under their project, most recent first", () => {
  // Scenario: Árbol de proyectos y sesiones.
  const tree = groupSessions(
    [project("C:\\repos\\alpha"), project("C:\\repos\\beta")],
    [
      session("s1", "C:\\repos\\alpha", { startedAt: "2026-07-24T09:00:00Z" }),
      session("s2", "C:\\repos\\beta"),
      session("s3", "C:\\repos\\alpha", { startedAt: "2026-07-24T11:00:00Z" }),
    ],
  );
  assert.deepEqual(
    tree.map((group) => [group.name, group.sessions.length]),
    [
      ["alpha", 2],
      ["beta", 1],
    ],
  );
  assert.deepEqual(tree[0].sessions.map((s) => s.sessionId), ["s3", "s1"]);
});

test("two subscriptions of the same agent stay distinguishable", () => {
  // Scenario: Dos suscripciones del mismo agente.
  const tree = groupSessions(
    [project("/repos/alpha")],
    [
      session("s1", "/repos/alpha", { agentId: "claude-code", profile: "work" }),
      session("s2", "/repos/alpha", { agentId: "claude-code", profile: "personal" }),
    ],
  );
  const [group] = tree;
  assert.equal(group.sessions.length, 2);
  // Same agent identity...
  assert.deepEqual(group.sessions.map(agentLabelOf), ["claude-code", "claude-code"]);
  // ...different subscription.
  assert.deepEqual(new Set(group.sessions.map((s) => s.profile)), new Set(["work", "personal"]));
});

test("a worktree session belongs to its repository, not to a phantom project", () => {
  const tree = groupSessions(
    [project("/repos/alpha")],
    [session("s1", "/repos/alpha/.meltemi/worktrees/add-thing/1-1-claude")],
  );
  assert.equal(tree.length, 1, "no phantom node for the worktree");
  assert.equal(tree[0].sessions.length, 1);
});

test("the longest matching project wins over a parent that also matches", () => {
  const tree = groupSessions(
    [project("/repos"), project("/repos/alpha")],
    [session("s1", "/repos/alpha/sub")],
  );
  const byName = new Map(tree.map((group) => [group.name, group.sessions.length]));
  assert.equal(byName.get("alpha"), 1);
  assert.equal(byName.get("repos"), 0);
});

test("a session outside the registry is surfaced, never dropped", () => {
  const tree = groupSessions([project("/repos/alpha")], [session("s1", "/elsewhere/gamma")]);
  assert.equal(tree.length, 2);
  const inferred = tree.find((group) => group.inferred);
  assert.equal(inferred?.name, "gamma");
  // Registry nodes come first: the registry is the spine of the tree.
  assert.equal(tree[0].name, "alpha");
});

test("a project whose root vanished keeps its node and its mark", () => {
  const tree = groupSessions([project("/repos/moved", { exists: false })], []);
  assert.equal(tree.length, 1);
  assert.equal(tree[0].exists, false);
});

test("live sessions are counted per project", () => {
  const tree = groupSessions(
    [project("/repos/alpha")],
    [
      session("s1", "/repos/alpha", { state: "active" }),
      session("s2", "/repos/alpha", { state: "waiting_permission" }),
      session("s3", "/repos/alpha", { state: "ended" }),
    ],
  );
  assert.equal(tree[0].live, 2);
});

test("the agent label falls back to the binary, without its extension", () => {
  assert.equal(agentLabelOf(session("s", "/r")), "claude");
  assert.equal(agentLabelOf(session("s", "/r", { agentId: "codex-cli" })), "codex-cli");
  assert.equal(
    agentLabelOf(session("s", "/r", { agentCommand: ["/usr/local/bin/opencode"] })),
    "opencode",
  );
});

test("the project name is the last path segment on both separators", () => {
  assert.equal(projectName("C:\\repos\\alpha"), "alpha");
  assert.equal(projectName("/home/g/repos/beta/"), "beta");
});

test("four states, four buckets in signal order", () => {
  // Scenario: Cuatro estados, cuatro cubetas en orden de señal
  const buckets = bucketSessions([
    session("a", "C:\repos\alpha", { state: "ended" }),
    session("b", "C:\repos\alpha", { state: "active" }),
    session("c", "C:\repos\alpha", { state: "waiting_instruction" }),
    session("d", "C:\repos\alpha", { state: "waiting_permission" }),
  ]);
  assert.deepEqual(
    buckets.map((b) => b.id),
    ["decision", "instruction", "working", "stopped"],
    "what needs a human decision comes first, what is over comes last",
  );
  assert.deepEqual(
    buckets.map((b) => b.sessions.map((s) => s.sessionId)),
    [["d"], ["c"], ["b"], ["a"]],
  );
  // The header says the count, so every bucket carries one.
  assert.deepEqual(buckets.map((b) => b.total), [1, 1, 1, 1]);
});

test("starting joins working, and interrupted joins stopped", () => {
  // Scenario: Cuatro estados, cuatro cubetas en orden de señal. Six contract
  // states, four buckets: the two pairs share a bucket on purpose.
  const buckets = bucketSessions([
    session("a", "C:\repos\alpha", { state: "starting" }),
    session("b", "C:\repos\alpha", { state: "active" }),
    session("c", "C:\repos\alpha", { state: "interrupted" }),
    session("d", "C:\repos\alpha", { state: "ended" }),
  ]);
  assert.deepEqual(
    buckets.map((b) => [b.id, b.total]),
    [
      ["working", 2],
      ["stopped", 2],
    ],
  );
});

test("an empty bucket takes up no room", () => {
  // Scenario: Una cubeta vacía no ocupa sitio. A header reading "Working 0" is
  // noise, so the bucket is not returned at all rather than returned empty.
  const buckets = bucketSessions([
    session("a", "C:\repos\alpha", { state: "waiting_permission" }),
  ]);
  assert.deepEqual(buckets.map((b) => b.id), ["decision"]);
  assert.deepEqual(bucketSessions([]), [], "no sessions, no headers");
});

test("the stopped bucket shows only the most recent, and says how many there are", () => {
  // Scenario: Las detenidas no desbordan la barra. Handed over OLDEST first on
  // purpose: the cap promises "the most recent", and a promise that depends on
  // the caller having sorted is one that breaks the day someone calls this
  // directly.
  const many = Array.from({ length: STOPPED_SHOWN + 3 }, (_, i) =>
    session(`s${i}`, "C:\repos\alpha", {
      state: "ended",
      startedAt: `2026-07-${String(10 + i).padStart(2, "0")}T10:00:00Z`,
    }),
  );
  const [stopped] = bucketSessions(many);
  assert.equal(stopped.id, "stopped");
  assert.equal(stopped.sessions.length, STOPPED_SHOWN);
  assert.equal(stopped.total, STOPPED_SHOWN + 3, "the count is the whole bucket, not the page");
  assert.ok(stopped.capped, "so the bar can offer the way to see the rest");
  assert.equal(stopped.sessions[0].sessionId, `s${STOPPED_SHOWN + 2}`, "newest first");
});

test("a live bucket is never capped: what is asking for you is all shown", () => {
  // Scenario: Las detenidas no desbordan la barra — by its negative. Only what
  // accumulates forever is capped; sessions that are asking for something are
  // bounded by practice, not by a rule.
  const many = Array.from({ length: STOPPED_SHOWN + 4 }, (_, i) =>
    session(`w${i}`, "C:\repos\alpha", { state: "active" }),
  );
  const [working] = bucketSessions(many);
  assert.equal(working.sessions.length, many.length);
  assert.equal(working.capped, false);
});

test("the bucket order is the table, not a coincidence of the first input", () => {
  // The terminal carries its own copy of this table in Rust; the pin that keeps
  // them from diverging reads THIS constant, so it has to be the one the
  // splitter uses.
  assert.deepEqual([...BUCKET_ORDER], ["decision", "instruction", "working", "stopped"]);
});
