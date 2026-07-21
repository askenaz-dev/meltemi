// SPDX-License-Identifier: Apache-2.0
// CodeMirror 6 assembly for the utilitarian editor: line numbers, history,
// search, syntax highlight by extension, diagnostics gutter, and a
// design-system theme over CSS variables (both color schemes for free).

import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightSpecialChars,
  drawSelection,
} from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import {
  defaultHighlightStyle,
  syntaxHighlighting,
  indentOnInput,
  bracketMatching,
} from "@codemirror/language";
import { lintGutter, setDiagnostics, type Diagnostic } from "@codemirror/lint";
import {
  autocompletion,
  completionKeymap,
  type CompletionSource,
} from "@codemirror/autocomplete";
import { rust } from "@codemirror/lang-rust";
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";

export function languageFor(path: string): Extension[] {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "rs":
      return [rust()];
    case "ts":
    case "tsx":
      return [javascript({ typescript: true, jsx: ext === "tsx" })];
    case "js":
    case "jsx":
    case "mjs":
      return [javascript({ jsx: ext === "jsx" })];
    case "json":
      return [json()];
    case "md":
      return [markdown()];
    default:
      return [];
  }
}

/** The LSP language id for a path, when we know one (BYO servers). */
export function lspLanguageFor(path: string): string | null {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "rs":
      return "rust";
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "mjs":
      return "typescript";
    case "py":
      return "python";
    default:
      return null;
  }
}

const theme = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "var(--surface)",
    color: "var(--text)",
    fontSize: "0.8125rem",
  },
  ".cm-content": { fontFamily: "var(--font-mono)" },
  ".cm-gutters": {
    backgroundColor: "var(--surface-2)",
    color: "var(--text-muted)",
    border: "none",
  },
  ".cm-activeLine": { backgroundColor: "transparent" },
  "&.cm-focused .cm-activeLine": { backgroundColor: "rgb(127 127 127 / 0.08)" },
  "&.cm-focused": { outline: "2px solid var(--focus)", outlineOffset: "-2px" },
  ".cm-selectionBackground": { backgroundColor: "rgb(37 99 235 / 0.25)" },
});

export interface EditorHandle {
  view: EditorView;
  setDiagnostics(diagnostics: Diagnostic[]): void;
  destroy(): void;
}

export function createEditor(options: {
  parent: HTMLElement;
  doc: string;
  path: string;
  onChange: (doc: string) => void;
  onSave: () => void;
  completionSource?: CompletionSource;
  /** F12: navigate to definition at the given 0-based LSP position. */
  onGotoDefinition?: (position: { line: number; character: number }) => void;
}): EditorHandle {
  const extensions: Extension[] = [
    lineNumbers(),
    highlightSpecialChars(),
    history(),
    drawSelection(),
    indentOnInput(),
    bracketMatching(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    lintGutter(),
    autocompletion(
      options.completionSource
        ? { override: [options.completionSource] }
        : {},
    ),
    keymap.of([
      {
        key: "Mod-s",
        run: () => {
          options.onSave();
          return true;
        },
      },
      {
        key: "F12",
        run: (view) => {
          if (!options.onGotoDefinition) return false;
          const head = view.state.selection.main.head;
          const line = view.state.doc.lineAt(head);
          options.onGotoDefinition({
            line: line.number - 1,
            character: head - line.from,
          });
          return true;
        },
      },
      ...defaultKeymap,
      ...historyKeymap,
      ...searchKeymap,
      ...completionKeymap,
      indentWithTab,
    ]),
    theme,
    ...languageFor(options.path),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        options.onChange(update.state.doc.toString());
      }
    }),
  ];

  const view = new EditorView({
    state: EditorState.create({ doc: options.doc, extensions }),
    parent: options.parent,
  });

  return {
    view,
    setDiagnostics(diagnostics: Diagnostic[]) {
      view.dispatch(setDiagnostics(view.state, diagnostics));
    },
    destroy() {
      view.destroy();
    },
  };
}

/** Moves the cursor to a 1-based line and scrolls it into view. */
export function gotoLine(handle: EditorHandle, line: number): void {
  const doc = handle.view.state.doc;
  const clamped = Math.max(1, Math.min(line, doc.lines));
  const position = doc.line(clamped).from;
  handle.view.dispatch({
    selection: { anchor: position },
    effects: EditorView.scrollIntoView(position, { y: "center" }),
  });
  handle.view.focus();
}
