// SPDX-License-Identifier: Apache-2.0
// Project → Sessions aggregation (multiproyecto-suscripciones 8.5). Pure
// functions, so the tree is testable without a window: node --test with Node's
// native type stripping, no test framework dependency.

import assert from "node:assert/strict";
import { test } from "node:test";
import { agentLabelOf, groupSessions, projectName } from "../src/lib/tree.ts";
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
