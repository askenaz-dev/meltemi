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
  /** Whether the navigation sidebar is folded to its rail. */
  navCollapsed: boolean;
  /** Navigation height in px; null means "as the browser lays it out". */
  navSplit: number | null;
  /**
   * Whether the composer has already acknowledged the detected fleet once.
   * Once said, never again: the recognition is a greeting, not a badge
   * (primer-arranque-del-home design D3).
   */
  fleetGreeted: boolean;
  /** Whether OS notices are on. Absent means on: the surface asks for the
      permission at the first real moment, so the default costs nothing until
      there is something to say (avisos-de-escritorio design D1). */
  noticesEnabled: boolean;
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
  navCollapsed: false,
  navSplit: null,
  fleetGreeted: false,
  noticesEnabled: true,
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

/** Remembers the sidebar's folded state beside the theme and the geometry:
    a layout preference belongs with the other layout preferences (D3). */
export function setNavCollapsed(collapsed: boolean): void {
  persist({ ...get(store), navCollapsed: collapsed });
}

/** Remembers the split, on release and never mid-drag: this writes the whole
    object, and ~200 writes per drag would feed a race the host already has
    with its own load-modify-save of the window geometry (D3). */
export function setNavSplit(px: number | null): void {
  persist({ ...get(store), navSplit: px });
}

export function setActiveProject(root: string | null): void {
  persist({ ...get(store), activeProject: root });
}

/** Turns OS notices on or off; the attention request keeps its own rule. */
export function setNoticesEnabled(on: boolean): void {
  persist({ ...get(store), noticesEnabled: on });
}

/** Marks the fleet acknowledgement as said, so it is never said twice. */
export function markFleetGreeted(): void {
  persist({ ...get(store), fleetGreeted: true });
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
