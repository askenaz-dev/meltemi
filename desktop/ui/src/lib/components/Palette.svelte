<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { get } from "svelte/store";
  import { t } from "../i18n";
  import { REGISTRY, type RegistryEntry, type ViewId } from "../registry";
  import { projectRoot } from "../stores";
  import { request } from "../daemon";
  import { invoke } from "@tauri-apps/api/core";

  let {
    onClose,
    onNavigate,
  }: { onClose: () => void; onNavigate: (view: ViewId) => void } = $props();

  let filter = $state("");
  let cursor = $state(0);
  let selected: RegistryEntry | null = $state(null);
  let paramsText = $state("");
  let confirmDangerous = $state(false);
  let running = $state(false);
  let result: string | null = $state(null);
  let error: {
    message: string;
    detail: string | null;
    remedy: string | null;
  } | null = $state(null);
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
  });

  const entries = $derived(
    REGISTRY.filter((entry) => {
      const needle = filter.trim().toLowerCase();
      if (!needle) return true;
      return (
        entry.method.toLowerCase().includes(needle) ||
        $t(entry.descKey).toLowerCase().includes(needle)
      );
    }),
  );

  function select(entry: RegistryEntry) {
    selected = entry;
    result = null;
    error = null;
    confirmDangerous = false;
    const template = structuredClone(entry.template ?? {});
    if (entry.injectRoot) {
      const root = get(projectRoot);
      if (root) {
        (template as Record<string, unknown>)[entry.injectRoot] = root;
      }
    }
    paramsText = JSON.stringify(template, null, 2);
  }

  async function run() {
    if (!selected || running) return;
    if (selected.dangerous && !confirmDangerous) return;
    running = true;
    result = null;
    error = null;
    try {
      const params = paramsText.trim() ? JSON.parse(paramsText) : {};
      if (selected.kind === "notify") {
        await invoke("daemon_notify", { method: selected.method, params });
        result = "—";
      } else {
        const value = await request(selected.method, params);
        result = JSON.stringify(value, null, 2);
      }
    } catch (raw) {
      const e = raw as {
        message?: string;
        detail?: string | null;
        remedy?: string | null;
      };
      error = {
        message: e?.message ?? String(raw),
        detail: e?.detail ?? null,
        remedy: e?.remedy ?? null,
      };
    } finally {
      running = false;
    }
  }

  function onListKeydown(event: KeyboardEvent) {
    // Text-entry context: keys stay local; Esc is the announced way out.
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      if (selected) {
        selected = null;
      } else {
        onClose();
      }
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
      cursor = Math.min(cursor + 1, entries.length - 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      cursor = Math.max(cursor - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const entry = entries[cursor];
      if (entry) select(entry);
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="scrim" role="presentation" onkeydown={onListKeydown}>
  <div
    class="palette"
    role="dialog"
    aria-modal="true"
    aria-label={$t("palette.title")}
  >
    {#if !selected}
      <input
        bind:this={input}
        bind:value={filter}
        oninput={() => (cursor = 0)}
        placeholder={$t("palette.placeholder")}
        aria-label={$t("palette.title")}
      />
      <p class="hint">{$t("palette.hint")}</p>
      <ul role="listbox" aria-label={$t("palette.title")}>
        {#each entries as entry, index (entry.method)}
          <li
            role="option"
            aria-selected={index === cursor}
            class:active={index === cursor}
          >
            <button onclick={() => select(entry)}>
              <code>{entry.method}</code>
              <span class="desc">{$t(entry.descKey)}</span>
              {#if entry.view}
                <span class="nav">{$t("palette.nav")}</span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="runner">
        <p><code>{selected.method}</code> — {$t(selected.descKey)}</p>
        {#if selected.view}
          <button
            class="navBtn"
            onclick={() => {
              const view = selected?.view;
              onClose();
              if (view) onNavigate(view);
            }}
          >
            {$t("palette.nav")}: {$t(("nav." + selected.view) as never)}
          </button>
        {/if}
        <label>
          {$t("palette.params")}
          <textarea bind:value={paramsText} rows="6" spellcheck="false"
          ></textarea>
        </label>
        {#if selected.dangerous}
          <label class="danger">
            <input type="checkbox" bind:checked={confirmDangerous} />
            {$t("palette.dangerous")}
          </label>
        {/if}
        <div class="actions">
          <button onclick={() => (selected = null)}>{$t("common.back")}</button>
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
          <pre class="dangerText">{error.message}{error.detail
              ? "\n" + error.detail
              : ""}</pre>
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
    background: rgb(0 0 0 / 0.4);
    display: grid;
    justify-items: center;
    align-items: start;
    padding-top: 10vh;
    z-index: 30;
  }
  .palette {
    width: min(680px, 92vw);
    max-height: 70vh;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--sp-4);
    display: grid;
    gap: var(--sp-2);
  }
  input[type="checkbox"] {
    width: auto;
  }
  input,
  textarea {
    font: inherit;
    width: 100%;
    padding: var(--sp-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--bg);
    color: var(--text);
  }
  textarea {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.8125rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  li button {
    width: 100%;
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    text-align: left;
    font: inherit;
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }
  li.active button,
  li button:hover {
    background: var(--surface-2);
  }
  li.active button::before {
    content: "▸ ";
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .desc {
    color: var(--text-muted);
    font-size: 0.8125rem;
  }
  .nav {
    margin-left: auto;
    color: var(--accent);
    font-size: 0.75rem;
  }
  .runner {
    display: grid;
    gap: var(--sp-3);
  }
  .runner p {
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
  .actions button,
  .navBtn {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .primary {
    border-color: var(--accent);
  }
  .danger {
    color: var(--warn);
    display: flex;
    gap: var(--sp-2);
    align-items: center;
  }
  h3 {
    margin: 0;
    font-size: 0.875rem;
  }
  pre {
    margin: 0;
    padding: var(--sp-2);
    background: var(--surface-2);
    border-radius: var(--radius-control);
    overflow: auto;
    max-height: 30vh;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .dangerText {
    color: var(--danger);
  }
  .remedy {
    margin: 0;
    color: var(--info);
  }
</style>
