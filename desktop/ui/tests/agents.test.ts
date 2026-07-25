// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { binaryName, initialsFor, toneFor } from "../src/lib/agents.ts";

// Scenario: Avatar estable por id
test("the tone of an id is stable and matches the design system", () => {
  // Same id, same tone, every call — no state, no configuration.
  assert.deepEqual(toneFor("claude-code"), toneFor("claude-code"));
  // The design system's own assignment for the known fleet entries.
  assert.equal(toneFor("claude-code").bg, "#3c3489");
  assert.equal(toneFor("opencode").bg, "#085041");
  assert.equal(toneFor("codex-cli").bg, "#0c447c");
});

test("an unknown id gets a deterministic tone from the same palette", () => {
  const first = toneFor("my-own-agent");
  const second = toneFor("my-own-agent");
  assert.deepEqual(first, second);
  assert.match(first.bg, /^#[0-9a-f]{6}$/);
  // Different ids generally land on different tones.
  assert.notDeepEqual(toneFor("agent-a"), toneFor("agent-zzzz"));
});

test("initials come from the display name, with an id fallback", () => {
  assert.equal(initialsFor("Claude Code"), "CC");
  assert.equal(initialsFor("OpenCode"), "OP");
  assert.equal(initialsFor("Gemini CLI"), "GC");
  assert.equal(initialsFor("", "codex-cli"), "CC");
  assert.equal(initialsFor("aider"), "AI");
});

test("the binary name is the last path segment on both separators", () => {
  assert.equal(binaryName(["C:/tools/opencode.exe", "acp"]), "opencode.exe");
  assert.equal(binaryName(["/usr/local/bin/claude"]), "claude");
  assert.equal(binaryName("C:\\tools\\codex.cmd"), "codex.cmd");
  assert.equal(binaryName(undefined), "");
});
