// SPDX-License-Identifier: Apache-2.0
// The pairing screen's rules, executed (companero-que-conecta D7).

import assert from "node:assert/strict";
import test from "node:test";

import {
  actions,
  pairing,
  retriesByItself,
  worthSending,
} from "../src/lib/mobile-pair.ts";

// Scenario: La clave se ve y se copia sin conexión
test("never paired, the screen offers the key and a way in, and nothing else", () => {
  const state = pairing(null, { state: "connecting" }, 0);
  assert.deepEqual(state, { kind: "unpaired" });
  assert.deepEqual(actions(state!), ["copyKey", "connect"]);
});

// Scenario: El destino se escribe una vez
test("with a destination remembered, the screen is connecting to it and counts the attempts", () => {
  const first = pairing("ana@mac-studio", { state: "connecting" }, 0);
  assert.deepEqual(first, {
    kind: "connecting",
    destination: "ana@mac-studio",
    attempt: 1,
    detail: null,
  });
  const down = pairing(
    "ana@mac-studio",
    { state: "unreachable", detail: "could not reach ana@mac-studio", endpoint: "ana@mac-studio" },
    2,
  );
  assert.equal(down?.kind, "connecting");
  assert.equal(down && down.kind === "connecting" ? down.attempt : 0, 2);
  assert.equal(
    down && down.kind === "connecting" ? down.detail : null,
    "could not reach ana@mac-studio",
  );
  assert.ok(retriesByItself(down!), "the actor keeps dialling a machine that may answer in a moment");
  assert.deepEqual(actions(down!), ["cancel"]);
});

// Scenario: Conectar abre el túnel y la pantalla deja de prometer
test("connected, the pairing screen has nothing to say", () => {
  const state = pairing(
    "ana@mac-studio",
    { state: "connected", version: "0.2.0", uptimeSeconds: 1, sessions: 0, endpoint: "ana@mac-studio" },
    3,
  );
  assert.equal(state, null);
});

// Scenario: Una clave rechazada no se reintenta en bucle
test("a refusal is its own state, offers what fits its cause, and never retries by itself", () => {
  const changed = pairing(
    "ana@mac-studio",
    { state: "refused", kind: "hostKeyChanged", detail: "not that machine", endpoint: "ana@mac-studio" },
    0,
  );
  assert.equal(changed?.kind, "refused");
  assert.ok(!retriesByItself(changed!), "a changed host key is never dialled past");
  // Forgetting the host is the destructive choice, so it is not the one
  // under the thumb; staying put is.
  assert.deepEqual(actions(changed!), ["forgetHost", "stay"]);

  const rejected = pairing(
    "ana@mac-studio",
    { state: "refused", kind: "rejected", detail: "the server refused this app's key", endpoint: "ana@mac-studio" },
    0,
  );
  assert.ok(!retriesByItself(rejected!));
  // The key is at hand, because adding it to authorized_keys is the fix.
  assert.deepEqual(actions(rejected!), ["copyKey", "retry"]);

  // No state's actions include an automatic retry: `retry` is a person's
  // tap, offered only after a rejection, and never after a changed host key.
  assert.ok(!actions(changed!).includes("retry"));
});

test("a destination is worth sending only with a user, an at and no spaces", () => {
  assert.ok(worthSending("ana@mac-studio"));
  assert.ok(worthSending("  ana@[fe80::1]:2222 "));
  assert.ok(!worthSending(""));
  assert.ok(!worthSending("mac-studio"), "no user");
  assert.ok(!worthSending("@mac-studio"), "empty user");
  assert.ok(!worthSending("ana@"), "empty host");
  assert.ok(!worthSending("ana @mac-studio"), "a space is not a destination");
});
