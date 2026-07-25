// SPDX-License-Identifier: Apache-2.0
// Node's built-in test runner over the TypeScript source (native type
// stripping): no test framework dependency (constitution §10).

import assert from "node:assert/strict";
import test from "node:test";

import { fuzzyMatch, fuzzyRank, highlightRuns } from "../src/lib/fuzzy.ts";

const METHODS = [
  "status",
  "shutdown",
  "propose",
  "fleet/list",
  "session/list",
  "session/direct",
  "worktree/apply-edit",
  "worktree/assign",
  "sdd/validate",
  "sdd/verify-mark",
  "checkpoint/revert",
  "commit/task",
];

const rank = (needle: string) =>
  fuzzyRank(needle, METHODS, (method) => method).map((row) => row.item);

// Scenario: Subsecuencia encuentra el método
test("a segment-initial subsequence reaches its method first", () => {
  assert.equal(rank("wapp")[0], "worktree/apply-edit");
  assert.equal(rank("sdv")[0], "sdd/validate");
  assert.equal(rank("ctask")[0], "commit/task");
  assert.equal(rank("sesdir")[0], "session/direct");
});

test("a non-subsequence matches nothing", () => {
  assert.equal(fuzzyMatch("zzz", "worktree/apply-edit"), null);
  assert.deepEqual(rank("qqq"), []);
});

test("an empty query keeps every candidate", () => {
  assert.equal(rank("").length, METHODS.length);
});

test("matching is case-insensitive and reports positions", () => {
  const match = fuzzyMatch("STA", "status");
  assert.ok(match);
  assert.deepEqual(match.positions, [0, 1, 2]);
});

test("a whole-word match outranks a buried one", () => {
  const ranked = rank("list");
  assert.equal(ranked[0], "fleet/list");
  assert.ok(ranked.includes("session/list"));
});

test("the frecency bonus can lift a match above a better textual one", () => {
  const withoutBonus = fuzzyRank("list", METHODS, (method) => method)[0].item;
  const withBonus = fuzzyRank(
    "list",
    METHODS,
    (method) => method,
    (method) => (method === "session/list" ? 100 : 0),
  )[0].item;
  assert.equal(withoutBonus, "fleet/list");
  assert.equal(withBonus, "session/list");
});

test("highlight runs reconstruct the original text", () => {
  const match = fuzzyMatch("wapp", "worktree/apply-edit");
  assert.ok(match);
  const runs = highlightRuns("worktree/apply-edit", match.positions);
  assert.equal(runs.map((run) => run.text).join(""), "worktree/apply-edit");
  assert.ok(runs.some((run) => run.hit));
});
