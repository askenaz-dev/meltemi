<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  A context chip: the label of what it selects, the current value, and a menu
  of the alternatives. Chips live INSIDE the composer (lanzador-conversacional),
  so the context of a launch is read and changed without leaving the sentence
  being written. The menu never animates its layout and closes on Escape or on
  a click outside — nothing moves under the cursor and nothing traps it.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    label,
    value,
    title,
    tone = "plain",
    menu,
    lead,
  }: {
    /** What this chip selects, always visible: the chip is not a mystery icon. */
    label: string;
    /** The current selection, in the user's words. */
    value: string;
    title?: string;
    /** `warn` marks a selection that cannot be launched as it stands. */
    tone?: "plain" | "warn";
    /** The menu body; receives the closer so an item can dismiss the popover. */
    menu: Snippet<[() => void]>;
    /** Optional glyph rendered before the label (an avatar, for instance). */
    lead?: Snippet;
  } = $props();

  let open = $state(false);
  let host: HTMLDivElement | undefined = $state();

  function close(): void {
    open = false;
  }

  // A click outside or Escape closes it. Registered only while open, so the
  // composer pays nothing for chips that are shut.
  $effect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      if (host && !host.contains(event.target as Node)) close();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      close();
      host?.querySelector<HTMLButtonElement>("button.face")?.focus();
    };
    window.addEventListener("mousedown", away);
    window.addEventListener("keydown", escape, true);
    return () => {
      window.removeEventListener("mousedown", away);
      window.removeEventListener("keydown", escape, true);
    };
  });
</script>

<div class="chip" bind:this={host}>
  <button
    class="face"
    class:warn={tone === "warn"}
    class:open
    aria-expanded={open}
    aria-haspopup="menu"
    {title}
    onclick={() => (open = !open)}
  >
    {#if lead}<span class="lead">{@render lead()}</span>{/if}
    <span class="label">{label}</span>
    <span class="value">{value}</span>
    <Icon name="chevronDown" size={12} />
  </button>

  {#if open}
    <div class="menu" role="menu" tabindex="-1">
      {@render menu(close)}
    </div>
  {/if}
</div>

<style>
  .chip {
    position: relative;
  }
  .face {
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 6px;
    max-width: 26ch;
    padding: 2px var(--sp-2);
    background: var(--surface-2);
    border-color: transparent;
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .face.open {
    border-color: var(--border);
  }
  .face.warn {
    border-color: var(--warn);
    color: var(--warn);
  }
  .label {
    flex: none;
    color: var(--text-faint);
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 500;
  }
  .lead {
    display: inline-flex;
    flex: none;
  }
  .menu {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 0;
    z-index: 20;
    min-width: 260px;
    max-width: min(420px, 80vw);
    max-height: 46vh;
    overflow-y: auto;
    padding: var(--sp-1);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    /* Hard rule: a menu never animates its geometry. */
    transition: none !important;
  }
</style>
