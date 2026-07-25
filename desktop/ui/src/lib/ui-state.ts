// SPDX-License-Identifier: Apache-2.0
// Surface state that outlives the process: theme, locale, last view, window
// geometry, editor recents, palette frecency and the "open with" template.
// Persisted by the Rust side in the user's data directory (design D3).

import { invoke } from "@tauri-apps/api/core";
import { get, writable, type Readable } from "svelte/store";

export type ThemeChoice = "system" | "light" | "dark";

export interface Usage {
  count: number;
  lastAt: number;
}

export interface UiState {
  theme: ThemeChoice | null;
  locale: "es" | "en" | null;
  lastView: string | null;
  window: unknown | null;
  openWithTemplate: string | null;
  editorRecents: Record<string, string[]>;
  paletteUsage: Record<string, Usage>;
  activeProject: string | null;
}

const EMPTY: UiState = {
  theme: null,
  locale: null,
  lastView: null,
  window: null,
  openWithTemplate: null,
  editorRecents: {},
  paletteUsage: {},
  activeProject: null,
};

const store = writable<UiState>(EMPTY);
export const uiState: Readable<UiState> = store;

let loaded = false;

/**
 * Reads the persisted state and applies the theme before the first paint. The
 * entry point awaits this BEFORE mounting, so a chosen theme never flashes its
 * opposite; calling it again returns the state already held.
 */
export async function loadUiState(): Promise<UiState> {
  if (loaded) return get(store);
  const state = { ...EMPTY, ...(await invoke<Partial<UiState>>("ui_state_load")) };
  state.editorRecents ??= {};
  state.paletteUsage ??= {};
  store.set(state);
  loaded = true;
  applyTheme(state.theme ?? "system");
  return state;
}

function persist(next: UiState): void {
  store.set(next);
  if (loaded) void invoke("ui_state_save", { state: next });
}

/** Applies a theme choice to the document (system = follow the OS). */
export function applyTheme(choice: ThemeChoice): void {
  const root = document.documentElement;
  if (choice === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", choice);
  }
}

export function setTheme(choice: ThemeChoice): void {
  applyTheme(choice);
  persist({ ...get(store), theme: choice });
}

export function setStoredLocale(locale: "es" | "en"): void {
  persist({ ...get(store), locale });
}

export function setLastView(view: string): void {
  const current = get(store);
  if (current.lastView === view) return;
  persist({ ...current, lastView: view });
}

export function setOpenWithTemplate(template: string): void {
  persist({ ...get(store), openWithTemplate: template.trim() || null });
}

export function setActiveProject(root: string | null): void {
  persist({ ...get(store), activeProject: root });
}

/** Records a palette invocation: recency + count drive the frecency order. */
export function recordPaletteUse(method: string): void {
  const current = get(store);
  const previous = current.paletteUsage[method] ?? { count: 0, lastAt: 0 };
  persist({
    ...current,
    paletteUsage: {
      ...current.paletteUsage,
      [method]: {
        count: previous.count + 1,
        lastAt: Math.floor(Date.now() / 1000),
      },
    },
  });
}

/** A frecency bonus for palette ranking: recent and frequent rise first. */
export function frecencyBonus(method: string): number {
  const usage = get(store).paletteUsage[method];
  if (!usage) return 0;
  const ageHours = (Date.now() / 1000 - usage.lastAt) / 3600;
  const recency = ageHours < 1 ? 30 : ageHours < 24 ? 20 : ageHours < 168 ? 10 : 4;
  return recency + Math.min(usage.count, 10) * 2;
}

/** The most recently opened files of a project, most recent first. */
export function recentFiles(project: string): string[] {
  return get(store).editorRecents[project] ?? [];
}

export function recordRecentFile(project: string, file: string): void {
  const current = get(store);
  const previous = current.editorRecents[project] ?? [];
  const next = [file, ...previous.filter((entry) => entry !== file)].slice(0, 20);
  persist({
    ...current,
    editorRecents: { ...current.editorRecents, [project]: next },
  });
}
