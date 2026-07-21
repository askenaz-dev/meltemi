// SPDX-License-Identifier: Apache-2.0
// Locale store + translation function. The locale follows the OS language
// with an explicit override persisted in the app's local settings.

import { derived, writable, type Readable } from "svelte/store";
import { messages, type Locale, type MessageKey } from "./messages";

const STORAGE_KEY = "meltemi.locale";

function detect(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "es" || stored === "en") {
    return stored;
  }
  return navigator.language?.toLowerCase().startsWith("es") ? "es" : "en";
}

const localeStore = writable<Locale>(detect());

export const locale: Readable<Locale> = localeStore;

export function setLocale(next: Locale): void {
  localStorage.setItem(STORAGE_KEY, next);
  localeStore.set(next);
}

export type Translate = (
  key: MessageKey,
  vars?: Record<string, string | number>,
) => string;

/** `$t("key")` — every visible string routes through the catalog. */
export const t: Readable<Translate> = derived(
  localeStore,
  ($locale): Translate =>
    (key, vars) => {
      let text: string = messages[$locale][key] ?? messages.es[key] ?? key;
      if (vars) {
        for (const [name, value] of Object.entries(vars)) {
          text = text.replaceAll(`{${name}}`, String(value));
        }
      }
      return text;
    },
);
