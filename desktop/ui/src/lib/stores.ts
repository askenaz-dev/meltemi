// SPDX-License-Identifier: Apache-2.0
// Shared view-model stores fed by the daemon bridge: sessions, permissions,
// fleet, project scope and persistent notices (signal discipline: nothing is
// dismissed silently).

import { get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { onIncoming, request } from "./daemon";
// Imported, not only re-exported below: `export … from` creates no local
// binding, so the push handlers in this module need the name in scope.
import { pushNotice } from "./notices";
import { setActiveProject } from "./ui-state";

// ---- contract shapes (camelCase mirror of meltemi-proto) -------------------

export type SessionState =
  | "starting"
  | "active"
  | "waiting_permission"
  | "ended"
  | "interrupted";

export interface SessionInfo {
  sessionId: string;
  agentCommand: string[];
  projectRoot: string;
  state: SessionState;
  level: number;
  finalStatus?: string;
  startedAt: string;
  endedAt?: string;
  resumable: boolean;
  /** The catalog id the agent resolved to, when the resolution named one. */
  agentId?: string;
  /** The launch profile — the subscription — by NAME only, never its env. */
  profile?: string;
  /**
   * What the session is about, derived by the daemon from the instruction that
   * opened it. Absent when no user sentence started the session — a dispatched
   * lane — and for sessions from before titles existed.
   */
  title?: string;
}

/** A known project (`project/list`): the registry, fed by real use. */
export interface ProjectInfo {
  projectKey: string;
  root: string;
  /** False when the root no longer exists on disk; the entry stays listed. */
  exists: boolean;
  firstSeenAt: string;
  lastSeenAt: string;
  sessionsTotal: number;
  resumableSessions: number;
}

export interface PermissionRule {
  effect: string;
  tool?: string;
  commandPrefix?: string;
  pathPrefix?: string;
  scope: string;
}

export interface PendingPermission {
  requestId: string;
  sessionId: string;
  tool: string;
  summary: string;
  options: { optionId: string; name: string; kind?: string }[];
  waitingSeconds: number;
  /** Absent when the wait policy imposes no deadline (waiting for the human). */
  expiresInSeconds?: number;
  expired: boolean;
  suggestedRule?: PermissionRule;
}

export interface FleetLayer {
  kind: "cli" | "adapter";
  bin: string;
  detected: boolean;
  binaryPath?: string;
  evidenceOnly?: boolean;
  install?: string;
  /** The layer travels in Meltemi's own installers; it has no install command. */
  bundled?: boolean;
  /** Where the find came from; absent when the layer was not detected. */
  source?: "path" | "candidate_path" | "bundled";
}

export type FleetInstallState =
  | "ready"
  | "adapter_missing"
  | "cli_missing"
  | "not_detected"
  | "not_launchable";

export interface FleetAgent {
  id: string;
  displayName: string;
  source: "registry" | "custom" | "profile";
  integrationLevel: number;
  verifiedLevel?: number;
  verifiedAt?: string;
  mcpSupport: boolean;
  detected: boolean;
  binaryPath?: string;
  configured: boolean;
  underlyingAgent?: string;
  layers?: FleetLayer[];
  installState?: FleetInstallState;
  remedy?: string;
  remedyCommand?: string;
  legalStatus?: "sanctioned" | "tolerated" | "grey";
  legalNote?: string;
  /** The auth-context variable the registry declares: its presence makes the
      entry a linkable subscription target (vincular-suscripciones). */
  authContextVar?: string;
}

export interface ChangeInfo {
  name: string;
  archived: boolean;
  archivedAt?: string;
  artifacts: Record<string, boolean>;
  tasksDone: number;
  tasksTotal: number;
  reviewDecided: number;
  reviewTotal: number;
  verified: number;
  verifyTotal: number;
  /** Whether an authoring gate awaits a human decision on this change. */
  gatePending: boolean;
  /** The artifact the pending gate is about; absent when none is pending. */
  gateArtifact?: string;
}

export interface SpecInfo {
  capability: string;
  requirements: number;
  scenarios: number;
}

// ---- stores ----------------------------------------------------------------

/** The project every project-scoped call is made against. */
export const activeProject = writable<string | null>(null);
/** The active project's sessions (what the scoped views render). */
export const sessions = writable<SessionInfo[]>([]);
/** Every session of every project — the spine of the sidebar tree (D7). */
export const allSessions = writable<SessionInfo[]>([]);
/** The known projects, most recently used first. */
export const projects = writable<ProjectInfo[]>([]);
export const pending = writable<PendingPermission[]>([]);
export const fleet = writable<FleetAgent[]>([]);
/**
 * The active project's changes. Lifted out of the Project view so the status
 * bar can name the gate that is waiting without opening a view to find it
 * (barra-de-estado-agentica design D4). One source, two readers.
 */
export const changes = writable<ChangeInfo[]>([]);

// Notices live in their own pure module so their policy is driven by an
// executed test; re-exported here because every view already imports them from
// the stores.
export {
  NOTICE_TTL_MS,
  dismissAllNotices,
  dismissNotice,
  holdNotice,
  notices,
  pushNotice,
  releaseNotice,
  type Notice,
} from "./notices";

// ---- project scope -----------------------------------------------------------

/**
 * Resolves the initial project: the persisted active project when it is still
 * set, otherwise the working directory the app was launched in.
 */
export async function initProjectScope(persisted: string | null): Promise<string | null> {
  const cwd = await invoke<string | null>("project_root");
  const root = persisted ?? cwd;
  activeProject.set(root);
  return root;
}

export function switchProject(root: string): void {
  activeProject.set(root);
  setActiveProject(root);
  // The scope changed: renarrow what the views show from the list we hold, so
  // switching is instant, then refetch for the authoritative answer.
  sessions.set(scopedTo(root, get(allSessions)));
  void refreshSessions().catch(() => {});
}

// ---- refreshers --------------------------------------------------------------

/**
 * One unfiltered `session/list` feeds both the tree and the scoped views
 * (design D7): the daemon reports every session with its own root, and the
 * client narrows to the active project — including sessions that ran in a
 * worktree under it.
 */
export async function refreshSessions(): Promise<void> {
  const result = await request<{ sessions: SessionInfo[] }>("session/list", {});
  allSessions.set(result.sessions);
  const root = get(activeProject);
  sessions.set(root ? scopedTo(root, result.sessions) : result.sessions);
}

/** The sessions of one project root, worktree sessions included. */
export function scopedTo(root: string, all: SessionInfo[]): SessionInfo[] {
  const norm = (p: string) => p.replaceAll("\\", "/").replace(/\/+$/, "").toLowerCase();
  const scope = norm(root);
  return all.filter((session) => {
    const own = norm(session.projectRoot);
    return own === scope || own.startsWith(scope + "/");
  });
}

/** Refreshes the known-project registry (the switcher and the tree). */
export async function refreshProjects(): Promise<void> {
  const result = await request<{ projects: ProjectInfo[] }>("project/list", {});
  projects.set(result.projects);
}

/**
 * Registers a directory as a project and answers with the root the daemon
 * stored — its CANONICAL form, which is what the registry lists and what a
 * later forget has to match. Returning the typed path instead would show one
 * spelling and hold another.
 */
export async function registerProject(root: string): Promise<string> {
  const result = await request<{ project: ProjectInfo }>("project/register", { root });
  await refreshProjects().catch(() => {});
  return result.project.root;
}

/**
 * Opens the client's own folder picker and registers what was chosen. The
 * dialog is chrome of this client (design D8): the daemon receives the path and
 * validates it, and never participates in showing the window. A dismissed
 * dialog answers null and nothing is sent anywhere.
 */
export async function pickAndRegisterProject(): Promise<string | null> {
  const picked = await invoke<string | null>("pick_project_folder");
  if (!picked) return null;
  return registerProject(picked);
}

/**
 * Drops a project from the REGISTRY LISTING. Nothing on disk is touched, its
 * sessions keep listing and its logs keep reading, and it comes back the moment
 * it is used or registered again.
 */
export async function forgetProject(root: string): Promise<boolean> {
  const result = await request<{ forgotten: boolean }>("project/forget", { root });
  await refreshProjects().catch(() => {});
  return result.forgotten;
}

/**
 * Refreshes the active project's changes. A directory without `.meltemi/`
 * answers with empty lists, so emptiness is not evidence of absence — the same
 * guard the Project view already applied is kept here, at the one place that
 * asks.
 */
export async function refreshChanges(): Promise<void> {
  const root = get(activeProject);
  if (!root) {
    changes.set([]);
    return;
  }
  try {
    const result = await request<{ changes: ChangeInfo[] }>("change/list", {
      projectRoot: root,
    });
    changes.set(result.changes);
  } catch {
    // A project that cannot answer leaves the bar without a change segment,
    // never with a wrong one.
    changes.set([]);
  }
}

/**
 * The change the bar names: the one asking for a decision, because that is the
 * one a person can act on. `change/list` does not declare an "active" change,
 * so the criterion lives here and is written down rather than guessed at each
 * call site (design D2).
 */
export function gateWaiting(all: ChangeInfo[]): ChangeInfo | undefined {
  return all.find((change) => change.gatePending);
}

export async function refreshPending(): Promise<void> {
  const result = await request<{ pending: PendingPermission[] }>("permission/pending");
  pending.set(result.pending);
}

export async function refreshFleet(): Promise<void> {
  const result = await request<{ agents: FleetAgent[] }>("fleet/list", {
    projectRoot: get(activeProject) ?? undefined,
  });
  fleet.set(result.agents);
}

// ---- daemon-initiated traffic ------------------------------------------------

export interface SessionEventMessage {
  sessionId: string;
  event: { type: string; payload?: unknown };
}

type SessionEventHandler = (message: SessionEventMessage) => void;
const sessionEventHandlers = new Set<SessionEventHandler>();

/** Views (e.g. the session drill-in transcript) subscribe to live events. */
export function onSessionEvent(handler: SessionEventHandler): () => void {
  sessionEventHandlers.add(handler);
  return () => sessionEventHandlers.delete(handler);
}

/**
 * Routes `daemon:incoming` into the stores and asks the OS for attention when
 * a permission lands with the window unfocused (design D4). Call once at app
 * start; the translator is passed in so notices honor the catalog.
 */
export function startIncomingRouter(
  translate: (
    key:
      | "permissions.timeout.notice"
      | "permissions.arrived.notice"
      | "permissions.timeout.unknownTool"
      | "window.title"
      | "window.title.pending",
    vars: Record<string, string>,
  ) => string,
): Promise<() => void> {
  const attention = (count: number) => {
    // The OS title is user-visible text, so it comes from the catalog here and
    // not from the host (§11).
    const title =
      count > 0
        ? translate("window.title.pending", { n: String(count) })
        : translate("window.title", {});
    void invoke("request_attention", { pending: count, title }).catch(() => {});
  };
  return onIncoming((message) => {
    const params = (message.params ?? {}) as Record<string, unknown>;
    switch (message.method) {
      case "permission/changed": {
        const queue = (params.pending as PendingPermission[]) ?? [];
        pending.set(queue);
        attention(queue.length);
        return;
      }
      case "permission/request": {
        // The push is held unanswered on purpose (the tray decides); it is the
        // earliest moment we can reclaim the user's attention.
        pushNotice(
          translate("permissions.arrived.notice", {
            session: String(params.sessionId ?? "?"),
          }),
          "warn",
        );
        void refreshPending()
          .then(() => attention(get(pending).length))
          .catch(() => {});
        return;
      }
      case "permission/timeout": {
        // The contract's timeout carries the session and, when known, the tool
        // call id — never a tool name. The operation is what the client already
        // holds in its queue; look it up BEFORE refreshing drops the entry, and
        // fall back to the id rather than to a question mark.
        const sessionId = String(params.sessionId ?? "");
        const expired = get(pending).find(
          (candidate) => candidate.sessionId === sessionId,
        );
        const tool =
          expired?.tool ??
          (typeof params.toolCallId === "string" && params.toolCallId.length > 0
            ? params.toolCallId
            : translate("permissions.timeout.unknownTool", {}));
        pushNotice(
          translate("permissions.timeout.notice", {
            session: sessionId || "?",
            tool,
          }),
          "warn",
        );
        void refreshPending()
          .then(() => attention(get(pending).length))
          .catch(() => {});
        return;
      }
      case "session/event": {
        const event = message.params as SessionEventMessage;
        for (const handler of sessionEventHandlers) handler(event);
        return;
      }
      default:
        return;
    }
  });
}
