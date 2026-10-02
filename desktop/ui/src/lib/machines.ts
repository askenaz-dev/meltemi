// SPDX-License-Identifier: Apache-2.0
// What the surface shows about the user's machines, decided here rather than
// inside a component so the rules are driven by an executed test
// (maquinas-remotas design D4, D6).

import type { IdentityStatus, MeshPresence } from "./identity";

/** One machine, as the native side reports it. */
export interface MachineRow {
  name: string;
  /** What the user wrote, which is what they recognise. */
  configured: string;
  /** What is actually dialed. */
  endpoint: string;
  /** `ssh my-pc`, when the way in is a command. */
  via?: string | null;
  /** Whether the client is talking to this one right now. */
  active: boolean;
}

/** The machines of this user, with whatever could not be read. */
export interface MachineList {
  machines: MachineRow[];
  diagnostics: string[];
}

/** A machine as the list draws it, presence included when the mesh knows it. */
export interface ShownMachine extends MachineRow {
  /**
   * What the mesh says about a machine of the same name, when it says anything.
   *
   * Presence, never a promise about a daemon: a machine being reachable on the
   * mesh says nothing about `meltemid` running there, and the surface must not
   * let one be read as the other.
   */
  presence?: MeshPresence;
}

/**
 * Crosses the configured machines with the mesh's, by name.
 *
 * By name because that is the only thing the two lists share — the mesh knows
 * nothing about endpoints and the configuration knows nothing about the mesh —
 * and when they do not cross, nothing is invented: the row is shown without
 * presence rather than guessed at by suffix or by position.
 */
export function withPresence(
  machines: MachineRow[],
  identity: IdentityStatus | null,
): ShownMachine[] {
  const mesh = new Map<string, MeshPresence>();
  if (identity?.available) {
    for (const machine of identity.machines) {
      mesh.set(machine.name.toLowerCase(), machine.presence);
    }
  }
  return machines.map((machine) => {
    const presence = mesh.get(machine.name.toLowerCase());
    return presence ? { ...machine, presence } : { ...machine };
  });
}

/** What the status bar says about the machine in use, if anything. */
export function activeMachine(machines: MachineRow[]): string | null {
  return machines.find((machine) => machine.active)?.name ?? null;
}

/**
 * Whether the chrome should show anything about machines at all.
 *
 * One machine is not a fleet: a chrome segment naming the only computer the user
 * has is noise, and the status bar already says it is connected.
 */
export function worthShowing(machines: MachineRow[]): boolean {
  return machines.length > 1;
}

/** What a connection test found. */
export interface ProbeResult {
  answered: boolean;
  version?: string | null;
  millis: number;
  detail?: string | null;
}

/** One line for a probe's result, in the surface's words. */
export function probeLine(
  probe: ProbeResult,
  words: { answered: string; failed: string },
): string {
  if (probe.answered) {
    return `${words.answered} · ${probe.version ?? "?"} · ${probe.millis} ms`;
  }
  return `${words.failed}${probe.detail ? `: ${probe.detail}` : ""}`;
}
