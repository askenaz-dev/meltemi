<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { get } from "svelte/store";
  import { invoke } from "@tauri-apps/api/core";
  import { request } from "../daemon";
  import { t } from "../i18n";
  import { fuzzyRank, highlightRuns } from "../fuzzy";
  import { groupOf, REGISTRY, type MethodGroup, type RegistryEntry, type ViewId } from "../registry";
  import { activeProject } from "../stores";
  import { frecencyBonus, recordPaletteUse } from "../ui-state";
  import { METHOD_FORMS } from "../generated/method-forms";
  import Icon from "./Icon.svelte";
  import MethodForm from "./MethodForm.svelte";

  let {
    onClose,
    onNavigate,
  }: { onClose: () => void; onNavigate: (view: ViewId) => void } = $props();

  const GROUP_ORDER: MethodGroup[] = [
    "session",
    "sdd",
    "worktree",
    "checkpoint",
    "spec",
    "system",
  ];

  let query = $state("");
  let cursor = $state(0);
  let selected: RegistryEntry | null = $state(null);
  let values: Record<string, unknown> = $state({});
  let rawParams = $state("{}");
  let mode: "form" | "raw" = $state("form");
  let confirmDangerous = $state(false);
  let running = $state(false);
  let result: string | null = $state(null);
  let error: { message: string; detail: string | null; remedy: string | null } | null =
    $state(null);
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
  });

  /** Fuzzy over "method description", frecency-boosted, grouped by domain. */
  const ranked = $derived(
    fuzzyRank(
      query,
      REGISTRY,
      (entry) => `${entry.method} ${$t(entry.descKey)}`,
      (entry) => frecencyBonus(entry.method),
    ),
  );

  const flat = $derived(ranked.map((row) => row.item));

  const grouped = $derived.by(() => {
    const groups = new Map<MethodGroup, typeof ranked>();
    for (const row of ranked) {
      const group = groupOf(row.item.method);
      const bucket = groups.get(group) ?? [];
      bucket.push(row);
      groups.set(group, bucket);
    }
    return GROUP_ORDER.filter((group) => groups.has(group)).map((group) => ({
      group,
      rows: groups.get(group)!,
    }));
  });

  function indexOf(entry: RegistryEntry): number {
    return flat.indexOf(entry);
  }

  function select(entry: RegistryEntry) {
    selected = entry;
    result = null;
    error = null;
    confirmDangerous = false;
    mode = METHOD_FORMS[entry.method] ? "form" : "raw";
    const template: Record<string, unknown> = structuredClone(entry.template ?? {});
    if (entry.injectRoot) {
      const root = get(activeProject);
      if (root) template[entry.injectRoot] = root;
    }
    values = template;
    rawParams = JSON.stringify(template, null, 2);
  }

  async function run() {
    if (!selected || running) return;
    if (selected.dangerous && !confirmDangerous) return;
    running = true;
    result = null;
    error = null;
    try {
      const params = rawParams.trim() ? JSON.parse(rawParams) : {};
      if (selected.kind === "notify") {
        await invoke("daemon_notify", { method: selected.method, params });
        result = "—";
      } else {
        result = JSON.stringify(await request(selected.method, params), null, 2);
      }
      recordPaletteUse(selected.method);
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      error = {
        message: e?.message ?? String(raw),
        detail: e?.detail ?? null,
        remedy: e?.remedy ?? null,
      };
    } finally {
      running = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    // A text-entry context: keys stay local, Esc is the announced way out.
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      if (selected) selected = null;
      else onClose();
      return;
    }
    if (selected) {
      if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        void run();
      }
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      cursor = Math.min(cursor + 1, flat.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      cursor = Math.max(cursor - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const entry = flat[cursor];
      if (entry) select(entry);
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="scrim" role="presentation" onkeydown={onKeydown} onclick={onClose}>
  <div class="palette" role="dialog" aria-modal="true" aria-label={$t("palette.title")}>
    {#if !selected}
      <div class="searchRow">
        <Icon name="search" size={16} />
        <input
          bind:this={input}
          bind:value={query}
          oninput={() => (cursor = 0)}
          placeholder={$t("palette.placeholder")}
          aria-label={$t("palette.title")}
        />
        <kbd>Ctrl K</kbd>
      </div>
      <p class="hint">{$t("palette.hint")}</p>
      <div class="list" role="listbox" aria-label={$t("palette.title")}>
        {#each grouped as bucket (bucket.group)}
          <p class="group">{$t(("palette.group." + bucket.group) as never)}</p>
          {#each bucket.rows as row (row.item.method)}
            {@const index = indexOf(row.item)}
            <button
              role="option"
              aria-selected={index === cursor}
              class="row"
              class:active={index === cursor}
              onclick={() => select(row.item)}
            >
              <code>
                {#each highlightRuns(row.item.method, row.match.positions.filter((p) => p < row.item.method.length)) as run, i (i)}
                  {#if run.hit}<mark>{run.text}</mark>{:else}{run.text}{/if}
                {/each}
              </code>
              <span class="desc">{$t(row.item.descKey)}</span>
              {#if row.item.dangerous}
                <span class="pill warn">{$t("palette.irreversible")}</span>
              {/if}
              {#if row.item.view}
                <span class="nav">{$t("palette.nav")}</span>
              {/if}
            </button>
          {/each}
        {/each}
        {#if flat.length === 0}
          <p class="hint">{$t("palette.noMatch")}</p>
        {/if}
      </div>
    {:else}
      <div class="runner">
        <p class="method">
          <code>{selected.method}</code>
          <span class="desc">{$t(selected.descKey)}</span>
        </p>
        {#if selected.view}
          <button
            class="ghost navBtn"
            onclick={() => {
              const view = selected?.view;
              onClose();
              if (view) onNavigate(view);
            }}
          >
            {$t("palette.nav")}: {$t(("nav." + selected.view) as never)}
          </button>
        {/if}

        <div class="modeRow">
          <span class="label">{$t("palette.params")}</span>
          <div class="modes" role="group" aria-label={$t("palette.params")}>
            <button
              class="ghost"
              aria-pressed={mode === "form"}
              class:sel={mode === "form"}
              disabled={!METHOD_FORMS[selected.method]}
              onclick={() => (mode = "form")}>{$t("palette.mode.form")}</button
            >
            <button
              class="ghost"
              aria-pressed={mode === "raw"}
              class:sel={mode === "raw"}
              onclick={() => (mode = "raw")}>{$t("palette.mode.raw")}</button
            >
          </div>
        </div>

        {#if mode === "form"}
          <MethodForm
            method={selected.method}
            {values}
            bind:raw={rawParams}
            bind:mode
          />
        {:else}
          <textarea bind:value={rawParams} rows="6" spellcheck="false"></textarea>
        {/if}

        {#if selected.dangerous}
          <label class="danger">
            <input type="checkbox" bind:checked={confirmDangerous} />
            {$t("palette.dangerous")}
          </label>
        {/if}
        <div class="actions">
          <button class="ghost" onclick={() => (selected = null)}>{$t("common.back")}</button>
          <button
            class="primary"
            disabled={running || (selected.dangerous && !confirmDangerous)}
            onclick={() => void run()}
          >
            {running ? $t("common.loading") : $t("palette.run")}
          </button>
        </div>
        {#if result !== null}
          <h3>{$t("palette.result")}</h3>
          <pre>{result}</pre>
        {/if}
        {#if error}
          <h3 class="dangerText">{$t("palette.error")}</h3>
          <pre class="dangerText">{error.message}{error.detail ? "\n" + error.detail : ""}</pre>
          {#if error.remedy}
            <p class="remedy">
              <strong>{$t("common.remedy")}:</strong> {error.remedy}
            </p>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    justify-items: center;
    align-items: start;
    padding-top: 10vh;
    z-index: 30;
  }
  .palette {
    width: min(720px, 92vw);
    max-height: 72vh;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--sp-3);
    display: grid;
    gap: var(--sp-2);
    align-content: start;
  }
  .searchRow {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    color: var(--text-muted);
  }
  .searchRow input {
    flex: 1;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-caption);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    padding: 0 5px;
    color: var(--text-faint);
  }
  .hint {
    margin: 0;
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .list {
    display: grid;
    gap: 1px;
  }
  .group {
    margin: var(--sp-2) 0 2px;
    font-size: var(--fs-caption);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-faint);
  }
  .row {
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    text-align: left;
    background: transparent;
    border-color: transparent;
    width: 100%;
    padding: var(--sp-1) var(--sp-2);
  }
  .row.active,
  .row:hover {
    background: var(--surface-2);
    border-color: transparent;
  }
  .row.active::before {
    content: "▸";
    color: var(--accent);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  mark {
    background: transparent;
    color: var(--accent);
    font-weight: 500;
  }
  .desc {
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
  .nav {
    margin-left: auto;
    color: var(--accent);
    font-size: var(--fs-caption);
  }
  .runner {
    display: grid;
    gap: var(--sp-3);
  }
  .method {
    margin: 0;
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    flex-wrap: wrap;
  }
  .modeRow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-2);
  }
  .label {
    font-size: var(--fs-dense);
    color: var(--text-muted);
  }
  .modes {
    display: flex;
    gap: var(--sp-1);
  }
  .modes .sel {
    color: var(--accent);
    border-color: var(--border);
  }
  .navBtn {
    justify-self: start;
    color: var(--accent);
  }
  textarea {
    width: 100%;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
  .danger {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
    color: var(--warn);
    font-size: var(--fs-dense);
  }
  .danger input {
    width: auto;
  }
  h3 {
    margin: 0;
    font-size: var(--fs-body);
  }
  pre {
    margin: 0;
    padding: var(--sp-2);
    background: var(--surface-2);
    border-radius: var(--radius-control);
    overflow: auto;
    max-height: 30vh;
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  .dangerText {
    color: var(--danger);
  }
  .remedy {
    margin: 0;
    color: var(--info);
    font-size: var(--fs-dense);
  }
</style>
