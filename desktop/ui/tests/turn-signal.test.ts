// SPDX-License-Identifier: Apache-2.0
// The life signal of a turn (senales-de-vida design D4). Built from the same
// fold the transcript draws, so the cases are real event sequences and not
// hand-made items that the fold would never produce.

import assert from "node:assert/strict";
import { test } from "node:test";

import { fold, type FoldEvent } from "../src/lib/conversation.ts";
import { durationLabel } from "../src/lib/time.ts";
import { elapsedClock, lifeSignal } from "../src/lib/turn-signal.ts";

let seq = 0;
function event(type: string, payload: unknown, ts = "2026-09-26T10:00:00Z"): FoldEvent {
  seq += 1;
  return { id: seq, ts, type, payload };
}

function chunk(text: string): FoldEvent {
  return event("agent_update", {
    update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text } },
  });
}

// Scenario: Antes del primer fragmento se dice que trabaja
test("before the first fragment the turn is waiting, timed from its instruction", () => {
  const items = fold([event("prompt_sent", { text: "fix the build" }, "2026-09-26T10:00:05Z")]);
  assert.deepEqual(lifeSignal(items, true), { kind: "waiting", since: "2026-09-26T10:00:05Z" });

  // A queued instruction behind the turn does not move the clock: the agent
  // is still on the first one.
  const queued = fold([
    event("prompt_sent", { text: "fix the build" }, "2026-09-26T10:00:05Z"),
    event("instruction_queued", { instruction: "then run the tests" }, "2026-09-26T10:00:09Z"),
  ]);
  assert.deepEqual(lifeSignal(queued, true), { kind: "waiting", since: "2026-09-26T10:00:05Z" });

  // After a decided permission the agent works again and has said nothing
  // new: still the same turn, still timed from its instruction.
  const resumed = fold([
    event("prompt_sent", { text: "fix the build" }, "2026-09-26T10:00:05Z"),
    event("permission_requested", { request: { toolCall: { title: "edit" }, options: [] } }),
    event("permission_decided", { decidedBy: "user", denied: false }),
  ]);
  assert.deepEqual(lifeSignal(resumed, true), { kind: "waiting", since: "2026-09-26T10:00:05Z" });

  // A session that works before any instruction is in view still says so,
  // without inventing a time.
  assert.deepEqual(lifeSignal([], true), { kind: "waiting", since: null });
});

// Scenario: El cursor sigue al último fragmento sin parpadear
test("an open turn with fragments is where the cursor goes, and a closed one has none", () => {
  const open = fold([event("prompt_sent", { text: "fix the build" }), chunk("looking at ")]);
  const turn = open[1];
  assert.equal(turn.kind, "agent");
  assert.deepEqual(lifeSignal(open, true), { kind: "streaming", itemId: turn.id });

  const closed = fold([
    event("prompt_sent", { text: "fix the build" }),
    chunk("done"),
    event("turn_completed", { stopReason: "end_turn" }),
  ]);
  // Even while a lagging state still says the agent works.
  assert.deepEqual(lifeSignal(closed, true), { kind: "none" });
  assert.deepEqual(lifeSignal(closed, false), { kind: "none" });
});

// Scenario: Esperar una decisión no es trabajar
test("waiting on a decision or an instruction is not working", () => {
  const asking = fold([
    event("prompt_sent", { text: "fix the build" }),
    chunk("I need to edit a file"),
    event("permission_requested", { request: { toolCall: { title: "edit" }, options: [] } }),
  ]);
  // The session's state says it waits: no signal at all.
  assert.deepEqual(lifeSignal(asking, false), { kind: "none" });
  // And a card still undecided is a wait even if the state lags behind it.
  assert.deepEqual(lifeSignal(asking, true), { kind: "none" });

  const between = fold([
    event("prompt_sent", { text: "fix the build" }),
    chunk("done"),
    event("turn_completed", { stopReason: "end_turn" }),
  ]);
  assert.deepEqual(lifeSignal(between, false), { kind: "none" });
});

// Scenario: El tiempo de la sesión corre mientras vive
test("the clock reads m:ss past the instruction, and a session's time runs until it ends", () => {
  assert.equal(elapsedClock(0), "0:00");
  assert.equal(elapsedClock(59_999), "0:59");
  assert.equal(elapsedClock(61_000), "1:01");
  assert.equal(elapsedClock(3_725_000), "1:02:05");
  assert.equal(elapsedClock(-5_000), "0:00");

  const start = "2026-09-26T10:00:00Z";
  const later = Date.parse("2026-09-26T10:00:42Z");
  // A living session is measured against the clock it is handed, so a tick
  // that hands a new instant is what makes it run.
  assert.equal(durationLabel(start, undefined, later), "42s");
  assert.equal(durationLabel(start, undefined, later + 30_000), "1m 12s");
  // An ended one is fixed at its final duration, whatever the clock says.
  assert.equal(durationLabel(start, "2026-09-26T10:05:00Z", later), "5m 0s");
});
