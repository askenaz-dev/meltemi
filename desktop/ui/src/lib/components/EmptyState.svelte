<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { IconName } from "../icons";
  import Icon from "./Icon.svelte";

  let {
    glyph,
    title,
    hint,
    children,
  }: { glyph: IconName; title: string; hint: string; children?: Snippet } = $props();
</script>

<!-- A labeled empty state that teaches the next step and offers it as a
     control: never a dead end, never text-only. -->
<div class="empty">
  <span class="glyph"><Icon name={glyph} size={28} /></span>
  <h2>{title}</h2>
  <p class="hint">{hint}</p>
  {#if children}
    <div class="actions">{@render children()}</div>
  {/if}
</div>

<style>
  .empty {
    display: grid;
    place-content: center;
    justify-items: center;
    text-align: center;
    gap: var(--sp-3);
    height: 100%;
    padding: var(--sp-8);
  }
  .glyph {
    color: var(--text-faint);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    max-width: 52ch;
    font-size: var(--fs-dense);
  }
  .actions {
    display: flex;
    /* Never stretch one action to the height of another — also when the row
       wraps, where each wrapped line would otherwise stretch on its own. */
    align-items: center;
    gap: var(--sp-2);
    flex-wrap: wrap;
    justify-content: center;
  }
</style>
