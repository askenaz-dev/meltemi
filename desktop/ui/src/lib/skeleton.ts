// SPDX-License-Identifier: Apache-2.0
// The waiting state of a view (senales-de-vida design D3): a skeleton that
// shows up only when the wait is long enough to be seen. A load that answers
// first must not flash a placeholder, so the reveal is delayed and cancelled
// when the data wins the race.

/** Below this a wait is not shown: the answer arrives before the eye notices. */
export const SKELETON_DELAY_MS = 150;

/** Bar widths in percent: fixed, so nothing jitters from one draw to the next. */
const WIDTHS = [72, 48, 64, 36, 56];

/** The widths of `rows` bars, cycling the fixed pattern. */
export function skeletonWidths(rows: number): number[] {
  return Array.from({ length: Math.max(0, Math.floor(rows)) }, (_, index) => {
    return WIDTHS[index % WIDTHS.length];
  });
}

/**
 * Calls `reveal` once `delay` has passed, unless the returned cancel runs
 * first — which is what the data arriving does, by unmounting the skeleton.
 */
export function revealAfter(reveal: () => void, delay = SKELETON_DELAY_MS): () => void {
  const handle = setTimeout(reveal, delay);
  return () => clearTimeout(handle);
}
