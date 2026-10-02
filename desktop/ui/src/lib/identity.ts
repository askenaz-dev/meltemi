// SPDX-License-Identifier: Apache-2.0
// What the chrome shows about the user's mesh identity, decided here rather than
// inside a component, so the rule is driven by an executed test instead of by
// whoever opens the sidebar (identidad-propia design D6, D7).

/** Whether a machine of the mesh is reachable right now. */
export type MeshPresence = "online" | "offline";

/** Why identity could not be read; three reasons, three different remedies. */
export type IdentityUnavailable = "not_configured" | "client_missing" | "not_linked";

/** One machine of the user's mesh, as the mesh's own client reports it. */
export interface MeshMachine {
  name: string;
  presence: MeshPresence;
  system?: string;
  lastSeen?: string;
  thisMachine: boolean;
}

/** The daemon's answer to `identity/status`. */
export interface IdentityStatus {
  available: boolean;
  reason?: IdentityUnavailable;
  remedy?: string;
  user?: string;
  provider?: string;
  this?: string;
  machines: MeshMachine[];
}

/**
 * What the sidebar's foot shows, if anything.
 *
 * `hidden` is the default and the important one: a permanent invitation to link
 * in the chrome of somebody who never wanted identity would be asking for
 * something, and this change asks for nothing.
 */
export type FootIdentity =
  | { kind: "hidden" }
  | { kind: "link" }
  | { kind: "attention" }
  | { kind: "who"; user: string; machines: number };

/**
 * The foot's identity row for `status`.
 *
 * Absent when identity is off, which includes "the daemon has not answered yet":
 * a row that appeared a moment after the window opened would be chrome that
 * flickers, and nothing here is urgent.
 */
export function footIdentity(status: IdentityStatus | null): FootIdentity {
  if (!status) return { kind: "hidden" };
  if (status.available) {
    // The login name is the point; without one there is nothing to show that a
    // person would recognise as themselves.
    const user = status.user ?? status.this;
    if (!user) return { kind: "attention" };
    return { kind: "who", user, machines: status.machines.length };
  }
  switch (status.reason) {
    // Not configured is the one case where the chrome does not change at all.
    case "not_configured":
      return { kind: "hidden" };
    // Configured, and this machine has no session: the one offer this change
    // makes, and it leads to Settings rather than doing anything.
    case "not_linked":
      return { kind: "link" };
    // Configured, and the client cannot be read. Saying "link this machine"
    // would be false — there is nothing to run — so the row leads to the
    // diagnosis instead of inventing a remedy.
    case "client_missing":
      return { kind: "attention" };
    default:
      return { kind: "hidden" };
  }
}

/**
 * The presence symbol of the design system's vocabulary.
 *
 * Always rendered beside its word, never alone and never as colour alone: the
 * same rule the terminal follows with its ASCII twin.
 */
export function presenceGlyph(presence: MeshPresence): string {
  return presence === "online" ? "●" : "○";
}

/**
 * The machines in the order a person wants them: this one first, then by name.
 *
 * The daemon already answers in this order. Sorting again here is not mistrust
 * of the daemon but independence from it: this surface must not depend on an
 * ordering the contract never promised.
 */
export function orderedMachines(machines: MeshMachine[]): MeshMachine[] {
  return [...machines].sort((a, b) => {
    if (a.thisMachine !== b.thisMachine) return a.thisMachine ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}
