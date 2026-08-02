// SPDX-License-Identifier: Apache-2.0
// The shared unified-diff grammar (tablero-de-carrera design D3). Pure
// functions over text, so the parser the review drill-in and the race board
// share is testable without a window: node --test with Node's native type
// stripping, no test framework dependency.

import assert from "node:assert/strict";
import { test } from "node:test";
import { fileSections, hunksOf } from "../src/lib/diff.ts";

const TWO_FILES = [
  "diff --git a/src/a.rs b/src/a.rs",
  "--- a/src/a.rs",
  "+++ b/src/a.rs",
  "@@ -1,3 +1,4 @@",
  " fn main() {",
  "-    old();",
  "+    fresh();",
  "+    more();",
  " }",
  "diff --git a/README.md b/README.md",
  "--- a/README.md",
  "+++ b/README.md",
  "@@ -10,2 +10,2 @@",
  "-gone",
  "+here",
].join("\n");

test("Un diff se parte por archivo y numera contra el archivo nuevo", () => {
  const sections = fileSections(TWO_FILES);
  assert.deepEqual(
    sections.map((s) => s.file),
    ["src/a.rs", "README.md"],
  );

  const first = sections[0];
  const added = first.lines.filter((l) => l.kind === "add");
  assert.deepEqual(
    added.map((l) => l.newLine),
    [2, 3],
    "the added lines are numbered against the new file, in order",
  );
  const removed = first.lines.filter((l) => l.kind === "del");
  assert.equal(removed.length, 1);
  assert.equal(
    removed[0].newLine,
    null,
    "a removed line has no line in the new file, and does not borrow one",
  );
  // The ---/+++ header lines are metadata, not content.
  assert.deepEqual(
    first.lines.filter((l) => l.kind === "meta").map((l) => l.text),
    ["--- a/src/a.rs", "+++ b/src/a.rs"],
  );
  // The second file's numbering restarts from its own @@, not from the first.
  assert.equal(sections[1].lines.find((l) => l.kind === "add")?.newLine, 10);
});

test("Texto anterior al primer diff --git no inventa un archivo", () => {
  const sections = fileSections("noise before anything\n@@ -1 +1 @@\n+x");
  assert.deepEqual(sections, [], "nothing is attributed to a file nobody named");
});

test("Los hunks conservan su cabecera y su primera línea nueva", () => {
  const [section] = fileSections(TWO_FILES);
  const hunks = hunksOf(section);
  assert.equal(hunks.length, 1);
  assert.equal(hunks[0].header, "@@ -1,3 +1,4 @@");
  assert.equal(hunks[0].startLine, 1, "the hunk starts where its context does");
  assert.ok(
    hunks[0].lines.every((l) => l.kind !== "meta"),
    "metadata does not travel inside a hunk",
  );
});

test("Las líneas anteriores al primer @@ quedan visibles en su propio hunk", () => {
  // A rename/mode-only change has no @@ at all: dropping those lines would
  // silently hide a real change from the board.
  const section = fileSections(
    ["diff --git a/old.rs b/new.rs", "similarity index 100%", "rename from old.rs"].join("\n"),
  )[0];
  const hunks = hunksOf(section);
  assert.equal(hunks.length, 1);
  assert.equal(hunks[0].header, "", "an unlabeled hunk, not a missing one");
  assert.equal(hunks[0].lines.length, 2);
});
