// SPDX-License-Identifier: Apache-2.0
// What the companion's bottom navigation is, and what each destination is for.
// Here rather than inside the component so the subset rule — four verbs, no
// fifth — is something a test can hold (companero-movil design D5, D6).

import type { IconName } from "./icons";

/** The four verbs of the Agent Boss, and nothing else. */
export type MobileView = "sessions" | "permissions" | "review" | "settings";

/** One destination of the bottom navigation. */
export interface Destination {
  id: MobileView;
  icon: IconName;
  /** The message key for its label, which is never hardcoded text. */
  label: string;
}

/**
 * The destinations, in the order a thumb meets them.
 *
 * Four, and the count is the point: `mobile-companion` limits this surface to
 * monitor, approve, review and direct, and widening it needs a foundational
 * amendment — not a fifth entry added here one afternoon. Directing is not a
 * destination because it is something you do to a session you are looking at,
 * not a place you go.
 */
export const DESTINATIONS: Destination[] = [
  { id: "sessions", icon: "sessions", label: "mobile.nav.sessions" },
  { id: "permissions", icon: "permissions", label: "mobile.nav.permissions" },
  { id: "review", icon: "diff", label: "mobile.nav.review" },
  { id: "settings", icon: "settings", label: "mobile.nav.settings" },
];

/**
 * Whether this surface may author content.
 *
 * A function rather than a comment, so the answer is executed. The spec forbids
 * authoring of code or specs here; adopting or reverting under explicit
 * confirmation is a governed decision and not editing, which is the distinction
 * the whole screen rests on.
 */
export function mayAuthor(): boolean {
  return false;
}

/** The minimum touch target, from the design system's own token. */
export const TOUCH_MIN_PX = 44;

/**
 * Whether a destructive action may sit in the thumb's resting place.
 *
 * The mock-up fixed this before the app existed: on a phone the risk is the
 * thumb, so the destructive action never sits where it falls by default.
 */
export function destructiveMayRestUnderThumb(): boolean {
  return false;
}
