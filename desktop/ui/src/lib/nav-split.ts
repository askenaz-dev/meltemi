// SPDX-License-Identifier: Apache-2.0
// How the sidebar's height is split between the navigation entries and the
// projects tree. Pure arithmetic, kept out of the component so the bounds are
// driven by an executed test instead of by the browser that happens to run it.

/** Two rows of entries: the floor below which the navigation stops being one. */
export const MIN_NAV_PX = 64;

/** Three rows of tree: enough to see that there is a tree, and to scroll it. */
export const MIN_TREE_PX = 96;

/** One arrow press, matching the skin's --sp-4. */
export const STEP_PX = 16;

/**
 * The navigation height that `desired` resolves to inside `available`.
 *
 * When the bar is too short to satisfy both floors the entries keep theirs and
 * the tree takes what is left, scrolling. That is a decision, not an accident:
 * the navigation is the way back to everything else.
 */
export function clampNavHeight(desired: number, available: number): number {
  const upper = Math.max(MIN_NAV_PX, available - MIN_TREE_PX);
  return Math.min(Math.max(Math.round(desired), MIN_NAV_PX), upper);
}

/** One keyboard step from `current`, clamped the same way a drag is. */
export function stepNavHeight(current: number, delta: number, available: number): number {
  return clampNavHeight(current + delta, available);
}
