<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../i18n";
  import Icon from "./Icon.svelte";

  let {
    label,
    onClose,
    children,
  }: { label: string; onClose: () => void; children: Snippet } = $props();

  let panel: HTMLElement | null = $state(null);

  // Opening the drawer moves the focus into it, so a keyboard user is where the
  // detail is; Escape (below) returns them to the list.
  $effect(() => {
    panel?.focus();
  });

</script>

<svelte:window
  onkeydown={(event) => {
    // The drawer is the innermost dismissible surface: it takes Escape before
    // the shell does, whatever holds the focus.
    if (event.key === "Escape") {
      event.stopPropagation();
      event.preventDefault();
      onClose();
    }
  }}
/>

<aside
  aria-label={label}
  bind:this={panel}
  tabindex="-1"
>
  <header>
    <span class="title">{label}</span>
    <button class="ghost" aria-label={$t("common.close")} onclick={onClose}>
      <Icon name="close" size={14} />
    </button>
  </header>
  <div class="body">{@render children()}</div>
</aside>

<style>
  aside {
    width: 268px;
    flex: none;
    border-left: 1px solid var(--hair);
    background: var(--panel);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--hair);
  }
  .title {
    font-size: var(--fs-dense);
    font-weight: 500;
  }
  .body {
    padding: var(--panel-pad);
    /* One axis. A fixed-width detail panel that scrolls sideways hides half of
       every line behind a gesture nobody makes in a drawer. */
    overflow-y: auto;
    overflow-x: hidden;
    display: grid;
    gap: var(--sp-3);
    align-content: start;
    min-height: 0;
  }
  /* Grid items default to min-width: auto and cannot shrink below their own
     content, which is what pushed a long path past the panel. */
  .body > :global(*) {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
