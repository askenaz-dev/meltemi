// SPDX-License-Identifier: Apache-2.0
// The webview's only door to the daemon: the `daemon_request` Tauri command
// (RPC passthrough) plus the `daemon:state` / `daemon:incoming` events pushed
// by the bridge actor. No fetch, no sockets (CSP enforces it).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { readonly, writable, type Readable } from "svelte/store";

export type ConnState =
  | { state: "connecting" }
  | {
      state: "connected";
      version: string;
      uptimeSeconds: number;
      sessions: number;
      endpoint: string;
    }
  | { state: "unreachable"; detail: string; endpoint: string };

/**
 * One fleet agent offered as a way out of a resolution refusal (2000/2001).
 * Same vocabulary `fleet/list` publishes, computed by the same detection path,
 * so this list and the Fleet view cannot disagree.
 */
export interface AgentCandidate {
  id: string;
  detected: boolean;
  installState: string;
  remedy?: string;
  remedyCommand?: string;
}

/** The bridge's structured error: contract `ErrorData` or local unreachable. */
export interface DaemonError {
  code: number | null;
  message: string;
  kind: string;
  detail: string | null;
  remedy: string | null;
  /** Present when the refusal was about resolving an agent. */
  candidates?: AgentCandidate[];
}

export interface IncomingMessage {
  method: string;
  params: unknown;
}

/** Sends one JSON-RPC request to the daemon through the bridge. */
export function request<T = unknown>(
  method: string,
  params: unknown = {},
): Promise<T> {
  return invoke<T>("daemon_request", { method, params });
}

const connStore = writable<ConnState>({ state: "connecting" });

/** Live connection state, fed by the `daemon:state` event. */
export const conn: Readable<ConnState> = readonly(connStore);

/** Starts the connection-state listener; returns its unlisten handle. */
export function startConnListener(): Promise<UnlistenFn> {
  return listen<ConnState>("daemon:state", (event) => {
    connStore.set(event.payload);
  });
}

/** Subscribes to daemon-initiated traffic (session events, permissions). */
export function onIncoming(
  handler: (message: IncomingMessage) => void,
): Promise<UnlistenFn> {
  return listen<IncomingMessage>("daemon:incoming", (event) => {
    handler(event.payload);
  });
}
