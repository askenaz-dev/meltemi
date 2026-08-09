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
  import {
    MAX_TAB_PX,
    MIN_TAB_PX,
    SCROLL_STEP_PX,
    TAB_JOIN_PX,
    canScroll,
    overflows,
  } from "../tab-strip";
  import Icon from "./Icon.svelte";

  export interface TabItem {
    id: string;
    label: string;
    /** The full story the label had to shorten. */
    title?: string;
    closable: boolean;
    /** The group this tab belongs to, when it belongs to one. */
    group?: { id: string; name: string; color: string; collapsed: boolean; size: number };
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
    onToggleGroup = () => {},
    collapsedLabel = (g) => `${g.name} (${g.size})`,
    mark,
    menu,
  }: {
    items: TabItem[];
    activeId: string;
    label: string;
    closeLabel: string;
    scrollLeftLabel: string;
    scrollRightLabel: string;
    onSelect: (id: string) => void;
    onClose: (id: string) => void;
    /** Collapses or expands a group from its own label. */
    onToggleGroup?: (groupId: string, collapsed: boolean) => void;
    collapsedLabel?: (group: { name: string; size: number }) => string;
    /** Rendered inside each tab, before its label: a badge, a count, a glyph. */
    mark?: Snippet<[TabItem]>;
    /** Rendered inside the ACTIVE tab: the caller's own per-tab actions. */
    menu?: Snippet<[TabItem]>;
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

<!-- The strip's numbers have one owner: the module declares them, the element
     publishes them, and the stylesheet consumes them. A literal copy in the CSS
     is a second owner nobody keeps in step (piel-de-pestanas design D8). -->
<div
  class="strip"
  style="--tab-min: {MIN_TAB_PX}px; --tab-max: {MAX_TAB_PX}px; --tab-join: {TAB_JOIN_PX}px"
>
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
    {#each items as item, index (item.id)}
      {@const active = item.id === activeId}
      {@const group = item.group}
      {@const opens = group !== undefined && items[index - 1]?.group?.id !== group.id}
      {#if opens && group}
        <!-- The group's own label: its colour identifies it at a glance, its
             name and count carry the same information as text. -->
        <button
          class="groupTag ghost tone-{group.color}"
          aria-expanded={!group.collapsed}
          aria-label={group.collapsed ? collapsedLabel({ name: group.name, size: group.size }) : group.name}
          title={group.collapsed ? collapsedLabel({ name: group.name, size: group.size }) : group.name}
          onclick={() => onToggleGroup(group.id, !group.collapsed)}
        >
          <span class="swatch" aria-hidden="true"></span>
          <span class="text">
            {group.collapsed ? collapsedLabel({ name: group.name, size: group.size }) : group.name}
          </span>
        </button>
      {/if}
      {#if !group?.collapsed}
      <span class="tab" class:active class:grouped={group !== undefined}>
        <button
          role="tab"
        id="tab-{item.id}"
        aria-controls="panel-{item.id}"
        aria-selected={active}
        tabindex={active ? 0 : -1}
        title={item.title ?? item.label}
        aria-label={item.group ? `${item.label} — ${item.group.name}` : undefined}
        onclick={() => onSelect(item.id)}
        onkeydown={onKeys}
      >
        {#if mark}{@render mark(item)}{/if}
        <span class="text">{item.label}</span>
      </button>
        {#if menu && active}{@render menu(item)}{/if}
        {#if item.closable}
          <button class="x ghost" aria-label={closeLabel} onclick={() => onClose(item.id)}>
            <Icon name="close" size={11} />
          </button>
        {/if}
      </span>
      {/if}
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
    /* The strip is its own layer, darker than the panel: that contrast is what
       lets the active tab rise by joining the panel instead of by wearing an
       accent border (design D1). */
    background: var(--surface-2);
    overflow-x: auto;
    overflow-y: hidden;
    /* The strip scrolls by its controls and by the wheel; it never shows a bar
       of its own under a row 28px tall. */
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  /* A browser's anatomy: the tabs do not draw a box each, they rest on the
     strip's own layer, and the active one takes the panel's surface and joins
     it. Without that layer the active tab has nothing to rise from
     (piel-de-pestanas design D1). */
  .tab {
    display: inline-flex;
    align-items: center;
    position: relative;
    flex: 0 1 auto;
    min-width: var(--tab-min);
    max-width: var(--tab-max);
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius-control) var(--radius-control) 0 0;
    background: transparent;
  }
  .tab.active {
    background: var(--surface);
  }
  /* The seam: two squares filled with the panel's colour and bitten by a
     quarter circle, so the active tab spills into the strip in a curve instead
     of ending in a corner. Composition only — no mask-composite, no @property
     (design D2). */
  .tab.active::before,
  .tab.active::after {
    content: "";
    position: absolute;
    bottom: 0;
    width: var(--tab-join);
    height: var(--tab-join);
    pointer-events: none;
  }
  .tab.active::before {
    left: calc(-1 * var(--tab-join));
    background: radial-gradient(
      circle at top left,
      transparent var(--tab-join),
      var(--surface) var(--tab-join)
    );
  }
  .tab.active::after {
    right: calc(-1 * var(--tab-join));
    background: radial-gradient(
      circle at top right,
      transparent var(--tab-join),
      var(--surface) var(--tab-join)
    );
  }
  /* When the system substitutes its own colours, surfaces and gradients go with
     it — including the seam. Selection cannot ride on a fill the system is
     free to replace, so it falls back to a border in a system colour. The
     focus ring and aria-selected are separate carriers and are untouched
     (design D2). */
  @media (forced-colors: active) {
    .tab.active {
      border-color: Highlight;
    }
  }
  .nudge {
    flex: none;
    padding: 0 var(--sp-1);
    align-self: center;
  }
  /* The group's label sits before its members. Colour identifies; the name and
     the count say the same thing in words, which is what a screen reader and
     anyone who does not separate these hues actually receives. */
  .groupTag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: 0 0 auto;
    max-width: 160px;
    align-self: flex-end;
    font-size: var(--fs-caption);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: var(--radius-control) var(--radius-control) 0 0;
    background: var(--surface-2);
  }
  .swatch {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex: none;
    background: var(--text-faint);
  }
  .groupTag.tone-ok .swatch {
    background: var(--ok);
  }
  .groupTag.tone-warn .swatch {
    background: var(--warn);
  }
  .groupTag.tone-danger .swatch {
    background: var(--danger);
  }
  .groupTag.tone-info .swatch {
    background: var(--info);
  }
  .tab.grouped {
    border-top-left-radius: 0;
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
