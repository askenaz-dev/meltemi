<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { untrack } from "svelte";
  import { t } from "../i18n";
  import { pushNotice } from "../stores";
  import { fuzzyRank } from "../fuzzy";
  import { recentFiles, recordRecentFile } from "../ui-state";
  import { markClean, markDirty } from "../editor/dirty";
  import {
    loadTree,
    openWith,
    readFile,
    saveFile,
    searchProject,
    type SaveTarget,
    type SearchMatch,
    type TreeNode,
  } from "../editor/files";
  import { createEditor, gotoLine, lspLanguageFor, type EditorHandle } from "../editor/cm";
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
  import Icon from "../components/Icon.svelte";

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

  interface OpenFile {
    path: string;
    content: string;
    dirty: boolean;
  }

  let nodes: TreeNode[] = $state([]);
  let treeTruncated = $state(false);
  let allFiles: string[] = $state([]);
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
  let closeGuard: string | null = $state(null);
  let quickOpen = $state(false);
  let quickQuery = $state("");
  let quickInput: HTMLInputElement | undefined = $state();
  let lspLabel: string | null = $state(null);
  let validateSummary: string | null = $state(null);
  let validating = $state(false);
  let openedInitial = false;

  const active = $derived(open.find((file) => file.path === activePath) ?? null);
  const isMethodFile = $derived((activePath ?? "").startsWith(".meltemi/"));
  const recents = $derived(recentFiles(root));

  const quickResults = $derived.by(() => {
    const pool = quickQuery.trim() === "" ? recents.slice(0, 12) : allFiles;
    return fuzzyRank(quickQuery, pool, (path) => path).slice(0, 20);
  });

  $effect(() => {
    void loadTree(root)
      .then((result) => {
        nodes = result.nodes;
        treeTruncated = result.truncated;
        const files: string[] = [];
        const walk = (list: TreeNode[]) => {
          for (const node of list) {
            if (node.isDir) walk(node.children);
            else files.push(node.path);
          }
        };
        walk(result.nodes);
        allFiles = files;
      })
      .catch(() => {
        nodes = [];
      });
  });

  $effect(() => {
    if (initialFile && !openedInitial) {
      openedInitial = true;
      void openFile(initialFile);
    }
  });

  $effect(() => {
    if (quickOpen) quickInput?.focus();
  });

  // (Re)mount CodeMirror when the ACTIVE FILE changes — never on keystrokes.
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
        const entry = open.find((candidate) => candidate.path === path);
        if (entry && entry.content !== doc) {
          entry.content = doc;
          entry.dirty = true;
          markDirty(path);
          if (language) void lspNotifyChange(root, path, doc);
        }
      },
      onSave: () => void save(false),
      completionSource: language ? makeCompletionSource(root, path) : undefined,
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
            handle.setDiagnostics(toCmDiagnostics(handle.view.state.doc, diagnostics));
          }
        })
      : null;
    if (language) {
      void lspEnsure(root, language).then((server) => {
        lspLabel = server ? $t("editor.lsp.active", { server }) : $t("editor.lsp.none");
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
    quickOpen = false;
    const existing = open.find((file) => file.path === path);
    if (!existing) {
      try {
        const read = await readFile(root, path);
        open = [...open, { path, content: read.content, dirty: false }];
        recordRecentFile(root, path);
      } catch (raw) {
        const e = raw as { detail?: string };
        pushNotice(`${$t("editor.readError")}: ${e?.detail ?? path}`, "danger");
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

  /** Closing a tab with unsaved work always asks first. */
  function requestClose(path: string) {
    const file = open.find((candidate) => candidate.path === path);
    if (file?.dirty) {
      closeGuard = path;
      return;
    }
    closeTab(path);
  }

  function closeTab(path: string) {
    markClean(path);
    open = open.filter((file) => file.path !== path);
    if (activePath === path) {
      activePath = open.length > 0 ? open[open.length - 1].path : null;
    }
    closeGuard = null;
  }

  async function save(confirm: boolean) {
    const file = active;
    if (!file) return;
    try {
      const outcome = await saveFile(root, file.path, file.content, target, confirm);
      file.dirty = false;
      markClean(file.path);
      confirmState = null;
      const dest =
        outcome.loggedTo === "session"
          ? $t("editor.saved.session")
          : $t("editor.saved.project");
      pushNotice($t("editor.saved", { file: file.path, dest }), "info");
      if (isMethodFile) void validate();
    } catch (raw) {
      const e = raw as { kind?: string; message?: string; detail?: string };
      if (e?.kind === "session_active" || e?.kind === "turn_in_flight") {
        confirmState = e.kind;
      } else {
        pushNotice(
          `${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`,
          "danger",
        );
      }
    }
  }

  /** Saves the guarded tab, then closes it once the write lands. */
  async function saveThenClose(path: string) {
    const previous = activePath;
    activePath = path;
    await save(false);
    const file = open.find((candidate) => candidate.path === path);
    if (file && !file.dirty) closeTab(path);
    else activePath = previous;
    closeGuard = null;
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
      ? handle.view.state.doc.lineAt(handle.view.state.selection.main.head).number
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

  function onHostKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "p") {
      event.preventDefault();
      event.stopPropagation();
      quickQuery = "";
      quickOpen = true;
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

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="editor" aria-label={$t("editor.title")} onkeydown={onHostKeydown}>
  <aside aria-label={$t("editor.tree")}>
    <div class="search">
      <input
        bind:value={searchQuery}
        placeholder={$t("editor.search.placeholder")}
        onkeydown={(event) => {
          event.stopPropagation();
          if (event.key === "Enter") void runSearch();
        }}
      />
    </div>
    {#if searchRan && searchQuery.trim()}
      <div class="results" role="list">
        {#if searchMatches.length === 0}
          <p class="faint">{$t("editor.search.none")}</p>
        {/if}
        {#each searchMatches as match, index (match.file + ":" + match.line + ":" + index)}
          <button class="result" onclick={() => void openFile(match.file, match.line)}>
            <code>{match.file}:{match.line}</code>
            <span>{match.text}</span>
          </button>
        {/each}
        {#if searchTruncated}
          <p class="faint">! {$t("editor.search.truncated")}</p>
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
            style="padding-left: {6 + depth * 14}px"
            onclick={() => {
              if (node.isDir) collapsed[node.path] = !collapsed[node.path];
              else void openFile(node.path);
            }}
          >
            <span class="tw" aria-hidden="true">
              {node.isDir ? (collapsed[node.path] ? "▸" : "▾") : "·"}
            </span>
            {node.name}
          </button>
        {/each}
        {#if treeTruncated}
          <p class="faint">! {$t("editor.tree.truncated")}</p>
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
            <button class="x ghost" aria-label={$t("editor.close")} onclick={() => requestClose(file.path)}>
              <Icon name="close" size={11} />
            </button>
          </span>
        {/each}
        <button class="ghost quick" onclick={() => ((quickQuery = ""), (quickOpen = true))}>
          <Icon name="search" size={12} />
          <kbd>Ctrl P</kbd>
        </button>
      </div>
      <div class="actions">
        {#if lspLabel}
          <span class="faint lsp">{lspLabel}</span>
        {/if}
        {#if isMethodFile}
          <button class="ghost" disabled={validating} onclick={() => void validate()}>
            {validating ? $t("common.loading") : $t("editor.validate.run")}
          </button>
          {#if validateSummary}
            <span class="faint" aria-live="polite">{validateSummary}</span>
          {/if}
        {/if}
        <button class="ghost" onclick={() => void openExternally()} disabled={!active}>
          <Icon name="external" size={13} />
          {$t("editor.openWith")}
        </button>
        <button class="primary" onclick={() => void save(false)} disabled={!active || !active.dirty}>
          <Icon name="save" size={13} />
          {$t("editor.save")}
        </button>
        <button class="ghost" onclick={onBack}>{$t("common.back")}</button>
      </div>
    </header>

    {#if active}
      {#key active.path}
        <div class="cm-host" bind:this={editorHost}></div>
      {/key}
    {:else}
      <p class="faint empty">{$t("editor.pickFile")}</p>
    {/if}
  </div>
</section>

{#if quickOpen}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="scrim"
    role="presentation"
    onkeydown={(event) => {
      event.stopPropagation();
      if (event.key === "Escape") quickOpen = false;
      if (event.key === "Enter" && quickResults[0]) void openFile(quickResults[0].item);
    }}
  >
    <div class="quickPanel" role="dialog" aria-modal="true" aria-label={$t("editor.quickOpen")}>
      <input
        bind:this={quickInput}
        bind:value={quickQuery}
        placeholder={$t("editor.quickOpen.placeholder")}
        aria-label={$t("editor.quickOpen")}
      />
      <div class="quickList" role="listbox">
        {#each quickResults as row, index (row.item)}
          <button role="option" aria-selected={index === 0} onclick={() => void openFile(row.item)}>
            <code>{row.item}</code>
          </button>
        {/each}
        {#if quickResults.length === 0}
          <p class="faint">{$t("editor.search.none")}</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

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

{#if closeGuard}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("editor.guard.tab", { file: closeGuard })}
    confirmLabel={$t("editor.guard.discard")}
    extraLabel={$t("editor.guard.save")}
    onExtra={() => closeGuard && void saveThenClose(closeGuard)}
    onConfirm={() => closeGuard && closeTab(closeGuard)}
    onCancel={() => (closeGuard = null)}
  />
{/if}

<style>
  .editor {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: var(--sp-2);
    height: 100%;
    min-height: 0;
    padding: var(--sp-2);
  }
  aside {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: var(--sp-2);
    min-height: 0;
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
    background: var(--panel);
    padding: var(--sp-2);
  }
  .search input {
    width: 100%;
    font-size: var(--fs-dense);
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
    font-size: var(--fs-dense);
    text-align: left;
    border: 0;
    background: transparent;
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
  .node[aria-selected="true"] {
    background: var(--surface-2);
    color: var(--accent);
  }
  .tw {
    color: var(--text-faint);
  }
  .result {
    display: grid;
    white-space: normal;
  }
  .result code {
    font-size: var(--fs-caption);
    color: var(--accent);
  }
  .result span {
    font-size: var(--fs-caption);
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
    align-items: center;
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
    border: 0;
    background: transparent;
    font-size: var(--fs-dense);
  }
  .quick {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    color: var(--text-faint);
  }
  kbd {
    font: inherit;
    font-size: var(--fs-caption);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 3px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .actions button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-dense);
  }
  .cm-host {
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
  }
  .empty {
    display: grid;
    place-content: center;
  }
  .faint {
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .lsp {
    font-family: var(--font-mono);
  }
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    justify-items: center;
    align-items: start;
    padding-top: 12vh;
    z-index: 32;
  }
  .quickPanel {
    width: min(560px, 92vw);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--sp-3);
    display: grid;
    gap: var(--sp-2);
  }
  .quickPanel input {
    width: 100%;
  }
  .quickList {
    display: grid;
    gap: 1px;
    max-height: 50vh;
    overflow: auto;
  }
  .quickList button {
    text-align: left;
    background: transparent;
    border-color: transparent;
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  .quickList button:hover,
  .quickList button[aria-selected="true"] {
    background: var(--surface-2);
  }
</style>
