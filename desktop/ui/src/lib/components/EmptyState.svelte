<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    glyph,
    title,
    hint,
    children,
  }: { glyph: string; title: string; hint: string; children?: Snippet } =
    $props();
</script>

<!-- A labeled empty state teaching the next step: never a dead end. -->
<div class="empty">
  <p class="glyph" aria-hidden="true">{glyph}</p>
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
    text-align: center;
    gap: var(--sp-3);
    height: 100%;
    padding: var(--sp-8);
  }
  .glyph {
    margin: 0;
    font-size: 2rem;
    color: var(--text-muted);
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    max-width: 48ch;
  }
  .actions {
    display: flex;
    justify-content: center;
    gap: var(--sp-2);
  }
</style>
