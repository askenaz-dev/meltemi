// SPDX-License-Identifier: Apache-2.0
// What the surface decides about the user's machines. The rules worth an
// executed test are the ones about not inventing: presence that is not there,
// and chrome for a fleet of one (maquinas-remotas design D4, D6).

import assert from "node:assert/strict";
import test from "node:test";

import {
  activeMachine,
  probeLine,
  withPresence,
  worthShowing,
  type MachineRow,
} from "../src/lib/machines.ts";
import type { IdentityStatus } from "../src/lib/identity.ts";

const configured: MachineRow[] = [
  { name: "esta", configured: "local", endpoint: "local", active: true },
  {
    name: "la-torre",
    configured: "exec:ssh la-torre meltemi bridge",
    endpoint: "exec:ssh la-torre meltemi bridge",
    via: "ssh la-torre",
    active: false,
  },
];

const mesh: IdentityStatus = {
  available: true,
  user: "guillmar@mock.example",
  machines: [
    { name: "la-torre", presence: "offline", thisMachine: false },
    { name: "el-portatil", presence: "online", thisMachine: false },
  ],
};

// Scenario: La presencia se muestra a pedido y no es una prueba de conexión
test("presence is crossed by name and never invented", () => {
  const shown = withPresence(configured, mesh);
  assert.equal(shown.length, 2);
  // Crosses by name, which is the only thing the two lists share.
  assert.equal(shown[1].presence, "offline");
  // And does NOT invent one for the machine the mesh says nothing about.
  assert.equal(shown[0].presence, undefined);
});

test("with no mesh identity nothing carries presence", () => {
  for (const identity of [null, { available: false, machines: [] } as IdentityStatus]) {
    const shown = withPresence(configured, identity);
    assert.deepEqual(
      shown.map((m) => m.presence),
      [undefined, undefined],
    );
  }
});

test("a mesh machine that is not configured adds no row", () => {
  // `el-portatil` is on the mesh and not in the configuration. The list is of
  // machines the user can CONNECT to, and presence does not make one of those.
  const shown = withPresence(configured, mesh);
  assert.deepEqual(
    shown.map((m) => m.name),
    ["esta", "la-torre"],
  );
});

test("the active machine is the one the client is on", () => {
  assert.equal(activeMachine(configured), "esta");
  assert.equal(
    activeMachine(configured.map((m) => ({ ...m, active: false }))),
    null,
  );
});

// Scenario: Sin máquinas configuradas el cromo no cambia
test("the chrome says nothing about machines unless there is a choice", () => {
  assert.equal(worthShowing([]), false);
  // One machine is not a fleet: naming the only computer somebody has is noise.
  assert.equal(worthShowing([configured[0]]), false);
  assert.equal(worthShowing(configured), true);
});

// Scenario: Probar una máquina dice si respondió y cuánto tardó
test("a probe reports the version and the time, or why not", () => {
  const words = { answered: "respondió", failed: "no respondió" };
  assert.equal(
    probeLine({ answered: true, version: "0.1.1", millis: 42 }, words),
    "respondió · 0.1.1 · 42 ms",
  );
  assert.equal(
    probeLine({ answered: false, millis: 20000, detail: "no answer within 20s" }, words),
    "no respondió: no answer within 20s",
  );
  // A failure with nothing to say still says it failed, rather than reading as
  // a success with an empty version.
  assert.equal(probeLine({ answered: false, millis: 5 }, words), "no respondió");
});
