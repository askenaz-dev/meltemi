// SPDX-License-Identifier: Apache-2.0
// Whether the transcript says a turn is in flight, and where
// (senales-de-vida design D4). A turn of the agent is born with its first
// fragment, so between the instruction and that fragment the conversation has
// nothing to show; and a turn that a permission request interrupted keeps
// `closed: false` for good. Neither the items nor `closed` answer alone: the
// answer needs whether the agent is working right now, which the caller knows
// from the session's state.

import type { ConversationItem } from "./conversation";

export type LifeSignal =
  | { kind: "none" }
  /** The agent works and has said nothing yet; timed from the instruction. */
  | { kind: "waiting"; since: string | null }
  /** The agent works and this turn is where its words are arriving. */
  | { kind: "streaming"; itemId: number };

/** What the transcript shows of a turn in flight, given the agent's state. */
export function lifeSignal(items: ConversationItem[], working: boolean): LifeSignal {
  if (!working) return { kind: "none" };
  // An instruction queued behind the turn is not what the agent is doing.
  const settled = items.filter((item) => !(item.kind === "human" && item.pending));
  const last = settled[settled.length - 1];
  if (last?.kind === "agent") {
    // An open turn is where the words arrive; a closed one means the turn
    // just ended and the session's state has not caught up yet.
    return last.closed ? { kind: "none" } : { kind: "streaming", itemId: last.id };
  }
  // A turn that closed empty, or a card still waiting on the user: nothing is
  // running, whatever a lagging state says.
  if (last?.kind === "empty" || (last?.kind === "permission" && last.decided === null)) {
    return { kind: "none" };
  }
  const prompt = [...settled].reverse().find((item) => item.kind === "human");
  return { kind: "waiting", since: prompt?.ts ?? null };
}

/** `m:ss`, and `h:mm:ss` past the hour; a clock never runs backwards. */
export function elapsedClock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  if (hours > 0) return `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`;
  return `${minutes}:${seconds}`;
}
