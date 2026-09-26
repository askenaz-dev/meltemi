// SPDX-License-Identifier: Apache-2.0
// What the chrome says about talking to another machine. A surface that stayed
// quiet would make an action on the wrong machine look like an action on your
// own, so this is executed rather than assumed.

import assert from "node:assert/strict";
import test from "node:test";

import { via } from "../src/lib/via.ts";

// Scenario: El cromo nombra la vía
test("a command endpoint shows the program and the host", () => {
  assert.equal(via("exec:ssh my-pc meltemi bridge"), "ssh my-pc");
});

test("a spelled-out path shows its name, not the whole path", () => {
  assert.equal(
    via('exec:"C:/Program Files/OpenSSH/ssh.exe" my-pc meltemi bridge'),
    "ssh my-pc",
  );
});

test("a command with no arguments still names itself", () => {
  assert.equal(via("exec:local-helper"), "local-helper");
});

test("a local socket has no via to name", () => {
  // The chrome must not invent a via for this machine's own endpoint: "here" is
  // not worth a segment.
  assert.equal(via("\\\\.\\pipe\\meltemid-guill"), null);
  assert.equal(via("/run/user/1000/meltemi/meltemid.sock"), null);
  assert.equal(via(undefined), null);
  assert.equal(via(""), null);
});

test("an endpoint that names nothing shows nothing", () => {
  assert.equal(via("exec:"), null);
  assert.equal(via("exec:   "), null);
});
