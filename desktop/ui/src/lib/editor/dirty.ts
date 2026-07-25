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

/**
 * The save-all channel between the shell's unsaved-work guard and the editor:
 * the guard offers "save and continue" (the third path the requirement names),
 * and only the editor can flush its buffers through the daemon. The editor
 * registers its flusher while it is mounted.
 */
type Flusher = () => Promise<void>;
let flusher: Flusher | null = null;

export function registerSaveAll(handler: Flusher): () => void {
  flusher = handler;
  return () => {
    if (flusher === handler) flusher = null;
  };
}

/** Asks the mounted editor to save every dirty buffer; throws if a save failed. */
export async function requestSaveAll(): Promise<void> {
  if (!flusher) return;
  await flusher();
}
