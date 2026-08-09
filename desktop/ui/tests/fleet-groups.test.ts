// SPDX-License-Identifier: Apache-2.0
// The two things that break silently when a listing is grouped: the order, and
// the row that belongs nowhere. Both executed rather than reviewed.

import assert from "node:assert/strict";
import test from "node:test";

import { groupFleet, type FleetRow } from "../src/lib/fleet-groups.ts";

type Agent = Parameters<typeof groupFleet>[0][number];

function agent(id: string, displayName: string): Agent {
  return {
    id,
    displayName,
    source: "registry",
    integrationLevel: 1,
    mcpSupport: false,
    detected: true,
    configured: false,
  } as Agent;
}

function subscription(name: string, under?: string): Agent {
  return {
    id: name,
    displayName: name,
    source: "profile",
    integrationLevel: 1,
    mcpSupport: false,
    detected: true,
    configured: false,
    underlyingAgent: under,
  } as Agent;
}

const shape = (rows: FleetRow[]) =>
  rows.map((r) => `${r.child ? "  " : ""}${r.agent.displayName}${r.orphan ? " (huérfana)" : ""}`);

// Scenario: Varias suscripciones del mismo agente se leen juntas
test("each agent is followed by its own subscriptions, counted, in name order", () => {
  const rows = groupFleet([
    agent("claude-code", "Claude Code"),
    agent("codex-cli", "Codex CLI"),
    subscription("trabajo", "claude-code"),
    subscription("personal", "codex-cli"),
    subscription("askenaz", "claude-code"),
    subscription("cliente", "codex-cli"),
  ]);

  assert.deepEqual(shape(rows), [
    "Claude Code",
    "  askenaz",
    "  trabajo",
    "Codex CLI",
    "  cliente",
    "  personal",
  ]);

  // The count is on the agent, so the total reads without counting rows.
  const claude = rows.find((r) => r.agent.id === "claude-code");
  const codex = rows.find((r) => r.agent.id === "codex-cli");
  assert.equal(claude?.subscriptions, 2);
  assert.equal(codex?.subscriptions, 2);

  // And every child says whose it is, by display name.
  for (const row of rows.filter((r) => r.child)) {
    assert.ok(row.belongsTo, `a subscription with no agent named: ${row.agent.id}`);
  }
  assert.equal(rows.find((r) => r.agent.id === "askenaz")?.belongsTo, "Claude Code");
});

test("an agent with no subscriptions carries no count and gains no children", () => {
  const rows = groupFleet([agent("aider", "Aider"), agent("kiro-cli", "Kiro CLI")]);
  assert.deepEqual(shape(rows), ["Aider", "Kiro CLI"]);
  assert.equal(rows[0].subscriptions, undefined);
  assert.equal(rows[0].child, false);
});

test("agents keep the order the catalog gave them", () => {
  const rows = groupFleet([
    agent("zeta", "Zeta"),
    agent("alfa", "Alfa"),
    agent("mu", "Mu"),
  ]);
  // Not sorted: the catalog already decided, and a second opinion here would
  // silently disagree with every other surface.
  assert.deepEqual(shape(rows), ["Zeta", "Alfa", "Mu"]);
});

// Scenario: La suscripción sin agente conocido no desaparece
test("a subscription whose agent is not in the catalog is listed, marked, at the end", () => {
  const rows = groupFleet([
    agent("claude-code", "Claude Code"),
    subscription("trabajo", "claude-code"),
    subscription("fantasma", "un-agente-que-ya-no-existe"),
    subscription("sin-agente", undefined),
  ]);

  assert.deepEqual(shape(rows), [
    "Claude Code",
    "  trabajo",
    "  fantasma (huérfana)",
    "  sin-agente (huérfana)",
  ]);

  // It still says what it was aiming at, so the row is a diagnosis and not a
  // mystery. One that declares nothing says nothing rather than inventing.
  assert.equal(rows.find((r) => r.agent.id === "fantasma")?.belongsTo, "un-agente-que-ya-no-existe");
  assert.equal(rows.find((r) => r.agent.id === "sin-agente")?.belongsTo, "");

  // And it is never counted against an agent that does not own it.
  assert.equal(rows.find((r) => r.agent.id === "claude-code")?.subscriptions, 1);
});

test("an empty fleet groups to nothing rather than throwing", () => {
  assert.deepEqual(groupFleet([]), []);
});
