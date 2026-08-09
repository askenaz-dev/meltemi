// SPDX-License-Identifier: Apache-2.0
// The strip's own numbers, out of the CSS so a test can read them.

/**
 * The narrowest a tab may become before the strip starts scrolling instead of
 * shrinking. Chosen to hold the state mark and a few characters of the name: a
 * tab that says nothing is not a tab, it is an unlabelled button.
 */
export const MIN_TAB_PX = 96;

/** The widest a tab gets when there is room to spare. */
export const MAX_TAB_PX = 240;

/** How far one press of an overflow control moves the strip. */
export const SCROLL_STEP_PX = 200;

/** Whether the strip overflows, given what it can show and what it holds. */
export function overflows(visibleWidth: number, contentWidth: number): boolean {
  // A pixel of slack: sub-pixel layout rounding must not raise the controls on
  // a strip that actually fits.
  return contentWidth - visibleWidth > 1;
}

/** Whether each control can still do something, given the current offset. */
export function canScroll(
  offset: number,
  visibleWidth: number,
  contentWidth: number,
): { left: boolean; right: boolean } {
  return {
    left: offset > 1,
    right: contentWidth - (offset + visibleWidth) > 1,
  };
}
