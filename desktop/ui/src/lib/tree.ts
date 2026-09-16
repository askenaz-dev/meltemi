// SPDX-License-Identifier: Apache-2.0
// Project → Sessions aggregation (multiproyecto-suscripciones design D7). The
// daemon answers one unfiltered `session/list` (every session with its own
// root) and one `project/list` (the known projects); the tree is built here, in
// the client, so no new method exists just to shape a view.

import { isLive } from "./session-state.ts";
import type { ProjectInfo, SessionInfo, SessionState } from "./stores";

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


/**
 * The four buckets a project's sessions fall into, named by what they ask of
 * you rather than by the contract state that produces them
 * (sesiones-en-la-barra design D1).
 */
export type BucketId = "decision" | "instruction" | "working" | "stopped";

/**
 * Signal order: what needs a human decision comes first, what is over comes
 * last. The same criterion as the design system's signal priority, so the bar
 * and the status bar split the fleet the same way.
 *
 * The terminal carries its own copy of this table in Rust — TypeScript and Rust
 * share no function — and a wiring test reads both and fails if they diverge.
 */
export const BUCKET_ORDER: readonly BucketId[] = [
  "decision",
  "instruction",
  "working",
  "stopped",
];

/**
 * Which bucket each contract state falls into.
 *
 * A `Record` over the union, like `LIVE_STATE`: a state added to the contract
 * makes THIS a compile error, once, instead of quietly falling out of every
 * bucket and vanishing from the bar.
 */
export const BUCKET_OF: Record<SessionState, BucketId> = {
  waiting_permission: "decision",
  waiting_instruction: "instruction",
  active: "working",
  starting: "working",
  ended: "stopped",
  interrupted: "stopped",
};

/**
 * How many stopped sessions a bucket draws before it offers the count instead.
 *
 * Only the stopped bucket is capped. The live ones are sessions the user
 * launched and that are asking for something; their number is bounded by
 * practice, not by a rule. What is over accumulates forever, and that is the
 * one that would push the tree off the screen.
 */
export const STOPPED_SHOWN = 5;

/** One bucket of a project node, ready to draw. */
export interface SessionBucket {
  id: BucketId;
  /** The rows to draw, already capped. */
  sessions: SessionInfo[];
  /** How many the bucket holds — the number the header says. */
  total: number;
  /** True when the cap left some out, so "see all {n}" is offered. */
  capped: boolean;
}

/**
 * Splits a project's sessions into the four buckets, in signal order.
 *
 * An empty bucket is not returned at all: a header reading "Working 0" is
 * noise, and the caller should not have to filter it out to avoid drawing it.
 *
 * Order inside a live bucket is the caller's (`groupSessions` hands them over
 * most recent first). The stopped bucket sorts by start time itself, because
 * its cap promises "the most recent" and a promise that depends on the caller
 * having sorted is a promise that breaks the day someone calls it directly.
 */
export function bucketSessions(sessions: SessionInfo[]): SessionBucket[] {
  const held = new Map<BucketId, SessionInfo[]>();
  for (const id of BUCKET_ORDER) held.set(id, []);
  for (const session of sessions) {
    held.get(BUCKET_OF[session.state])?.push(session);
  }
  const buckets: SessionBucket[] = [];
  for (const id of BUCKET_ORDER) {
    const all = held.get(id) as SessionInfo[];
    if (all.length === 0) continue;
    if (id !== "stopped") {
      buckets.push({ id, sessions: all, total: all.length, capped: false });
      continue;
    }
    const recent = [...all].sort((a, b) => b.startedAt.localeCompare(a.startedAt));
    buckets.push({
      id,
      sessions: recent.slice(0, STOPPED_SHOWN),
      total: recent.length,
      capped: recent.length > STOPPED_SHOWN,
    });
  }
  return buckets;
}

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
    if (isLive(session.state)) group.live += 1;
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
