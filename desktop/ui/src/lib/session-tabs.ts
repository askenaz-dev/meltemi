// SPDX-License-Identifier: Apache-2.0
// Which sessions are open, in which order, and which one is in front. Pure
// reducers over arrays: no store, no Svelte, so the rules that decide what
// happens to a tab are driven by an executed test rather than by a click.

export interface SessionTab {
  sessionId: string;
  /** Events that arrived while this tab was not on screen. */
  unread: number;
}

/**
 * The tree already slices a project's sessions at eight, and a surface should
 * not contradict its own number.
 */
export const MAX_SESSION_TABS = 8;

/**
 * Open a session, or focus it if it is already open.
 *
 * At the cap this REFUSES rather than evicting: a background tab can hold an
 * unsent draft, and discarding it by an invisible rule is worse than saying so.
 */
export function openTab(
  tabs: SessionTab[],
  sessionId: string,
): { tabs: SessionTab[]; active: string; full?: false } | { full: true } {
  const existing = tabs.findIndex((tab) => tab.sessionId === sessionId);
  if (existing >= 0) {
    // Open-or-focus: the order does not change, and coming to the front is
    // what clears the count.
    return {
      tabs: tabs.map((tab) => (tab.sessionId === sessionId ? { ...tab, unread: 0 } : tab)),
      active: sessionId,
    };
  }
  if (tabs.length >= MAX_SESSION_TABS) return { full: true };
  return { tabs: [...tabs, { sessionId, unread: 0 }], active: sessionId };
}

/**
 * Close a tab and say what is in front afterwards.
 *
 * Closing the active tab falls to the LEFT neighbour — where the eye already
 * is — then to the new last, then to `null`, which means the list.
 */
export function closeTab(
  tabs: SessionTab[],
  active: string | null,
  sessionId: string,
): { tabs: SessionTab[]; active: string | null } {
  const index = tabs.findIndex((tab) => tab.sessionId === sessionId);
  if (index < 0) return { tabs, active };
  const next = tabs.filter((tab) => tab.sessionId !== sessionId);
  if (active !== sessionId) return { tabs: next, active };
  if (next.length === 0) return { tabs: next, active: null };
  const left = next[index - 1] ?? next[next.length - 1];
  return { tabs: next, active: left.sessionId };
}

/** Something arrived for a session that is not on screen. */
export function markUnread(tabs: SessionTab[], sessionId: string): SessionTab[] {
  return tabs.map((tab) =>
    tab.sessionId === sessionId ? { ...tab, unread: tab.unread + 1 } : tab,
  );
}

/** It has been read: the tab came to the front. */
export function clearUnread(tabs: SessionTab[], sessionId: string): SessionTab[] {
  return tabs.map((tab) => (tab.sessionId === sessionId ? { ...tab, unread: 0 } : tab));
}
