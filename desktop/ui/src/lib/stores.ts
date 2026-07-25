// SPDX-License-Identifier: Apache-2.0
// Shared view-model stores fed by the daemon bridge: sessions, permissions,
// fleet, project scope and persistent notices (signal discipline: nothing is
// dismissed silently).

import { get, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { onIncoming, request } from "./daemon";
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
  expiresInSeconds: number;
  expired: boolean;
  suggestedRule?: PermissionRule;
}

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
}

export interface SpecInfo {
  capability: string;
  requirements: number;
  scenarios: number;
}

// ---- stores ----------------------------------------------------------------

/** The project every project-scoped call is made against. */
export const activeProject = writable<string | null>(null);
export const sessions = writable<SessionInfo[]>([]);
export const pending = writable<PendingPermission[]>([]);
export const fleet = writable<FleetAgent[]>([]);

/** Persistent notices (permission expiries, session errors): never silent. */
export interface Notice {
  id: number;
  text: string;
  tone: "warn" | "danger" | "info";
  /** Unix ms, for the relative timestamp. */
  at: number;
}
export const notices = writable<Notice[]>([]);
let noticeSeq = 0;

export function pushNotice(text: string, tone: Notice["tone"] = "warn"): void {
  noticeSeq += 1;
  notices.update((all) => [...all, { id: noticeSeq, text, tone, at: Date.now() }]);
}

export function dismissNotice(id: number): void {
  notices.update((all) => all.filter((n) => n.id !== id));
}

export function dismissAllNotices(): void {
  notices.set([]);
}

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
}

// ---- refreshers --------------------------------------------------------------

export async function refreshSessions(): Promise<void> {
  const result = await request<{ sessions: SessionInfo[] }>("session/list", {
    projectRoot: get(activeProject) ?? undefined,
  });
  sessions.set(result.sessions);
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
    key: "permissions.timeout.notice" | "permissions.arrived.notice",
    vars: Record<string, string>,
  ) => string,
): Promise<() => void> {
  const attention = (count: number) => {
    void invoke("request_attention", { pending: count }).catch(() => {});
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
        pushNotice(
          translate("permissions.timeout.notice", {
            session: String(params.sessionId ?? "?"),
            tool: String(params.tool ?? "?"),
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
