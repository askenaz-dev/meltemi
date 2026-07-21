// SPDX-License-Identifier: Apache-2.0
// i18n lint (gui-shell "Internacionalización ES/EN"): refuses hardcoded
// visible strings in Svelte templates — every user-facing text must route
// through the message catalog (`$t("…")`). Heuristic but strict: a text node
// or labeling attribute containing a word of 3+ letters fails.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("../src", import.meta.url).pathname.replace(
  /^\/([A-Za-z]:)/,
  "$1",
);

const LABEL_ATTRS = /(title|placeholder|aria-label|alt|aria-description)\s*=\s*"([^"]*)"/g;
const WORD = /[A-Za-zÀ-ÖØ-öø-ÿ]{3,}/;

function* svelteFiles(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      yield* svelteFiles(path);
    } else if (name.endsWith(".svelte")) {
      yield path;
    }
  }
}

function templateOf(source) {
  return source
    .replace(/<script[\s\S]*?<\/script>/g, "")
    .replace(/<style[\s\S]*?<\/style>/g, "")
    .replace(/<!--[\s\S]*?-->/g, "");
}

function stripExpressions(text) {
  let previous;
  do {
    previous = text;
    text = text.replace(/\{[^{}]*\}/g, "");
  } while (text !== previous);
  return text;
}

const failures = [];

for (const file of svelteFiles(ROOT)) {
  const template = templateOf(readFileSync(file, "utf-8"));

  for (const match of template.matchAll(LABEL_ATTRS)) {
    if (WORD.test(match[2])) {
      failures.push(`${file}: attribute ${match[1]}="${match[2]}"`);
    }
  }

  const withoutExpressions = stripExpressions(template);
  for (const match of withoutExpressions.matchAll(/>([^<>]+)</g)) {
    const text = match[1].trim();
    if (text && WORD.test(text)) {
      failures.push(`${file}: text node "${text.slice(0, 60)}"`);
    }
  }
}

if (failures.length > 0) {
  console.error("i18n lint: hardcoded visible strings (route them through the catalog):");
  for (const failure of failures) {
    console.error("  - " + failure);
  }
  process.exit(1);
}
console.log("i18n lint: clean");
