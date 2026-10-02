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

// The two boundaries the review screen rests on, asserted against the source
// because they are rules about what is ABSENT — and absence is what quietly
// stops being true when somebody adds a field.
test("the review screen adopts with confirmation and offers nowhere to type", async () => {
  const { readFile } = await import("node:fs/promises");
  const mobile = await readFile(
    new URL("../src/Mobile.svelte", import.meta.url),
    "utf8",
  );

  // Scenario: Adopción de un archivo de un competidor desde el móvil
  const adopt = mobile.slice(mobile.indexOf("async function adopt("));
  const body = adopt.slice(0, adopt.indexOf("\n  }"));
  assert.ok(
    body.includes("confirm: true"),
    "the daemon's own gate is asked for explicitly",
  );
  assert.ok(
    mobile.includes('role="dialog"') && mobile.includes("mobile.review.adopt.ask"),
    "and the surface asks first, in its own words",
  );

  // Scenario: Propuesta de autoría en el móvil
  // The only free-text input in the whole companion is the composer, and what it
  // carries is an instruction for the agent, not content for a file.
  const inputs = mobile.match(/<textarea|<input/g) ?? [];
  assert.equal(
    inputs.length,
    1,
    `the companion has exactly one text field, the composer: found ${inputs.length}`,
  );
  assert.ok(
    mobile.includes("mobile.direct.placeholder"),
    "and that one is the composer",
  );
});
