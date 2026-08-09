// SPDX-License-Identifier: Apache-2.0
// The notice policy: which notices retire on their own and which never do.
// A pure module — only the store library — because the line between the two is
// a spec obligation (a timeout that expires or an operation that fails must not
// be discarded silently) and obligations are driven by an executed test.

import { writable } from "svelte/store";

export interface Notice {
  id: number;
  text: string;
  tone: "warn" | "danger" | "info";
  /** Unix ms, for the relative timestamp. */
  at: number;
}

export const notices = writable<Notice[]>([]);

/**
 * How long an INFORMATIONAL notice stays. Warnings and errors have no such
 * number, by rule rather than by taste.
 */
export const NOTICE_TTL_MS = 6000;

let noticeSeq = 0;
/** Live timers, by notice id, so a dismissal cancels the one that would fire. */
const timers = new Map<number, ReturnType<typeof setTimeout>>();

/**
 * Timers live here rather than in the component: the notices component unmounts
 * on a view change, and a timer that went with it would leave the notice up
 * forever for the sole reason that someone navigated.
 */
function schedule(id: number): void {
  cancel(id);
  timers.set(
    id,
    setTimeout(() => {
      timers.delete(id);
      notices.update((all) => all.filter((n) => n.id !== id));
    }, NOTICE_TTL_MS),
  );
}

function cancel(id: number): void {
  const timer = timers.get(id);
  if (timer !== undefined) {
    clearTimeout(timer);
    timers.delete(id);
  }
}

export function pushNotice(text: string, tone: Notice["tone"] = "warn"): void {
  noticeSeq += 1;
  const id = noticeSeq;
  notices.update((all) => [...all, { id, text, tone, at: Date.now() }]);
  // Only the informational tone gets a clock.
  if (tone === "info") schedule(id);
}

export function dismissNotice(id: number): void {
  cancel(id);
  notices.update((all) => all.filter((n) => n.id !== id));
}

export function dismissAllNotices(): void {
  for (const timer of timers.values()) clearTimeout(timer);
  timers.clear();
  notices.set([]);
}

/** Holds a transient notice open while it is being read. */
export function holdNotice(id: number): void {
  cancel(id);
}

/**
 * Restarts the clock — restarts rather than resumes, so a notice read again
 * lasts as long as reading it takes. Never mints a clock for a tone that has
 * none.
 */
export function releaseNotice(id: number, tone: Notice["tone"]): void {
  if (tone === "info") schedule(id);
}
