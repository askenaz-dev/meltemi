// SPDX-License-Identifier: Apache-2.0
// What the chrome decides about the user's mesh identity. The rules worth an
// executed test are the ones about asking for nothing: a permanent invitation in
// the chrome of somebody who never wanted identity would be asking for something
// (identidad-propia design D6, D7).

import assert from "node:assert/strict";
import test from "node:test";

import {
  footIdentity,
  orderedMachines,
  presenceGlyph,
  type IdentityStatus,
  type MeshMachine,
} from "../src/lib/identity.ts";

const linked: IdentityStatus = {
  available: true,
  user: "guillmar@mock.example",
  provider: "https://headscale.askenaz.dev",
  this: "mi-pc",
  machines: [
    { name: "mi-pc", presence: "online", system: "windows", thisMachine: true },
    {
      name: "la-torre",
      presence: "offline",
      system: "linux",
      lastSeen: "2026-09-30T18:21:05Z",
      thisMachine: false,
    },
  ],
};

// Scenario: Sin configurar, el cromo no pide nada
test("without an identity table the foot shows nothing at all", () => {
  assert.deepEqual(
    footIdentity({ available: false, reason: "not_configured", machines: [] }),
    { kind: "hidden" },
  );
  // And before the daemon has answered, which is most of the first second:
  // chrome that appears late is chrome that flickers.
  assert.deepEqual(footIdentity(null), { kind: "hidden" });
});

// Scenario: Sin enlazar, la barra ofrece enlazar y no exige nada
test("linking is offered only when this machine is what is missing", () => {
  assert.deepEqual(footIdentity({ available: false, reason: "not_linked", machines: [] }), {
    kind: "link",
  });
});

test("linking is not offered when there is no client to run", () => {
  // Saying "link this machine" with no mesh client installed would be false:
  // there is nothing to run. The row leads to the diagnosis instead of
  // inventing a remedy.
  assert.deepEqual(footIdentity({ available: false, reason: "client_missing", machines: [] }), {
    kind: "attention",
  });
});

test("a linked mesh shows the login name with the count of machines", () => {
  assert.deepEqual(footIdentity(linked), {
    kind: "who",
    user: "guillmar@mock.example",
    machines: 2,
  });
});

test("an identity the mesh did not name is not claimed", () => {
  const nameless: IdentityStatus = { available: true, machines: [] };
  assert.deepEqual(footIdentity(nameless), { kind: "attention" });
});

test("this machine's name stands in when the mesh names no login", () => {
  const noLogin: IdentityStatus = { available: true, this: "mi-pc", machines: [] };
  assert.deepEqual(footIdentity(noLogin), { kind: "who", user: "mi-pc", machines: 0 });
});

// Scenario: Ajustes muestra quién soy y mis equipos
test("presence is a filled symbol when here and a hollow one when not", () => {
  assert.equal(presenceGlyph("online"), "●");
  assert.equal(presenceGlyph("offline"), "○");
  // The two must differ, which is the whole point of a symbol: a vocabulary
  // where both states look the same is colour alone by another route.
  assert.notEqual(presenceGlyph("online"), presenceGlyph("offline"));
});

test("the machine list puts this machine first and then sorts by name", () => {
  const machines: MeshMachine[] = [
    { name: "zeta", presence: "online", thisMachine: false },
    { name: "alfa", presence: "offline", thisMachine: false },
    { name: "mi-pc", presence: "online", thisMachine: true },
  ];
  assert.deepEqual(
    orderedMachines(machines).map((m) => m.name),
    ["mi-pc", "alfa", "zeta"],
  );
});

test("ordering does not mutate what it was given", () => {
  const machines: MeshMachine[] = [
    { name: "zeta", presence: "online", thisMachine: false },
    { name: "mi-pc", presence: "online", thisMachine: true },
  ];
  orderedMachines(machines);
  assert.deepEqual(
    machines.map((m) => m.name),
    ["zeta", "mi-pc"],
  );
});
