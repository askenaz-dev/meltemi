// SPDX-License-Identifier: Apache-2.0
// The pairing screen's states and what each one offers, as functions a test
// can hold (companero-que-conecta D7). The view paints what these say; the
// rules — a refusal never retries by itself, a changed host key is left only
// on purpose — live here, where they are executed and not described.

import type { ConnState } from "./daemon";

/** Why the far side refused, in the actor's words. */
export type RefusedKind = "hostKeyChanged" | "rejected";

/** What the screen shows while there is no channel. */
export type Pairing =
  /** Never paired, or the destination was forgotten: key, destination, connect. */
  | { kind: "unpaired" }
  /** Dialling, with the attempt count and what the last attempt said. */
  | {
      kind: "connecting";
      destination: string;
      attempt: number;
      detail: string | null;
    }
  /** Refused for good: the actor has stopped, and so does the screen. */
  | {
      kind: "refused";
      destination: string;
      cause: RefusedKind;
      detail: string;
    };

/** One thing the screen offers in a state. The view maps these to handlers. */
export type Action =
  | "copyKey"
  | "connect"
  | "cancel"
  | "retry"
  | "forgetHost"
  | "stay";

/**
 * The pairing state for what the surface knows, or `null` once connected —
 * at which point the four verbs take the screen and this module has nothing
 * to say.
 *
 * `attempts` is how many times the actor reported `unreachable` since the
 * surface asked it to connect; the surface counts them, because the actor's
 * backoff is its own business and the honest thing to show is the count and
 * what the last attempt said (the mock-up's "intento N", without inventing a
 * timer the surface does not own).
 */
export function pairing(
  remembered: string | null,
  state: ConnState,
  attempts: number,
): Pairing | null {
  if (state.state === "connected") return null;
  if (state.state === "refused") {
    return {
      kind: "refused",
      destination: state.endpoint,
      cause: state.kind,
      detail: state.detail,
    };
  }
  if (!remembered) return { kind: "unpaired" };
  return {
    kind: "connecting",
    destination: remembered,
    attempt: Math.max(1, attempts + (state.state === "connecting" ? 1 : 0)),
    detail: state.state === "unreachable" ? state.detail : null,
  };
}

/**
 * What each state offers, and in the order the thumb meets them. The last
 * entry is the one under the thumb, so the destructive choice — forgetting a
 * host's key — is never last (mobile-nav's rule, applied here too).
 */
export function actions(state: Pairing): Action[] {
  switch (state.kind) {
    case "unpaired":
      return ["copyKey", "connect"];
    case "connecting":
      return ["cancel"];
    case "refused":
      return state.cause === "hostKeyChanged"
        ? ["forgetHost", "stay"]
        : ["copyKey", "retry"];
  }
}

/**
 * Whether a typed destination is worth sending. The Rust side is the one
 * that parses — `user@host[:port]`, with the bracket rule for IPv6 — and it
 * refuses with the shape it wanted; this only keeps an empty or user-less
 * string from being sent at all.
 */
export function worthSending(typed: string): boolean {
  const text = typed.trim();
  const at = text.indexOf("@");
  return at > 0 && at < text.length - 1 && !/\s/.test(text);
}

/** Whether a state is one the actor retries on its own. Only one is. */
export function retriesByItself(state: Pairing): boolean {
  return state.kind === "connecting";
}
