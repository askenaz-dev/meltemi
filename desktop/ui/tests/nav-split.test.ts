// SPDX-License-Identifier: Apache-2.0
// The bounds of the sidebar split. A divider whose floors are wrong is a
// divider that can hide the navigation, so the floors are executed, not read.

import assert from "node:assert/strict";
import test from "node:test";

import {
  MIN_NAV_PX,
  MIN_TREE_PX,
  STEP_PX,
  clampNavHeight,
  stepNavHeight,
} from "../src/lib/nav-split.ts";

// Scenario: El reparto tiene suelo por los dos lados
test("the split is floored on both sides, and the cramped bar favours the entries", () => {
  const available = 600;

  // In the middle, the ask is honoured verbatim.
  assert.equal(clampNavHeight(300, available), 300);

  // Below the navigation's floor, the floor wins.
  assert.equal(clampNavHeight(0, available), MIN_NAV_PX);
  assert.equal(clampNavHeight(MIN_NAV_PX - 1, available), MIN_NAV_PX);

  // Above it, the tree's floor is what caps the navigation.
  assert.equal(clampNavHeight(available, available), available - MIN_TREE_PX);
  assert.equal(clampNavHeight(9999, available), available - MIN_TREE_PX);

  // The cramped bar: too short for both floors. The entries keep theirs and
  // the tree takes what is left — it scrolls rather than disappearing, and the
  // navigation never becomes unreachable.
  const cramped = MIN_NAV_PX + MIN_TREE_PX - 30;
  assert.equal(clampNavHeight(9999, cramped), MIN_NAV_PX);
  assert.equal(clampNavHeight(0, cramped), MIN_NAV_PX);
  assert.ok(clampNavHeight(500, cramped) === MIN_NAV_PX);

  // Fractional pixels from a pointer never reach the DOM.
  assert.equal(clampNavHeight(300.4, available), 300);
  assert.equal(clampNavHeight(300.6, available), 301);
});

// Scenario: El reparto se ajusta con el teclado
test("a keyboard step moves by one step and is clamped exactly like a drag", () => {
  const available = 600;

  assert.equal(stepNavHeight(300, STEP_PX, available), 300 + STEP_PX);
  assert.equal(stepNavHeight(300, -STEP_PX, available), 300 - STEP_PX);

  // Stepping past either end lands on the end, never beyond it.
  assert.equal(stepNavHeight(MIN_NAV_PX, -STEP_PX, available), MIN_NAV_PX);
  assert.equal(
    stepNavHeight(available - MIN_TREE_PX, STEP_PX, available),
    available - MIN_TREE_PX,
  );

  // Home and End are the two ends the component asks for by name.
  assert.equal(clampNavHeight(MIN_NAV_PX, available), MIN_NAV_PX);
  assert.equal(clampNavHeight(available - MIN_TREE_PX, available), available - MIN_TREE_PX);
});
