// SPDX-License-Identifier: Apache-2.0
// What deserves the user's attention, and what may be said outside the window.
//
// Pure on purpose (avisos-de-escritorio design D2, D3): the decision is a
// function of the transition, not of the state, so a repaint cannot ring twice
// and an executed test can hold the rule. Nothing here talks to the OS.

/** The three moments a person is actually needed. */
export type Reason = "permission" | "gate" | "session";

export interface Moment {
  reason: Reason;
  /** How many of them, for the moments that can arrive in a burst. */
  count: number;
  /** What the moment is about — a change name, a session's title. Never turn text. */
  subject?: string;
}

/** What the surface should do about a moment, if anything. */
export interface Attention {
  /** The window title while the moment stands. */
  title: string;
  /** The notice to raise, or null when there is nothing new to say. */
  notice: { title: string; body: string } | null;
}

/**
 * The last moment announced, so the same one is not announced twice. A burst of
 * permissions from one session is one moment with a count, not five notices —
 * ringing again for what is already ringing is ringing in vain.
 */
export interface Announced {
  reason: Reason;
  count: number;
  subject?: string;
}

/** Whether a moment is new, or the same one the user was already told about. */
export function isNew(moment: Moment, last: Announced | null): boolean {
  if (moment.count === 0) return false;
  if (!last) return true;
  return (
    last.reason !== moment.reason ||
    last.subject !== moment.subject ||
    last.count !== moment.count
  );
}

/**
 * Decides what to say. `focused` is the whole gate for the notice: with the
 * window in front, the tray and the states already on screen ARE the notice,
 * and doing both is doing neither well.
 *
 * `translate` is the surface's catalogue (§11): the host never composes prose.
 *
 * What travels is the reason and the count. Never the instruction, never the
 * agent's answer — the window manager and the OS notification centre keep that
 * text outside the governed log, and an instruction can carry exactly what must
 * not leave the repository (design D3).
 */
export function decide(
  moment: Moment,
  opts: {
    focused: boolean;
    last: Announced | null;
    enabled: boolean;
    translate: (key: string, vars?: Record<string, string>) => string;
  },
): Attention {
  const { focused, last, enabled, translate } = opts;
  const title =
    moment.count > 0
      ? translate("window.title.pending", { n: String(moment.count) })
      : translate("window.title", {});

  if (focused || !enabled || !isNew(moment, last)) {
    return { title, notice: null };
  }
  return {
    title,
    notice: {
      title: translate(`attention.${moment.reason}` as string, {
        n: String(moment.count),
      }),
      body: moment.subject ?? translate("attention.body", {}),
    },
  };
}
