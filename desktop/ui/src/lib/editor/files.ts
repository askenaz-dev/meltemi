// SPDX-License-Identifier: Apache-2.0
// Editor data layer: contained reads and project search via the Rust side
// (surface concern), the repo tree via the daemon's `repo/map` (gitignore-
// honoring), and every save via the daemon's `worktree/apply-edit`.

import { invoke } from "@tauri-apps/api/core";
import { request } from "../daemon";

export interface RepoEntry {
  path: string;
  isDir: boolean;
  size: number;
}

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children: TreeNode[];
}

export async function loadTree(root: string): Promise<{
  nodes: TreeNode[];
  truncated: boolean;
}> {
  const result = await request<{ entries: RepoEntry[]; truncated: boolean }>(
    "repo/map",
    { projectRoot: root },
  );
  return { nodes: buildTree(result.entries), truncated: result.truncated };
}

export function buildTree(entries: RepoEntry[]): TreeNode[] {
  const rootNodes: TreeNode[] = [];
  const dirs = new Map<string, TreeNode>();

  const parentOf = (path: string): TreeNode[] => {
    const slash = path.lastIndexOf("/");
    if (slash === -1) return rootNodes;
    const parent = dirs.get(path.slice(0, slash));
    return parent ? parent.children : rootNodes;
  };

  for (const entry of entries) {
    const name = entry.path.split("/").pop() ?? entry.path;
    const node: TreeNode = {
      name,
      path: entry.path,
      isDir: entry.isDir,
      children: [],
    };
    if (entry.isDir) {
      dirs.set(entry.path, node);
    }
    parentOf(entry.path).push(node);
  }
  return rootNodes;
}

export function readFile(root: string, file: string): Promise<{ content: string }> {
  return invoke<{ file: string; content: string }>("tree_read", { root, file });
}

export interface SearchMatch {
  file: string;
  line: number;
  text: string;
}

export function searchProject(
  root: string,
  query: string,
): Promise<{ matches: SearchMatch[]; truncated: boolean }> {
  return invoke("tree_search", { root, query });
}

export function openWith(
  root: string,
  file: string,
  line?: number,
): Promise<string> {
  return invoke("open_with", { root, file, line });
}

export interface SaveTarget {
  change: string;
  task: string;
  agent: string;
}

export interface SaveOutcome {
  file: string;
  bytesWritten: number;
  treeState: "free" | "session_active" | "turn_in_flight";
  loggedTo: "session" | "project";
}

/** Saves through the daemon; throws the structured soft-lock refusal. */
export function saveFile(
  root: string,
  file: string,
  content: string,
  target: SaveTarget | null,
  confirm: boolean,
): Promise<SaveOutcome> {
  return request<SaveOutcome>("worktree/apply-edit", {
    projectRoot: root,
    ...(target ?? {}),
    file,
    content,
    confirm,
  });
}
