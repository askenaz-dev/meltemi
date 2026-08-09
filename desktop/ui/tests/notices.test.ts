// SPDX-License-Identifier: Apache-2.0
// Which notices expire and which never do. The line between them is a spec
// obligation — a timeout that expires or an operation that fails must not be
// discarded silently — so it is executed rather than reviewed.

import assert from "node:assert/strict";
import test from "node:test";
import { get } from "svelte/store";

import {
  NOTICE_TTL_MS,
  dismissAllNotices,
  dismissNotice,
  holdNotice,
  notices,
  pushNotice,
  releaseNotice,
} from "../src/lib/notices.ts";

// Scenario: La confirmación se retira sola
test("an informational notice retires on its own, and can be retired sooner", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  dismissAllNotices();

  pushNotice("enlace creado", "info");
  assert.equal(get(notices).length, 1);

  // Still there just before its time is up.
  t.mock.timers.tick(NOTICE_TTL_MS - 1);
  assert.equal(get(notices).length, 1, "it does not leave early");

  t.mock.timers.tick(2);
  assert.equal(get(notices).length, 0, "and it leaves without a gesture");

  // The control still works before the clock runs out, and cancels the timer so
  // nothing later tries to retire an id that is already gone.
  pushNotice("otra confirmación", "info");
  const id = get(notices)[0].id;
  dismissNotice(id);
  assert.equal(get(notices).length, 0);
  t.mock.timers.tick(NOTICE_TTL_MS * 2);
  assert.equal(get(notices).length, 0);
});

// Scenario: El error se queda hasta que alguien lo retira
test("a warning or an error has no clock at all", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  dismissAllNotices();

  pushNotice("permiso vencido: denegado por plazo", "warn");
  pushNotice("la operación falló", "danger");
  assert.equal(get(notices).length, 2);

  // However long: no timer exists that could reach them. This is the negative
  // half, and it is the one that matters.
  t.mock.timers.tick(NOTICE_TTL_MS * 100);
  assert.equal(get(notices).length, 2, "nothing retires an error but a gesture");

  // Releasing after a hover must not mint a clock for them either.
  for (const notice of get(notices)) releaseNotice(notice.id, notice.tone);
  t.mock.timers.tick(NOTICE_TTL_MS * 100);
  assert.equal(get(notices).length, 2);

  const [first] = get(notices);
  dismissNotice(first.id);
  assert.equal(get(notices).length, 1, "a gesture is what retires them");
});

// Scenario: Nada desaparece bajo la mano que iba a leerlo
test("holding a transient notice stops its clock, and leaving restarts it", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  dismissAllNotices();

  pushNotice("enlace creado", "info");
  const id = get(notices)[0].id;

  t.mock.timers.tick(NOTICE_TTL_MS - 100);
  holdNotice(id);
  // Held: however long the pointer stays, it stays.
  t.mock.timers.tick(NOTICE_TTL_MS * 5);
  assert.equal(get(notices).length, 1, "nothing fades under the hand reading it");

  // Leaving RESTARTS rather than resumes, so a notice read again lasts as long
  // as reading it takes.
  releaseNotice(id, "info");
  t.mock.timers.tick(NOTICE_TTL_MS - 1);
  assert.equal(get(notices).length, 1, "the clock started over, not resumed");
  t.mock.timers.tick(2);
  assert.equal(get(notices).length, 0);
});
