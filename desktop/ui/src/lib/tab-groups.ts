// SPDX-License-Identifier: Apache-2.0
// Tab groups: which tabs belong together, under what name and colour, and what
// happens when one is collapsed. Pure — the rules here decide whether someone's
// unsent work stays reachable, so they are driven by an executed test.

export interface TabGroup {
  id: string;
  name: string;
  /** A design-system token name, never a colour literal. */
  color: string;
  collapsed: boolean;
  /** Tab ids, in the order they joined. */
  members: string[];
}

export interface GroupState {
  groups: TabGroup[];
}

/** The tokens a group may take. Not a new palette: the skin's own tints. */
export const GROUP_COLORS = ["ok", "warn", "danger", "info"] as const;

export const EMPTY_GROUPS: GroupState = { groups: [] };

/** The colour of the next group, cycling so two neighbours rarely match. */
export function nextColor(state: GroupState): string {
  return GROUP_COLORS[state.groups.length % GROUP_COLORS.length];
}

function withoutMember(state: GroupState, tabId: string): GroupState {
  // A group that loses its last tab stops existing: a name with nothing behind
  // it is something the user would have to clean up by hand.
  const groups = state.groups
    .map((g) => ({ ...g, members: g.members.filter((m) => m !== tabId) }))
    .filter((g) => g.members.length > 0);
  return { groups };
}

/** The group a tab belongs to, or null. A tab belongs to at most one. */
export function groupOf(state: GroupState, tabId: string): TabGroup | null {
  return state.groups.find((g) => g.members.includes(tabId)) ?? null;
}

/** Creates a group holding exactly this tab, removing it from any other. */
export function createGroup(state: GroupState, tabId: string, name: string): GroupState {
  const pruned = withoutMember(state, tabId);
  const id = `g${pruned.groups.length}-${tabId}`;
  return {
    groups: [
      ...pruned.groups,
      { id, name, color: nextColor(pruned), collapsed: false, members: [tabId] },
    ],
  };
}

/** Moves a tab into an existing group. Unknown group ids change nothing. */
export function joinGroup(state: GroupState, tabId: string, groupId: string): GroupState {
  if (!state.groups.some((g) => g.id === groupId)) return state;
  const pruned = withoutMember(state, tabId);
  return {
    groups: pruned.groups.map((g) =>
      g.id === groupId ? { ...g, members: [...g.members, tabId] } : g,
    ),
  };
}

/** Takes a tab out of whatever group holds it. The tab itself stays open. */
export function leaveGroup(state: GroupState, tabId: string): GroupState {
  return withoutMember(state, tabId);
}

/** Forgets a tab entirely — what a close means for the group model. */
export function forgetTab(state: GroupState, tabId: string): GroupState {
  return withoutMember(state, tabId);
}

export function renameGroup(state: GroupState, groupId: string, name: string): GroupState {
  return {
    groups: state.groups.map((g) => (g.id === groupId ? { ...g, name } : g)),
  };
}

/**
 * Collapses or expands a group, and says which tab should be active afterwards.
 *
 * Collapsing NEVER closes a tab: the panels stay mounted and the drafts stay
 * put. But the active tab may not stay inside a collapsed group — a panel on
 * screen whose tab is not would be the surface lying about where the user is —
 * so the activity moves to the first tab outside it, or to `null`, the list.
 */
export function setCollapsed(
  state: GroupState,
  groupId: string,
  collapsed: boolean,
  active: string | null,
  order: string[],
): { state: GroupState; active: string | null } {
  const next: GroupState = {
    groups: state.groups.map((g) => (g.id === groupId ? { ...g, collapsed } : g)),
  };
  if (!collapsed || active === null) return { state: next, active };
  const group = next.groups.find((g) => g.id === groupId);
  if (!group || !group.members.includes(active)) return { state: next, active };
  const hidden = new Set(
    next.groups.filter((g) => g.collapsed).flatMap((g) => g.members),
  );
  const visible = order.find((id) => !hidden.has(id)) ?? null;
  return { state: next, active: visible };
}

/** The tabs a collapsed group is holding out of sight. */
export function hiddenTabs(state: GroupState): Set<string> {
  return new Set(state.groups.filter((g) => g.collapsed).flatMap((g) => g.members));
}
