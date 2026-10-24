// SPDX-License-Identifier: Apache-2.0
// The threshold of the waiting state. A skeleton that flashes on every fast
// load is worse than the word it replaced, so the rule is executed, not read.

import assert from "node:assert/strict";
import test, { mock } from "node:test";

import { SKELETON_DELAY_MS, revealAfter, skeletonWidths } from "../src/lib/skeleton.ts";

// Scenario: Una carga rápida no parpadea
test("a load that answers before the threshold never shows the skeleton", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    assert.equal(SKELETON_DELAY_MS, 150);
    let shown = false;
    const cancel = revealAfter(() => {
      shown = true;
    });
    mock.timers.tick(SKELETON_DELAY_MS - 1);
    // The data arrives: the view leaves its waiting branch and the skeleton
    // unmounts, which cancels the reveal.
    cancel();
    mock.timers.tick(10_000);
    assert.equal(shown, false);
  } finally {
    mock.timers.reset();
  }
});

// Scenario: Una carga lenta muestra el esqueleto
test("a load that outlasts the threshold shows the skeleton, once", () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    let reveals = 0;
    const cancel = revealAfter(() => {
      reveals += 1;
    });
    mock.timers.tick(SKELETON_DELAY_MS - 1);
    assert.equal(reveals, 0);
    mock.timers.tick(1);
    assert.equal(reveals, 1);
    // Cancelling after the reveal is harmless: unmounting is what withdraws it.
    cancel();
    mock.timers.tick(10_000);
    assert.equal(reveals, 1);
  } finally {
    mock.timers.reset();
  }
});

test("the bars keep fixed widths, so nothing jitters between draws", () => {
  assert.deepEqual(skeletonWidths(3), skeletonWidths(3));
  assert.equal(skeletonWidths(7).length, 7);
  assert.deepEqual(skeletonWidths(0), []);
  for (const width of skeletonWidths(10)) {
    assert.ok(width > 0 && width <= 100, `a bar is a share of its row: ${width}`);
  }
});
