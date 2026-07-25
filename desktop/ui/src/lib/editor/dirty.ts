// SPDX-License-Identifier: Apache-2.0
// Which editor tabs carry unsaved work. Ephemeral (never persisted): the guard
// exists so no human edit is discarded silently — on tab close, on navigation
// and on window close.

import { get, writable, type Readable } from "svelte/store";

const store = writable<string[]>([]);

/** Paths with unsaved changes, in open order. */
export const dirtyFiles: Readable<string[]> = store;

export function markDirty(path: string): void {
  const current = get(store);
  if (!current.includes(path)) store.set([...current, path]);
}

export function markClean(path: string): void {
  store.set(get(store).filter((entry) => entry !== path));
}

export function clearDirty(): void {
  store.set([]);
}

export function hasDirty(): boolean {
  return get(store).length > 0;
}
