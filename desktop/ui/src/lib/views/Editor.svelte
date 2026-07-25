<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { untrack } from "svelte";
  import { t } from "../i18n";
  import { pushNotice } from "../stores";
  import {
    loadTree,
    readFile,
    saveFile,
    searchProject,
    openWith,
    type SaveTarget,
    type SearchMatch,
    type TreeNode,
  } from "../editor/files";
  import {
    createEditor,
    gotoLine,
    lspLanguageFor,
    type EditorHandle,
  } from "../editor/cm";
  import {
    gotoDefinition,
    lspEnsure,
    lspNotifyChange,
    lspNotifyOpen,
    makeCompletionSource,
    onLspDiagnostics,
    toCmDiagnostics,
  } from "../editor/lsp";
  import { request } from "../daemon";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";

  let {
    root,
    target = null,
    initialFile = null,
    onBack,
  }: {
    root: string;
    target?: SaveTarget | null;
    initialFile?: string | null;
    onBack: () => void;
  } = $props();

  let openedInitial = false;
  $effect(() => {
    if (initialFile && !openedInitial) {
      openedInitial = true;
      void openFile(initialFile);
    }
  });

  interface OpenFile {
    path: string;
    content: string;
    dirty: boolean;
  }

  let nodes: TreeNode[] = $state([]);
  let treeTruncated = $state(false);
  let collapsed: Record<string, boolean> = $state({});
  let open: OpenFile[] = $state([]);
  let activePath: string | null = $state(null);
  let editorHost: HTMLDivElement | undefined = $state();
  let handle: EditorHandle | null = null;
  let searchQuery = $state("");
  let searchMatches: SearchMatch[] = $state([]);
  let searchTruncated = $state(false);
  let searchRan = $state(false);
  let confirmState: "session_active" | "turn_in_flight" | null = $state(null);
  let lspLabel: string | null = $state(null);
  let validateSummary: string | null = $state(null);
  let validating = $state(false);

  const active = $derived(open.find((f) => f.path === activePath) ?? null);
  const isMethodFile = $derived((activePath ?? "").startsWith(".meltemi/"));

  $effect(() => {
    void loadTree(root)
      .then((result) => {
        nodes = result.nodes;
        treeTruncated = result.truncated;
      })
      .catch(() => {
        nodes = [];
      });
  });

  // (Re)mount CodeMirror when the ACTIVE FILE changes — never on keystrokes:
  // the content read is untracked, or every edit would recreate the editor.
  $effect(() => {
    const file = active;
    const host = editorHost;
    if (!file || !host) return;
    const path = file.path;
    const language = lspLanguageFor(path);
    handle = createEditor({
      parent: host,
      doc: untrack(() => file.content),
      path,
      onChange: (doc) => {
        const entry = open.find((f) => f.path === path);
        if (entry && entry.content !== doc) {
          entry.content = doc;
          entry.dirty = true;
          if (language) void lspNotifyChange(root, path, doc);
        }
      },
      onSave: () => void save(false),
      completionSource: language
        ? makeCompletionSource(root, path)
        : undefined,
      onGotoDefinition: language
        ? (position) => {
            void gotoDefinition(root, path, position).then((found) => {
              if (found) void openFile(found.file, found.line);
            });
          }
        : undefined,
    });
    const offDiagnostics = language
      ? onLspDiagnostics(path, (diagnostics) => {
          if (handle) {
            handle.setDiagnostics(
              toCmDiagnostics(handle.view.state.doc, diagnostics),
            );
          }
        })
      : null;
    if (language) {
      void lspEnsure(root, language).then((server) => {
        lspLabel = server
          ? $t("editor.lsp.active", { server })
          : $t("editor.lsp.none");
        if (server) void lspNotifyOpen(root, path, language, file.content);
      });
    } else {
      lspLabel = null;
    }
    return () => {
      offDiagnostics?.();
      handle?.destroy();
      handle = null;
    };
  });

  async function openFile(path: string, line?: number) {
    const existing = open.find((f) => f.path === path);
    if (!existing) {
      try {
        const read = await readFile(root, path);
        open = [...open, { path, content: read.content, dirty: false }];
      } catch (raw) {
        const e = raw as { detail?: string };
        pushNotice(
          `${$t("editor.readError")}: ${e?.detail ?? path}`,
          "danger",
        );
        return;
      }
    }
    activePath = path;
    if (line !== undefined) {
      setTimeout(() => {
        if (handle) gotoLine(handle, line);
      }, 50);
    }
  }

  function closeTab(path: string) {
    open = open.filter((f) => f.path !== path);
    if (activePath === path) {
      activePath = open.length > 0 ? open[open.length - 1].path : null;
    }
  }

  async function save(confirm: boolean) {
    const file = active;
    if (!file) return;
    try {
      const outcome = await saveFile(root, file.path, file.content, target, confirm);
      file.dirty = false;
      confirmState = null;
      const dest =
        outcome.loggedTo === "session"
          ? $t("editor.saved.session")
          : $t("editor.saved.project");
      pushNotice($t("editor.saved", { file: file.path, dest }), "info");
      if (isMethodFile) void validate();
    } catch (raw) {
      const e = raw as { kind?: string };
      if (e?.kind === "session_active" || e?.kind === "turn_in_flight") {
        confirmState = e.kind;
      } else {
        const err = raw as { message?: string; detail?: string };
        pushNotice(
          `${$t("common.error")}: ${err?.detail ?? err?.message ?? String(raw)}`,
          "danger",
        );
      }
    }
  }

  async function runSearch() {
    searchRan = true;
    try {
      const result = await searchProject(root, searchQuery);
      searchMatches = result.matches;
      searchTruncated = result.truncated;
    } catch {
      searchMatches = [];
    }
  }

  async function openExternally() {
    const file = active;
    if (!file) return;
    const line = handle
      ? handle.view.state.doc.lineAt(handle.view.state.selection.main.head)
          .number
      : undefined;
    const editor = await openWith(root, file.path, line);
    pushNotice($t("editor.openedWith", { editor }), "info");
  }

  async function validate() {
    if (validating) return;
    validating = true;
    try {
      const result = await request<{ clean: boolean; diagnostics: unknown[] }>(
        "sdd/validate",
        { projectRoot: root },
      );
      validateSummary = result.clean
        ? $t("editor.validate.clean")
        : $t("editor.validate.findings", { n: result.diagnostics.length });
    } catch (raw) {
      const e = raw as { message?: string };
      validateSummary = `${$t("common.error")}: ${e?.message ?? String(raw)}`;
    } finally {
      validating = false;
    }
  }

  function visible(list: TreeNode[], depth: number): { node: TreeNode; depth: number }[] {
    const rows: { node: TreeNode; depth: number }[] = [];
    for (const node of list) {
      rows.push({ node, depth });
      if (node.isDir && !collapsed[node.path]) {
        rows.push(...visible(node.children, depth + 1));
      }
    }
    return rows;
  }
</script>

<section class="editor" aria-label={$t("editor.title")}>
  <aside aria-label={$t("editor.tree")}>
    <div class="search">
      <input
        bind:value={searchQuery}
        placeholder={$t("editor.search.placeholder")}
        onkeydown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") void runSearch();
        }}
      />
    </div>
    {#if searchRan && searchQuery.trim()}
      <div class="results" role="list">
        {#if searchMatches.length === 0}
          <p class="muted">{$t("editor.search.none")}</p>
        {/if}
        {#each searchMatches as match, i (match.file + ":" + match.line + ":" + i)}
          <button
            class="result"
            onclick={() => void openFile(match.file, match.line)}
          >
            <code>{match.file}:{match.line}</code>
            <span>{match.text}</span>
          </button>
        {/each}
        {#if searchTruncated}
          <p class="muted">! {$t("editor.search.truncated")}</p>
        {/if}
      </div>
    {:else}
      <div class="tree" role="tree">
        {#each visible(nodes, 0) as { node, depth } (node.path)}
          <button
            class="node"
            role="treeitem"
            aria-selected={node.path === activePath}
            aria-expanded={node.isDir ? !collapsed[node.path] : undefined}
            style="padding-left: {8 + depth * 14}px"
            onclick={() => {
              if (node.isDir) {
                collapsed[node.path] = !collapsed[node.path];
              } else {
                void openFile(node.path);
              }
            }}
          >
            <span aria-hidden="true">
              {node.isDir ? (collapsed[node.path] ? "▸" : "▾") : "·"}
            </span>
            {node.name}
          </button>
        {/each}
        {#if treeTruncated}
          <p class="muted">! {$t("editor.tree.truncated")}</p>
        {/if}
      </div>
    {/if}
  </aside>

  <div class="main">
    <header>
      <div class="tabs" role="tablist" aria-label={$t("editor.tabs")}>
        {#each open as file (file.path)}
          <span class="tab" class:active={file.path === activePath}>
            <button
              role="tab"
              aria-selected={file.path === activePath}
              onclick={() => (activePath = file.path)}
            >
              {file.path.split("/").pop()}{file.dirty ? " •" : ""}
            </button>
            <button
              class="x"
              aria-label={$t("editor.close")}
              onclick={() => closeTab(file.path)}>×</button
            >
          </span>
        {/each}
      </div>
      <div class="actions">
        {#if lspLabel}
          <span class="muted lsp">{lspLabel}</span>
        {/if}
        {#if isMethodFile}
          <button disabled={validating} onclick={() => void validate()}>
            {validating ? $t("common.loading") : $t("editor.validate.run")}
          </button>
          {#if validateSummary}
            <span class="muted" aria-live="polite">{validateSummary}</span>
          {/if}
        {/if}
        <button onclick={() => void openExternally()} disabled={!active}>
          {$t("editor.openWith")}
        </button>
        <button
          class="primary"
          onclick={() => void save(false)}
          disabled={!active || !active.dirty}
        >
          {$t("editor.save")}
        </button>
        <button onclick={onBack}>{$t("common.back")}</button>
      </div>
    </header>

    {#if active}
      {#key active.path}
        <div class="cm-host" bind:this={editorHost}></div>
      {/key}
    {:else}
      <p class="muted empty">{$t("editor.tree")}</p>
    {/if}
  </div>
</section>

{#if confirmState}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={confirmState === "turn_in_flight"
      ? $t("editor.confirm.reinforced")
      : $t("editor.confirm.simple")}
    confirmLabel={$t("editor.confirm.save")}
    onConfirm={() => void save(true)}
    onCancel={() => (confirmState = null)}
  />
{/if}

<style>
  .editor {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: var(--sp-3);
    height: 100%;
    min-height: 0;
  }
  aside {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: var(--sp-2);
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    background: var(--surface);
    padding: var(--sp-2);
  }
  .search input {
    width: 100%;
    font: inherit;
    padding: var(--sp-1) var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--bg);
    color: var(--text);
  }
  .tree,
  .results {
    overflow: auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }
  .node,
  .result {
    font: inherit;
    text-align: left;
    border: 0;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    padding: 2px var(--sp-2);
    border-radius: var(--radius-control);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .node:hover,
  .result:hover {
    background: var(--surface-2);
  }
  .result {
    display: grid;
    white-space: normal;
  }
  .result code {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--accent);
  }
  .result span {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .main {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: var(--sp-2);
    min-height: 0;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }
  .tabs {
    display: flex;
    gap: var(--sp-1);
    flex-wrap: wrap;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
  }
  .tab.active {
    border-color: var(--accent);
  }
  .tab button {
    font: inherit;
    border: 0;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
  }
  .tab .x {
    padding: var(--sp-1);
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .actions button {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .primary {
    border-color: var(--accent) !important;
  }
  .cm-host {
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
  }
  .empty {
    display: grid;
    place-content: center;
  }
  .muted {
    color: var(--text-muted);
    font-size: 0.8125rem;
  }
  .lsp {
    font-family: var(--font-mono);
  }
</style>
