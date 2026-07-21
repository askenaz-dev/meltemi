// SPDX-License-Identifier: Apache-2.0
// The typed RPC-method registry (gui-tauri-paridad design D3): every
// client-invocable method of the daemon contract has an entry here, so the
// palette reaches every capability — with or without a dedicated view. The
// parity gate (desktop/tests/parity.rs) fails the build when a contract
// method is missing from this registry, from the TUI palette or from
// docs/paridad-nucleo.md.

import type { MessageKey } from "./messages";

export type ViewId = "sessions" | "project" | "permissions" | "fleet";

export interface RegistryEntry {
  /** The contract method this entry exercises (parity key). */
  method: string;
  /** Localized one-line description (palette row). */
  descKey: MessageKey;
  /** `request` awaits a result; `notify` is fire-and-forget. */
  kind: "request" | "notify";
  /** JSON template prefilled in the palette's params editor. */
  template?: Record<string, unknown>;
  /** Inject the current project root into this template key when present. */
  injectRoot?: string;
  /** Irreversible / whole-daemon operations require explicit confirmation. */
  dangerous?: boolean;
  /** A dedicated view renders this method's domain (palette offers to open it). */
  view?: ViewId;
}

const R = (
  method: string,
  descKey: MessageKey,
  extra: Partial<RegistryEntry> = {},
): RegistryEntry => ({ method, descKey, kind: "request", ...extra });

/**
 * One entry per client-invocable contract method (39 today; `initialize` is
 * the connection handshake and lives in the bridge). Daemon-initiated traffic
 * (session/event, permission/request, permission/timeout, permission/changed)
 * arrives through the `daemon:incoming` event, not through here.
 */
export const REGISTRY: RegistryEntry[] = [
  R("status", "palette.m.status"),
  R("shutdown", "palette.m.shutdown", { dangerous: true }),
  R("propose", "palette.m.propose", {
    template: { projectRoot: "", idea: "" },
    injectRoot: "projectRoot",
  }),
  R("fleet/list", "palette.m.fleet.list", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
    view: "fleet",
  }),
  R("context/project", "palette.m.context.project", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
  }),
  R("session/list", "palette.m.session.list", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
    view: "sessions",
  }),
  R("session/log", "palette.m.session.log", {
    template: { projectRoot: "", sessionId: "" },
    injectRoot: "projectRoot",
  }),
  R("repo/map", "palette.m.repo.map", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/constitution", "palette.m.sdd.constitution", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/explore", "palette.m.sdd.explore", {
    template: { projectRoot: "", topic: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/propose", "palette.m.sdd.propose", {
    template: { projectRoot: "", idea: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/plan", "palette.m.sdd.plan", {
    template: { projectRoot: "", change: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/gate", "palette.m.sdd.gate", {
    template: { projectRoot: "", sessionId: "", decision: "approve" },
    injectRoot: "projectRoot",
  }),
  R("sdd/review", "palette.m.sdd.review", {
    template: { projectRoot: "", change: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/review-decide", "palette.m.sdd.review-decide", {
    template: { projectRoot: "", change: "", item: 0, decision: "approve" },
    injectRoot: "projectRoot",
  }),
  R("session/cancel", "palette.m.session.cancel", {
    kind: "notify",
    template: { sessionId: "" },
    dangerous: true,
  }),
  R("session/direct", "palette.m.session.direct", {
    template: { projectRoot: "", sessionId: "", instruction: "" },
    injectRoot: "projectRoot",
  }),
  R("permission/pending", "palette.m.permission.pending", {
    view: "permissions",
  }),
  R("permission/decide", "palette.m.permission.decide", {
    template: { requestId: "", optionId: "" },
    view: "permissions",
  }),
  R("worktree/assign", "palette.m.worktree.assign", {
    template: { projectRoot: "", change: "", tasks: [], agents: [] },
    injectRoot: "projectRoot",
  }),
  R("worktree/list", "palette.m.worktree.list", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
  }),
  R("worktree/remove", "palette.m.worktree.remove", {
    template: { projectRoot: "", change: "", task: "", agent: "" },
    injectRoot: "projectRoot",
    dangerous: true,
  }),
  R("worktree/diff", "palette.m.worktree.diff", {
    template: { projectRoot: "", change: "", task: "" },
    injectRoot: "projectRoot",
  }),
  R("worktree/apply-edit", "palette.m.worktree.apply-edit", {
    template: { projectRoot: "", file: "", content: "", confirm: false },
    injectRoot: "projectRoot",
  }),
  R("worktree/merge-file", "palette.m.worktree.merge-file", {
    template: { projectRoot: "", change: "", task: "", agent: "", file: "" },
    injectRoot: "projectRoot",
  }),
  R("worktree/dispatch", "palette.m.worktree.dispatch", {
    template: { projectRoot: "", change: "", task: "", agent: "" },
    injectRoot: "projectRoot",
  }),
  R("checkpoint/create", "palette.m.checkpoint.create", {
    template: { projectRoot: "", change: "", task: "", agent: "" },
    injectRoot: "projectRoot",
  }),
  R("checkpoint/list", "palette.m.checkpoint.list", {
    template: { projectRoot: "", change: "" },
    injectRoot: "projectRoot",
  }),
  R("checkpoint/revert", "palette.m.checkpoint.revert", {
    template: { projectRoot: "", change: "", task: "", agent: "", confirm: false },
    injectRoot: "projectRoot",
    dangerous: true,
  }),
  R("checkpoint/record-op", "palette.m.checkpoint.record-op", {
    template: { projectRoot: "", change: "", task: "", agent: "", operation: "" },
    injectRoot: "projectRoot",
  }),
  R("commit/task", "palette.m.commit.task", {
    template: { projectRoot: "", change: "", task: "", agent: "", title: "", confirm: false },
    injectRoot: "projectRoot",
  }),
  R("sdd/verify", "palette.m.sdd.verify", {
    template: { projectRoot: "", change: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/verify-mark", "palette.m.sdd.verify-mark", {
    template: { projectRoot: "", change: "", scenario: "", note: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/archive", "palette.m.sdd.archive", {
    template: { projectRoot: "", change: "", confirm: false },
    injectRoot: "projectRoot",
    dangerous: true,
  }),
  R("sdd/implement", "palette.m.sdd.implement", {
    template: { projectRoot: "", change: "", agent: "" },
    injectRoot: "projectRoot",
  }),
  R("change/list", "palette.m.change.list", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
    view: "project",
  }),
  R("change/show", "palette.m.change.show", {
    template: { projectRoot: "", change: "" },
    injectRoot: "projectRoot",
  }),
  R("spec/list", "palette.m.spec.list", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
    view: "project",
  }),
  R("spec/show", "palette.m.spec.show", {
    template: { projectRoot: "", capability: "" },
    injectRoot: "projectRoot",
  }),
  R("sdd/validate", "palette.m.sdd.validate", {
    template: { projectRoot: "" },
    injectRoot: "projectRoot",
  }),
];
