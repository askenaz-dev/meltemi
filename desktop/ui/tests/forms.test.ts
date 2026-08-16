// SPDX-License-Identifier: Apache-2.0
// The generated palette forms must describe the contract faithfully: a form
// that lies about its method would send invalid params with a confident UI.

import assert from "node:assert/strict";
import test from "node:test";

import { METHOD_FORMS } from "../src/lib/generated/method-forms.ts";

// Scenario: Formulario tipado con obligatorios marcados
test("apply-edit's form mirrors its schema exactly", () => {
  const form = METHOD_FORMS["worktree/apply-edit"];
  assert.ok(form, "the method has a typed form");
  assert.equal(form.schema, "worktree.schema.json");
  assert.equal(form.def, "applyEditParams");
  assert.deepEqual(
    form.fields.map((field) => [field.name, field.kind, field.required]),
    [
      ["projectRoot", "string", true],
      ["change", "string", false],
      ["task", "string", false],
      ["agent", "string", false],
      ["file", "string", true],
      ["content", "string", true],
      ["confirm", "boolean", false],
    ],
  );
});

test("every form resolves to a schema of its own family", () => {
  for (const [method, form] of Object.entries(METHOD_FORMS)) {
    const family = method.split("/")[0];
    const file = form.schema.replace(".schema.json", "");
    const ok =
      file.startsWith(family) ||
      family.startsWith(file.split("-")[0]) ||
      // Single-method schemas keep their own names (validate, implement…).
      ["validate", "implement", "verify-archive", "change", "spec", "repo-map", "commit"].includes(
        file,
      );
    assert.ok(ok, `${method} resolved to ${form.schema}`);
  }
});

test("required flags and enums survive generation", () => {
  const dispatch = METHOD_FORMS["worktree/dispatch"];
  // Its four identifying fields are required, and the generator says so. It
  // used to be every field; `mode` is optional and that is the point of it —
  // declaring no mode is not declaring one (modos-de-autonomia).
  const required = dispatch.fields.filter((field) => field.required).map((field) => field.name);
  assert.deepEqual(required.sort(), ["agent", "change", "projectRoot", "task"]);
  const optional = dispatch.fields.filter((field) => !field.required).map((field) => field.name);
  assert.deepEqual(optional, ["mode"], "and the optional one is optional in the form too");
  // A schema enum becomes a closed option list in the form.
  const withOptions = Object.values(METHOD_FORMS).flatMap((form) =>
    form.fields.filter((field) => field.options),
  );
  for (const field of withOptions) {
    assert.ok(Array.isArray(field.options) && field.options.length > 0);
  }
});
