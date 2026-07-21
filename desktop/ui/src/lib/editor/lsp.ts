// SPDX-License-Identifier: Apache-2.0
// BYO-LSP webview side: talks to the Rust LSP hub (detection, spawn, framing)
// through Tauri commands and the `lsp:diagnostics` event. Without a server,
// everything here no-ops and the editor stays a highlighted text editor.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Text } from "@codemirror/state";
import type { Diagnostic } from "@codemirror/lint";
import type { CompletionSource } from "@codemirror/autocomplete";
import { lspLanguageFor } from "./cm";

interface LspPosition {
  line: number;
  character: number;
}

interface LspRange {
  start: LspPosition;
  end: LspPosition;
}

export interface LspDiagnostic {
  range: LspRange;
  message: string;
  severity?: number;
}

interface PublishParams {
  uri: string;
  diagnostics: LspDiagnostic[];
}

export function lspEnsure(
  root: string,
  language: string,
): Promise<string | null> {
  return invoke<string | null>("lsp_ensure", { root, language }).catch(
    () => null,
  );
}

export function lspNotifyOpen(
  root: string,
  file: string,
  language: string,
  content: string,
): Promise<void> {
  return invoke("lsp_open", { root, file, language, content });
}

export function lspNotifyChange(
  root: string,
  file: string,
  content: string,
): Promise<void> {
  const language = lspLanguageFor(file);
  if (!language) return Promise.resolve();
  return invoke("lsp_change", { root, file, language, content });
}

type DiagnosticsHandler = (raw: LspDiagnostic[]) => void;
const handlers = new Map<string, DiagnosticsHandler>();
let listenerStarted = false;

function uriTail(uri: string): string {
  return decodeURIComponent(uri).replaceAll("\\", "/").toLowerCase();
}

/** Subscribes a file to its published diagnostics (raw LSP shape). */
export function onLspDiagnostics(
  file: string,
  handler: DiagnosticsHandler,
): () => void {
  if (!listenerStarted) {
    listenerStarted = true;
    void listen<PublishParams | null>("lsp:diagnostics", (event) => {
      const payload = event.payload;
      if (!payload) return;
      const tail = uriTail(payload.uri);
      for (const [registered, callback] of handlers) {
        if (tail.endsWith(registered.toLowerCase())) {
          callback(payload.diagnostics ?? []);
        }
      }
    });
  }
  handlers.set(file, handler);
  return () => {
    handlers.delete(file);
  };
}

/** Maps raw LSP diagnostics onto CodeMirror positions for the current doc. */
export function toCmDiagnostics(
  doc: Text,
  raw: LspDiagnostic[],
): Diagnostic[] {
  const offset = (position: LspPosition): number => {
    const line = doc.line(Math.min(position.line + 1, doc.lines));
    return Math.min(line.from + position.character, line.to);
  };
  return raw.map((diagnostic) => ({
    from: offset(diagnostic.range.start),
    to: Math.max(
      offset(diagnostic.range.start),
      offset(diagnostic.range.end),
    ),
    severity:
      diagnostic.severity === 1
        ? "error"
        : diagnostic.severity === 2
          ? "warning"
          : "info",
    message: diagnostic.message,
  }));
}

interface CompletionItem {
  label: string;
  insertText?: string;
  detail?: string;
  kind?: number;
}

/** A CodeMirror completion source backed by the BYO server. */
export function makeCompletionSource(
  root: string,
  file: string,
): CompletionSource {
  const language = lspLanguageFor(file);
  return async (context) => {
    if (!language) return null;
    const word = context.matchBefore(/[\w.]*/);
    if (!context.explicit && (!word || word.from === word.to)) return null;
    const line = context.state.doc.lineAt(context.pos);
    let result: unknown;
    try {
      result = await invoke("lsp_request", {
        root,
        file,
        language,
        method: "textDocument/completion",
        params: {
          position: { line: line.number - 1, character: context.pos - line.from },
        },
      });
    } catch {
      return null;
    }
    const items: CompletionItem[] = Array.isArray(result)
      ? (result as CompletionItem[])
      : ((result as { items?: CompletionItem[] })?.items ?? []);
    if (items.length === 0) return null;
    return {
      from: word?.from ?? context.pos,
      options: items.slice(0, 80).map((item) => ({
        label: item.label,
        apply: item.insertText ?? item.label,
        detail: item.detail,
      })),
    };
  };
}

export interface DefinitionTarget {
  file: string;
  line: number;
}

/** `textDocument/definition` → a target inside the tree, when one comes back. */
export async function gotoDefinition(
  root: string,
  file: string,
  position: { line: number; character: number },
): Promise<DefinitionTarget | null> {
  const language = lspLanguageFor(file);
  if (!language) return null;
  let result: unknown;
  try {
    result = await invoke("lsp_request", {
      root,
      file,
      language,
      method: "textDocument/definition",
      params: { position },
    });
  } catch {
    return null;
  }
  const first = Array.isArray(result) ? result[0] : result;
  const location = first as {
    uri?: string;
    targetUri?: string;
    range?: LspRange;
    targetRange?: LspRange;
  } | null;
  const uri = location?.uri ?? location?.targetUri;
  const range = location?.range ?? location?.targetRange;
  if (!uri || !range) return null;

  const targetPath = decodeURIComponent(uri)
    .replace(/^file:\/\/\/?/, "")
    .replaceAll("\\", "/")
    .replace(/^\/+/, "");
  const rootPath = root
    .replaceAll("\\", "/")
    .replace(/^\/+/, "")
    .replace(/\/+$/, "");
  if (!targetPath.toLowerCase().startsWith(rootPath.toLowerCase() + "/")) {
    // Outside the tree (stdlib, dependencies): not openable in the tree view.
    return null;
  }
  return {
    file: targetPath.slice(rootPath.length + 1),
    line: range.start.line + 1,
  };
}
