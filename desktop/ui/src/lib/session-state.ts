// SPDX-License-Identifier: Apache-2.0

import type { SessionState } from "./stores";

/**
 * Which states mean the session is ALIVE — it holds an agent subprocess and
 * takes the next instruction directly.
 *
 * A `Record` over the union rather than a `Set` of strings on purpose: adding a
 * state to the contract makes THIS a compile error, once, instead of leaving
 * four independent positive lists to silently disagree about what "live" means
 * (sesion-que-espera design D6).
 *
 * It lives in its own leaf module, importing only a type, so that everything
 * that needs the answer can reach it — including code the node test runner
 * loads directly, which cannot pull in the store's Tauri bridge.
 */
export const LIVE_STATE: Record<SessionState, boolean> = {
  starting: true,
  active: true,
  waiting_permission: true,
  // Between turns, not over: its agent is running and a directed instruction
  // becomes its next turn without resuming anything.
  waiting_instruction: true,
  ended: false,
  interrupted: false,
};

/** Whether a session is alive. The one question, answered in one place. */
export function isLive(state: SessionState): boolean {
  return LIVE_STATE[state] ?? false;
}
