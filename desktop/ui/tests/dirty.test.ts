// SPDX-License-Identifier: Apache-2.0
// The unsaved-work guard's bookkeeping: what the confirmation dialogs are
// driven by. No human edit may be discarded silently.

import assert from "node:assert/strict";
import test from "node:test";
import { get } from "svelte/store";

import {
  clearDirty,
  dirtyFiles,
  hasDirty,
  markClean,
  markDirty,
} from "../src/lib/editor/dirty.ts";

// Scenario: Cerrar pestaña sucia pide decisión
test("dirty tracking drives the guard and never double-counts", () => {
  clearDirty();
  assert.equal(hasDirty(), false);

  markDirty("src/lib.rs");
  markDirty("src/lib.rs"); // typing twice is still one dirty file
  assert.deepEqual(get(dirtyFiles), ["src/lib.rs"]);
  assert.equal(hasDirty(), true);

  markDirty("app.css");
  assert.deepEqual(get(dirtyFiles), ["src/lib.rs", "app.css"]);

  // Saving one file leaves the other guarded.
  markClean("src/lib.rs");
  assert.deepEqual(get(dirtyFiles), ["app.css"]);
  assert.equal(hasDirty(), true);

  markClean("app.css");
  assert.equal(hasDirty(), false);
});

test("cleaning an untracked file is a no-op", () => {
  clearDirty();
  markClean("never-opened.txt");
  assert.deepEqual(get(dirtyFiles), []);
});
