// SPDX-License-Identifier: Apache-2.0
// The conversational fold (lanzador-conversacional design D4). Pure functions
// over the event log, so the grammar is testable without a window: node --test
// with Node's native type stripping, no test framework dependency.

import assert from "node:assert/strict";
import { test } from "node:test";
import { fold, type FoldEvent } from "../src/lib/conversation.ts";

let seq = 0;
function event(type: string, payload: unknown): FoldEvent {
  seq += 1;
  return { id: seq, ts: "2026-07-31T10:00:00Z", type, payload };
}

/** An ACP agent message chunk, the level 1/2 shape. */
function chunk(text: string): FoldEvent {
  return event("agent_update", {
    update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text } },
  });
}

test("Un turno se pliega de prompt a cierre", () => {
  const items = fold([
    event("prompt_sent", { text: "arregla el build" }),
    chunk("mirando "),
    chunk("el build…"),
    event("turn_completed", { stopReason: "completed" }),
  ]);

  assert.equal(items.length, 2);
  assert.equal(items[0].kind, "human");
  assert.equal(items[1].kind, "agent");
  if (items[0].kind !== "human" || items[1].kind !== "agent") return;
  assert.equal(items[0].text, "arregla el build");
  assert.equal(items[0].pending, false);
  // Chunks accumulate into one turn rather than one bubble per chunk.
  assert.deepEqual(items[1].parts, [{ kind: "text", text: "mirando el build…" }]);
  // The closing reason is shown, not swallowed.
  assert.equal(items[1].closed, true);
  assert.equal(items[1].stopReason, "completed");
});

test("La instruccion encolada se muestra pendiente hasta su prompt", () => {
  const queued = fold([event("instruction_queued", { instruction: "y corre los tests" })]);
  assert.equal(queued.length, 1);
  assert.equal(queued[0].kind === "human" && queued[0].pending, true);

  // When the queue drains, the very same bubble becomes the sent prompt: two
  // bubbles for one instruction would read as two instructions.
  const dispatched = fold([
    event("instruction_queued", { instruction: "y corre los tests" }),
    event("prompt_sent", { text: "y corre los tests" }),
  ]);
  assert.equal(dispatched.length, 1);
  assert.equal(dispatched[0].kind === "human" && dispatched[0].pending, false);
});

test("El pensamiento no se mezcla con la respuesta", () => {
  const items = fold([
    event("prompt_sent", { text: "hola" }),
    event("agent_update", {
      update: { sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "dudo…" } },
    }),
    chunk("hola a ti"),
  ]);

  const turn = items[1];
  assert.equal(turn.kind, "agent");
  if (turn.kind !== "agent") return;
  assert.deepEqual(turn.parts, [
    { kind: "thought", text: "dudo…" },
    { kind: "text", text: "hola a ti" },
  ]);
});

test("Evento no clasificable cae a la vista, no al olvido", () => {
  const items = fold([
    event("prompt_sent", { text: "hola" }),
    // A type the fold has no grammar for.
    event("moon_phase_reported", { detail: "waxing" }),
    // And an `agent_update` whose shape it does not recognize: it must NOT be
    // swallowed into the prose, because folding what we do not understand into
    // the answer is inventing.
    event("agent_update", { update: { sessionUpdate: "something_new", weird: true } }),
  ]);

  assert.equal(items.length, 3);
  assert.equal(items[1].kind, "system");
  assert.equal(items[1].kind === "system" && items[1].type, "moon_phase_reported");
  assert.equal(items[2].kind, "system");
  assert.equal(items[2].kind === "system" && items[2].type, "agent_update");
});

test("Las tres formas de actualizacion del agente se leen como prosa", () => {
  // Level 1/2 (ACP), a level-3 line the daemon mapped, and a raw string for a
  // line it could not map: all three are the agent talking.
  const items = fold([
    chunk("uno "),
    event("agent_update", { update: { type: "text", text: "dos " } }),
    event("agent_update", { update: "tres" }),
  ]);

  assert.equal(items.length, 1);
  assert.equal(items[0].kind, "agent");
  assert.equal(items[0].kind === "agent" && items[0].parts.length, 1);
  assert.equal(
    items[0].kind === "agent" && items[0].parts[0].kind === "text" && items[0].parts[0].text,
    "uno dos tres",
  );
});

test("Una llamada a herramienta se actualiza en su sitio", () => {
  const items = fold([
    event("agent_update", {
      update: { sessionUpdate: "tool_call", toolCallId: "t1", title: "grep", status: "pending" },
    }),
    event("agent_update", {
      update: { sessionUpdate: "tool_call_update", toolCallId: "t1", status: "completed" },
    }),
  ]);

  const turn = items[0];
  assert.equal(turn.kind, "agent");
  if (turn.kind !== "agent") return;
  assert.equal(turn.parts.length, 1);
  assert.deepEqual(turn.parts[0], {
    kind: "tool",
    toolCallId: "t1",
    title: "grep",
    status: "completed",
  });
});

test("Una peticion de permiso se pliega a tarjeta y su decision la resuelve", () => {
  const items = fold([
    event("permission_requested", {
      request: {
        toolCall: { toolCallId: "c1", title: "write src/main.rs" },
        options: [{ optionId: "allow", name: "Allow" }],
      },
    }),
    event("permission_decided", {
      outcome: { outcome: "selected", optionId: "allow" },
      decidedBy: "client",
      denied: false,
    }),
  ]);

  assert.equal(items.length, 1);
  const card = items[0];
  assert.equal(card.kind, "permission");
  if (card.kind !== "permission") return;
  assert.equal(card.title, "write src/main.rs");
  assert.deepEqual(card.decided, { by: "client", denied: false, rule: null });
});

test("El conmutador no pierde nada", () => {
  // The property the switch rests on: the conversation accounts for EVERY event
  // handed in, exactly once. The operator log renders them one to one, so if
  // this holds the two readings show the same set and the counter is the same
  // number in both.
  const feed = [
    event("session_started", { sessionId: "s1" }),
    event("agent_resolved", { binary: "mock-agent" }),
    event("prompt_sent", { text: "hola" }),
    event("agent_update", {
      update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "hey" } },
    }),
    event("permission_requested", { request: { toolCall: { title: "rm -rf" }, options: [] } }),
    event("permission_decided", { outcome: {}, decidedBy: "timeout", denied: true }),
    event("usage_reported", { source: "official" }),
    event("turn_completed", { stopReason: "completed" }),
    event("session_ended", {}),
  ];

  const accounted = fold(feed).flatMap((item) => item.eventIds);
  assert.equal(accounted.length, feed.length, "every event is accounted for exactly once");
  assert.deepEqual(
    [...accounted].sort((a, b) => a - b),
    feed.map((one) => one.id),
    "and they are the very events handed in, none invented",
  );
});

test("Conmutar entre conversación y log de operador", () => {
  // Ordering is the other half of "nothing is lost": the reading is in arrival
  // order, so switching lenses never reorders the session's history.
  const feed = [
    event("prompt_sent", { text: "uno" }),
    event("mystery_event", { detail: "?" }),
    event("prompt_sent", { text: "dos" }),
  ];
  const ids = fold(feed).flatMap((item) => item.eventIds);
  assert.deepEqual(ids, feed.map((one) => one.id));
});
