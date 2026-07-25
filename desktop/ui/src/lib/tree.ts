// SPDX-License-Identifier: Apache-2.0
// Project → Sessions aggregation (multiproyecto-suscripciones design D7). The
// daemon answers one unfiltered `session/list` (every session with its own
// root) and one `project/list` (the known projects); the tree is built here, in
// the client, so no new method exists just to shape a view.

import type { ProjectInfo, SessionInfo } from "./stores";

/** One project node of the tree, with the sessions that ran inside it. */
export interface ProjectGroup {
  /** The project root as the daemon reported it (the switch key). */
  root: string;
  /** Last path segment — what the sidebar shows. */
  name: string;
  /** False when the root no longer exists on disk (kept, never dropped). */
  exists: boolean;
  sessions: SessionInfo[];
  /** Sessions currently running (starting, active, waiting on a permission). */
  live: number;
  /**
   * True when this node was inferred from sessions alone — a root that is not
   * in the project registry. Surfacing it beats hiding sessions.
   */
  inferred: boolean;
}

const LIVE = new Set(["starting", "active", "waiting_permission"]);

/** Separator-normalized, trailing-slash-free form for comparison. */
function normalize(path: string): string {
  const slashed = path.replaceAll("\\", "/").replace(/\/+$/, "");
  // Case-insensitive: Windows is a first-class platform and its roots differ
  // only in case. The cost on a case-sensitive filesystem is a rare misgroup,
  // never a lost session.
  return slashed.toLowerCase();
}

/** The last segment of a path (the display name). */
export function projectName(root: string): string {
  const parts = root.replaceAll("\\", "/").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? root;
}

/** Whether `child` is `parent` or lives under it, at a segment boundary. */
function isUnder(child: string, parent: string): boolean {
  return child === parent || child.startsWith(parent + "/");
}

/**
 * Groups sessions under their project. A session started in a worktree lives
 * under the repository root, so the longest matching project wins; a session
 * whose root matches no known project becomes its own inferred node.
 */
export function groupSessions(
  projects: ProjectInfo[],
  sessions: SessionInfo[],
): ProjectGroup[] {
  const groups = new Map<string, ProjectGroup>();
  const order: string[] = [];

  const add = (root: string, exists: boolean, inferred: boolean): ProjectGroup => {
    const key = normalize(root);
    let group = groups.get(key);
    if (!group) {
      group = { root, name: projectName(root), exists, sessions: [], live: 0, inferred };
      groups.set(key, group);
      order.push(key);
    }
    return group;
  };

  // The registry order is recency order; keep it, empty projects included.
  for (const project of projects) add(project.root, project.exists, false);

  const roots = [...groups.keys()].sort((a, b) => b.length - a.length);
  for (const session of sessions) {
    const root = normalize(session.projectRoot);
    const match = roots.find((candidate) => isUnder(root, candidate));
    const group = match
      ? (groups.get(match) as ProjectGroup)
      : add(session.projectRoot, true, true);
    group.sessions.push(session);
    if (LIVE.has(session.state)) group.live += 1;
  }

  for (const group of groups.values()) {
    group.sessions.sort((a, b) => b.startedAt.localeCompare(a.startedAt));
  }
  // Inferred nodes last: the registry is the spine of the tree.
  const nodes = order.map((key) => groups.get(key) as ProjectGroup);
  return [...nodes.filter((n) => !n.inferred), ...nodes.filter((n) => n.inferred)];
}

/** The subscription label of a session: the profile, else nothing to claim. */
export function subscriptionOf(session: SessionInfo): string | null {
  return session.profile ?? null;
}

/**
 * The agent identity of a session: the resolved catalog id when the daemon
 * recorded one, else the binary name of its command — never a guess dressed
 * up as a catalog id.
 */
export function agentLabelOf(session: SessionInfo): string {
  if (session.agentId) return session.agentId;
  const program = session.agentCommand[0] ?? "";
  const leaf = program.replaceAll("\\", "/").split("/").pop() ?? program;
  return leaf.replace(/\.(exe|cmd|bat|ps1)$/i, "") || "?";
}
