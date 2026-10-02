// SPDX-License-Identifier: Apache-2.0
// The companion's boundary, executed. `mobile-companion` limits this surface to
// four verbs and forbids authoring; a comment saying so would be a comment, and
// these are the two rules most likely to be widened by accident.

import assert from "node:assert/strict";
import test from "node:test";

import {
  DESTINATIONS,
  TOUCH_MIN_PX,
  destructiveMayRestUnderThumb,
  mayAuthor,
} from "../src/lib/mobile-nav.ts";

// Scenario: Propuesta de autoría en el móvil
test("the companion does not author, and says so in a way a test can hold", () => {
  assert.equal(mayAuthor(), false);
});

test("there are four destinations and a fifth needs an amendment, not an afternoon", () => {
  assert.equal(DESTINATIONS.length, 4);
  assert.deepEqual(
    DESTINATIONS.map((d) => d.id),
    ["sessions", "permissions", "review", "settings"],
  );
  // Directing is not a destination: it is something done to a session you are
  // looking at, not a place to go. Its absence here is deliberate.
  assert.ok(!DESTINATIONS.some((d) => d.id === ("direct" as never)));
});

test("every destination's label is a message key and never text", () => {
  for (const destination of DESTINATIONS) {
    assert.match(
      destination.label,
      /^mobile\.nav\./,
      `${destination.id} must label itself from the catalogue`,
    );
  }
});

test("the touch target is the design system's own token", () => {
  assert.equal(TOUCH_MIN_PX, 44);
});

test("a destructive action never rests under the thumb", () => {
  // Fixed by the mock-up before the app existed: on a phone the risk is the
  // thumb.
  assert.equal(destructiveMayRestUnderThumb(), false);
});
