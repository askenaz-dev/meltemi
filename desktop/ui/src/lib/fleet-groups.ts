// SPDX-License-Identifier: Apache-2.0
// From the flat fleet listing to rows that can be read: each catalog agent
// followed by the subscriptions linked to it. A pure view of what `fleet/list`
// already returns — nothing is asked of the daemon and nothing is stored.

import type { FleetAgent } from "./stores";

export interface FleetRow {
  agent: FleetAgent;
  /** A subscription row hangs off the agent above it. */
  child: boolean;
  /**
   * The catalog agent this subscription runs, as text. Present on every child;
   * for one whose agent is not in the catalog, this is the identifier it
   * declares rather than a display name.
   */
  belongsTo?: string;
  /** True for a subscription whose declared agent is not in the catalog. */
  orphan?: boolean;
  /** On an agent row: how many subscriptions hang off it. */
  subscriptions?: number;
}

function isProfile(agent: FleetAgent): boolean {
  return agent.source === "profile";
}

/**
 * Agents keep the order they arrive in — the catalog already decided it, and
 * reordering here would be a second opinion. Subscriptions sort by name, the
 * only stable thing the user controls.
 *
 * A subscription whose underlying agent is not in the catalog is NOT dropped:
 * it lands in a final group with the identifier it declares. Discarding it
 * would turn a hand-written configuration into a disappearance with no
 * diagnosis.
 */
export function groupFleet(fleet: FleetAgent[]): FleetRow[] {
  const catalog = fleet.filter((agent) => !isProfile(agent));
  const profiles = fleet.filter(isProfile);
  const known = new Set(catalog.map((agent) => agent.id));

  const byAgent = new Map<string, FleetAgent[]>();
  const orphans: FleetAgent[] = [];
  for (const profile of profiles) {
    const under = profile.underlyingAgent;
    if (under !== undefined && known.has(under)) {
      const list = byAgent.get(under) ?? [];
      list.push(profile);
      byAgent.set(under, list);
    } else {
      orphans.push(profile);
    }
  }
  for (const list of byAgent.values()) {
    list.sort((a, b) => a.displayName.localeCompare(b.displayName));
  }
  orphans.sort((a, b) => a.displayName.localeCompare(b.displayName));

  const rows: FleetRow[] = [];
  for (const agent of catalog) {
    const children = byAgent.get(agent.id) ?? [];
    rows.push({
      agent,
      child: false,
      subscriptions: children.length > 0 ? children.length : undefined,
    });
    for (const child of children) {
      rows.push({ agent: child, child: true, belongsTo: agent.displayName });
    }
  }
  for (const orphan of orphans) {
    rows.push({
      agent: orphan,
      child: true,
      // The identifier it declares, so the row still says what it was aiming at.
      belongsTo: orphan.underlyingAgent ?? "",
      orphan: true,
    });
  }
  return rows;
}
