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
  import { MIN_TAB_PX, SCROLL_STEP_PX, canScroll, overflows } from "../tab-strip";
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
    scrollLeftLabel,
    scrollRightLabel,
    onSelect,
    onClose,
    mark,
  }: {
    items: TabItem[];
    activeId: string;
    label: string;
    closeLabel: string;
    scrollLeftLabel: string;
    scrollRightLabel: string;
    onSelect: (id: string) => void;
    onClose: (id: string) => void;
    /** Rendered inside each tab, before its label: a badge, a count, a glyph. */
    mark?: Snippet<[TabItem]>;
  } = $props();

  let strip: HTMLDivElement | null = $state(null);

  /**
   * Overflow is measured, never assumed: the controls exist only while there is
   * something to scroll. A control that is always there and sometimes inert
   * teaches people to ignore it.
   */
  let visibleWidth = $state(0);
  let contentWidth = $state(0);
  let offset = $state(0);
  const overflowing = $derived(overflows(visibleWidth, contentWidth));
  const reach = $derived(canScroll(offset, visibleWidth, contentWidth));

  function measure() {
    if (!strip) return;
    visibleWidth = strip.clientWidth;
    contentWidth = strip.scrollWidth;
    offset = strip.scrollLeft;
  }

  $effect(() => {
    if (!strip) return;
    // Reading the layout during render is how a read/write loop starts, so the
    // observer is what tells us the size changed.
    const observer = new ResizeObserver(() => measure());
    observer.observe(strip);
    measure();
    return () => observer.disconnect();
  });

  // The list of items changing is the other thing that changes the content
  // width, and a ResizeObserver on the strip does not see it.
  $effect(() => {
    void items.length;
    queueMicrotask(measure);
  });

  function scrollBy(direction: -1 | 1) {
    strip?.scrollBy({ left: direction * SCROLL_STEP_PX, behavior: "auto" });
    queueMicrotask(measure);
  }

  /**
   * The active tab is brought into view with `nearest`, deliberately: it moves
   * the minimum, so a tab already visible does not jump. Without this the ARIA
   * arrows move focus to a tab off screen and the user types into something
   * they cannot see — an accessibility failure, not a cosmetic one.
   */
  $effect(() => {
    const id = activeId;
    if (!strip) return;
    queueMicrotask(() => {
      const el = strip?.querySelector<HTMLElement>(`[id="tab-${CSS.escape(id)}"]`);
      el?.scrollIntoView({ block: "nearest", inline: "nearest" });
      measure();
    });
  });

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

<div class="strip">
  {#if overflowing}
    <button
      class="ghost nudge"
      aria-label={scrollLeftLabel}
      title={scrollLeftLabel}
      disabled={!reach.left}
      onclick={() => scrollBy(-1)}
    >
      <Icon name="chevronLeft" size={14} />
    </button>
  {/if}
  <div
    class="tabs"
    role="tablist"
    aria-label={label}
    aria-orientation="horizontal"
    bind:this={strip}
    onscroll={measure}
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
  {#if overflowing}
    <button
      class="ghost nudge"
      aria-label={scrollRightLabel}
      title={scrollRightLabel}
      disabled={!reach.right}
      onclick={() => scrollBy(1)}
    >
      <Icon name="chevronRight" size={14} />
    </button>
  {/if}
</div>

<style>
  .strip {
    display: flex;
    align-items: stretch;
    gap: var(--sp-1);
    min-width: 0;
  }
  /* One row, always. Tabs shrink to their minimum first; only then does the
     strip scroll, which is what the overflow controls are for. */
  .tabs {
    display: flex;
    flex-wrap: nowrap;
    align-items: flex-end;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* The strip scrolls by its controls and by the wheel; it never shows a bar
       of its own under a row 28px tall. */
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    flex: 0 1 auto;
    min-width: 96px;
    max-width: 240px;
    border: 1px solid var(--border);
    /* Chrome's shape: contiguous, top corners rounded, the active one joined to
       the panel it governs. */
    border-bottom: none;
    border-radius: var(--radius-control) var(--radius-control) 0 0;
    background: var(--surface-2);
  }
  .tab.active {
    border-color: var(--accent);
    background: var(--surface);
  }
  .nudge {
    flex: none;
    padding: 0 var(--sp-1);
    align-self: center;
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
