// SPDX-License-Identifier: Apache-2.0
// What happens to an open session when another is opened, when one is closed,
// and when the cap is reached. A tab can hold an unsent draft, so these rules
// decide whether human work survives — they are executed, not reviewed.

import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_SESSION_TABS,
  clearUnread,
  closeTab,
  markUnread,
  openTab,
  type SessionTab,
} from "../src/lib/session-tabs.ts";

function opened(...ids: string[]): SessionTab[] {
  return ids.map((sessionId) => ({ sessionId, unread: 0 }));
}

// Scenario: Abrir dos veces la misma sesión enfoca, no duplica
test("opening focuses an already-open session instead of duplicating it", () => {
  const first = openTab([], "a");
  assert.ok(!("full" in first) || first.full !== true);
  if ("full" in first && first.full) throw new Error("unreachable");
  assert.deepEqual(
    first.tabs.map((t) => t.sessionId),
    ["a"],
  );
  assert.equal(first.active, "a");

  // A second, different session joins — the first stays open.
  const second = openTab(first.tabs, "b");
  if ("full" in second && second.full) throw new Error("unreachable");
  assert.deepEqual(
    second.tabs.map((t) => t.sessionId),
    ["a", "b"],
  );
  assert.equal(second.active, "b");

  // Asking again for one already open moves the pointer, adds nothing, and
  // clears what accumulated while it was in the background.
  const withUnread = markUnread(second.tabs, "a");
  assert.equal(withUnread.find((t) => t.sessionId === "a")?.unread, 1);
  const again = openTab(withUnread, "a");
  if ("full" in again && again.full) throw new Error("unreachable");
  assert.deepEqual(
    again.tabs.map((t) => t.sessionId),
    ["a", "b"],
    "the order does not change either",
  );
  assert.equal(again.active, "a");
  assert.equal(again.tabs.find((t) => t.sessionId === "a")?.unread, 0);
});

// Scenario: Cerrar la pestaña activa cae en la vecina
test("closing the active tab falls to the left neighbour, then the last, then the list", () => {
  const three = opened("a", "b", "c");

  // Middle tab: the eye is already to its left.
  const middle = closeTab(three, "b", "b");
  assert.deepEqual(
    middle.tabs.map((t) => t.sessionId),
    ["a", "c"],
  );
  assert.equal(middle.active, "a");

  // First tab: no left neighbour, so the new last takes the front.
  const first = closeTab(three, "a", "a");
  assert.deepEqual(
    first.tabs.map((t) => t.sessionId),
    ["b", "c"],
  );
  assert.equal(first.active, "c");

  // The last one standing: the list is what remains.
  const only = closeTab(opened("a"), "a", "a");
  assert.deepEqual(only.tabs, []);
  assert.equal(only.active, null);

  // Closing a background tab does not move the front.
  const background = closeTab(three, "c", "a");
  assert.deepEqual(
    background.tabs.map((t) => t.sessionId),
    ["b", "c"],
  );
  assert.equal(background.active, "c");

  // Closing something that is not open changes nothing.
  const absent = closeTab(three, "b", "zzz");
  assert.equal(absent.tabs.length, 3);
  assert.equal(absent.active, "b");
});

// Scenario: El tope se rehúsa nombrando el remedio
test("the cap refuses instead of evicting, so no unsent draft is discarded", () => {
  const full = opened(...Array.from({ length: MAX_SESSION_TABS }, (_, i) => `s${i}`));
  const refused = openTab(full, "one-more");
  assert.deepEqual(refused, { full: true }, "nothing is opened");

  // The refusal is what proves nothing was evicted: the array is untouched.
  assert.equal(full.length, MAX_SESSION_TABS);
  assert.equal(full[0].sessionId, "s0");

  // A session already open is still reachable at the cap — focusing is not
  // opening, so the cap does not lock anyone out of what they have.
  const focus = openTab(full, "s3");
  if ("full" in focus && focus.full) throw new Error("focusing must not be refused");
  assert.equal(focus.active, "s3");
  assert.equal(focus.tabs.length, MAX_SESSION_TABS);
});

// Scenario: La pestaña de fondo dice que llegó algo
test("unread accumulates for the tab it belongs to and clears when it is read", () => {
  let tabs = opened("a", "b");
  tabs = markUnread(tabs, "a");
  tabs = markUnread(tabs, "a");
  tabs = markUnread(tabs, "b");
  assert.equal(tabs.find((t) => t.sessionId === "a")?.unread, 2);
  assert.equal(tabs.find((t) => t.sessionId === "b")?.unread, 1);

  tabs = clearUnread(tabs, "a");
  assert.equal(tabs.find((t) => t.sessionId === "a")?.unread, 0);
  assert.equal(tabs.find((t) => t.sessionId === "b")?.unread, 1, "and only that one");

  // A count for a session with no tab is not an error and creates nothing.
  assert.equal(markUnread(tabs, "ghost").length, 2);
});
