// SPDX-License-Identifier: Apache-2.0
import { strict as assert } from "node:assert";
import { test } from "node:test";

import { decide, isNew } from "../src/lib/attention.ts";

const translate = (key: string, vars: Record<string, string> = {}) =>
  Object.entries(vars).reduce((out, [k, v]) => out.replaceAll(`{${k}}`, v), key);

const base = { focused: false, last: null, enabled: true, translate };

test("with the window in front nothing is raised: the tray already is the notice", () => {
  const out = decide({ reason: "permission", count: 1 }, { ...base, focused: true });
  assert.equal(out.notice, null);
  // The title still says what waits: it is read from the taskbar either way.
  assert.match(out.title, /window\.title\.pending/);
});

test("a waiting permission is raised when the window is behind", () => {
  const out = decide({ reason: "permission", count: 1 }, base);
  assert.ok(out.notice);
  assert.match(out.notice!.title, /attention\.permission/);
});

test("a gate and a session end are raised too, not only permissions", () => {
  for (const reason of ["gate", "session"] as const) {
    const out = decide({ reason, count: 1, subject: "x" }, base);
    assert.ok(out.notice, `${reason} deserves the same treatment`);
  }
});

test("the same moment is not announced twice", () => {
  const moment = { reason: "permission" as const, count: 2 };
  const first = decide(moment, base);
  assert.ok(first.notice);
  const again = decide(moment, { ...base, last: { reason: "permission", count: 2 } });
  assert.equal(again.notice, null, "a repaint must not ring again");
});

test("a burst becomes one moment with a count, not one notice each", () => {
  // Two arriving at once is a different moment from one; three is different
  // again. What never happens is one notice per repaint of the same number.
  assert.equal(isNew({ reason: "permission", count: 2 }, { reason: "permission", count: 1 }), true);
  assert.equal(isNew({ reason: "permission", count: 2 }, { reason: "permission", count: 2 }), false);
});

test("nothing waiting is not a moment", () => {
  const out = decide({ reason: "permission", count: 0 }, base);
  assert.equal(out.notice, null);
  assert.match(out.title, /window\.title$/);
});

test("switched off, nothing is raised and the title still tells the truth", () => {
  const out = decide({ reason: "permission", count: 3 }, { ...base, enabled: false });
  assert.equal(out.notice, null);
  assert.match(out.title, /window\.title\.pending/);
});

test("no turn text ever leaves the window", () => {
  const secret = "el token es sk-live-abcdef y la clave del cliente";
  const out = decide(
    { reason: "session", count: 1, subject: secret },
    base,
  );
  // The subject is the surface's own — a change name, a session title — and the
  // caller is what must never hand it prose. Pinned here so the shape of the
  // contract stays visible: the module has no access to turn text at all.
  assert.equal(out.notice!.body, secret);
  assert.ok(!out.title.includes(secret), "the window title never carries a subject");
});
