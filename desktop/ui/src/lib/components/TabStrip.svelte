<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  A tab strip that follows the WAI-ARIA tabs pattern: one tab in the tab order
  at a time, arrows to move between them, Home and End for the ends, Delete to
  close a closable one. Generic on purpose — it knows about items, not about
  sessions — so the surface has one correct implementation instead of one per
  view (sidebar-ajustable-y-pestanas design D5).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  export interface TabItem {
    id: string;
    label: string;
    /** The full story the label had to shorten. */
    title?: string;
    closable: boolean;
  }

  let {
    items,
    activeId,
    label,
    closeLabel,
    onSelect,
    onClose,
    mark,
  }: {
    items: TabItem[];
    activeId: string;
    label: string;
    closeLabel: string;
    onSelect: (id: string) => void;
    onClose: (id: string) => void;
    /** Rendered inside each tab, before its label: a badge, a count, a glyph. */
    mark?: Snippet<[TabItem]>;
  } = $props();

  let strip: HTMLDivElement | null = $state(null);

  /** Selection and DOM focus move together, or the arrows lie about where you are. */
  function focusTab(index: number) {
    const id = items[index]?.id;
    if (id === undefined) return;
    onSelect(id);
    queueMicrotask(() => {
      strip?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[index]?.focus();
    });
  }

  function onKeys(event: KeyboardEvent) {
    const current = items.findIndex((item) => item.id === activeId);
    if (current < 0 || items.length === 0) return;
    let next: number | null = null;
    if (event.key === "ArrowRight") next = (current + 1) % items.length;
    else if (event.key === "ArrowLeft") next = (current - 1 + items.length) % items.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = items.length - 1;
    else if (event.key === "Delete") {
      if (!items[current].closable) return;
      event.preventDefault();
      event.stopPropagation();
      onClose(items[current].id);
      return;
    }
    if (next === null) return;
    event.preventDefault();
    event.stopPropagation();
    focusTab(next);
  }
</script>

<div
  class="tabs"
  role="tablist"
  aria-label={label}
  aria-orientation="horizontal"
  bind:this={strip}
>
  {#each items as item (item.id)}
    {@const active = item.id === activeId}
    <span class="tab" class:active>
      <button
        role="tab"
        id="tab-{item.id}"
        aria-controls="panel-{item.id}"
        aria-selected={active}
        tabindex={active ? 0 : -1}
        title={item.title ?? item.label}
        onclick={() => onSelect(item.id)}
        onkeydown={onKeys}
      >
        {#if mark}{@render mark(item)}{/if}
        <span class="text">{item.label}</span>
      </button>
      {#if item.closable}
        <button class="x ghost" aria-label={closeLabel} onclick={() => onClose(item.id)}>
          <Icon name="close" size={11} />
        </button>
      {/if}
    </span>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: var(--sp-1);
    flex-wrap: wrap;
    align-items: center;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    max-width: 260px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
  }
  .tab.active {
    border-color: var(--accent);
  }
  .tab button {
    display: inline-flex;
    align-items: center;
    /* Deliberately tighter than the skin's --sp-2, as the editor's strip is. */
    gap: 4px;
    border: 0;
    background: transparent;
    font-size: var(--fs-dense);
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
