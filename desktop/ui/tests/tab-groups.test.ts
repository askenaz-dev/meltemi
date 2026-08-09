// SPDX-License-Identifier: Apache-2.0
// The four rules that decide whether grouped work stays reachable: one group
// per tab, an empty group stops existing, collapsing never closes anything, and
// the activity never hides inside a collapsed group.

import assert from "node:assert/strict";
import test from "node:test";

import {
  EMPTY_GROUPS,
  createGroup,
  forgetTab,
  groupOf,
  hiddenTabs,
  joinGroup,
  leaveGroup,
  renameGroup,
  setCollapsed,
} from "../src/lib/tab-groups.ts";

test("a tab belongs to at most one group, and joining moves it", () => {
  let s = createGroup(EMPTY_GROUPS, "a", "Refactor");
  s = createGroup(s, "b", "Bugs");
  assert.equal(groupOf(s, "a")?.name, "Refactor");
  assert.equal(groupOf(s, "b")?.name, "Bugs");

  const bugs = s.groups.find((g) => g.name === "Bugs")!;
  s = joinGroup(s, "a", bugs.id);
  assert.equal(groupOf(s, "a")?.name, "Bugs", "it moved rather than doubled");
  assert.equal(s.groups.length, 1, "and the group it left, now empty, is gone");

  // Joining a group that does not exist changes nothing at all.
  const before = JSON.stringify(s);
  assert.equal(JSON.stringify(joinGroup(s, "a", "no-such-group")), before);
});

// Scenario: Salir del grupo y el grupo que se queda vacío
test("the last tab leaving destroys the group, and the tab stays open", () => {
  let s = createGroup(EMPTY_GROUPS, "a", "Refactor");
  s = joinGroup(s, "b", s.groups[0].id);
  assert.equal(s.groups[0].members.length, 2);

  s = leaveGroup(s, "a");
  assert.equal(s.groups.length, 1, "one member left, so the group remains");
  assert.equal(groupOf(s, "a"), null, "and the tab is simply ungrouped");

  s = leaveGroup(s, "b");
  assert.deepEqual(s.groups, [], "a name with nothing behind it stops existing");

  // Closing a tab is the same thing for the model.
  let t = createGroup(EMPTY_GROUPS, "x", "Solo");
  t = forgetTab(t, "x");
  assert.deepEqual(t.groups, []);
});

// Scenario: Plegar guarda espacio, no trabajo
test("collapsing hides tabs from the strip and closes none of them", () => {
  let s = createGroup(EMPTY_GROUPS, "a", "Refactor");
  s = joinGroup(s, "b", s.groups[0].id);
  const id = s.groups[0].id;

  const out = setCollapsed(s, id, true, null, ["a", "b", "c"]);
  assert.equal(out.state.groups[0].collapsed, true);
  assert.deepEqual(
    out.state.groups[0].members,
    ["a", "b"],
    "collapsing is about space, never about the work",
  );
  assert.deepEqual([...hiddenTabs(out.state)].sort(), ["a", "b"]);

  // Expanding brings them back with their membership untouched.
  const back = setCollapsed(out.state, id, false, null, ["a", "b", "c"]);
  assert.deepEqual([...hiddenTabs(back.state)], []);
  assert.deepEqual(back.state.groups[0].members, ["a", "b"]);
});

// Scenario: Plegar el grupo de la pestaña activa mueve la actividad
test("collapsing the active tab's group moves the activity to a visible tab", () => {
  let s = createGroup(EMPTY_GROUPS, "a", "Refactor");
  s = joinGroup(s, "b", s.groups[0].id);
  const id = s.groups[0].id;

  // Active inside the group: it must not stay there.
  const moved = setCollapsed(s, id, true, "a", ["a", "b", "c"]);
  assert.equal(moved.active, "c", "the first tab outside the group takes over");

  // Active outside it: nothing moves.
  const still = setCollapsed(s, id, true, "c", ["a", "b", "c"]);
  assert.equal(still.active, "c");

  // Nothing left visible: the list, which is always there.
  const nowhere = setCollapsed(s, id, true, "a", ["a", "b"]);
  assert.equal(nowhere.active, null);

  // Expanding never moves the activity.
  const expanded = setCollapsed(moved.state, id, false, "c", ["a", "b", "c"]);
  assert.equal(expanded.active, "c");
});

test("renaming touches the name and nothing else", () => {
  let s = createGroup(EMPTY_GROUPS, "a", "Refactor");
  const id = s.groups[0].id;
  const color = s.groups[0].color;
  s = renameGroup(s, id, "Publicación");
  assert.equal(s.groups[0].name, "Publicación");
  assert.equal(s.groups[0].color, color);
  assert.deepEqual(s.groups[0].members, ["a"]);
});
