<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { revealAfter, skeletonWidths } from "../skeleton";

  let {
    rows = 3,
    /**
     * One short bar in line, for a wait beside content that stays on screen —
     * a refresh must not empty the table it is refreshing.
     */
    compact = false,
  }: { rows?: number; compact?: boolean } = $props();

  // Nothing is drawn until the wait is long enough to be seen; unmounting
  // first — the data arriving — cancels the reveal (senales-de-vida D3).
  let shown = $state(false);
  $effect(() =>
    revealAfter(() => {
      shown = true;
    }),
  );
</script>

{#if shown}
  {#if compact}
    <span class="skeleton compact" role="status" aria-busy="true">
      <span class="bar" aria-hidden="true"></span>
      <span class="label">{$t("common.loading")}</span>
    </span>
  {:else}
    <div class="skeleton" role="status" aria-busy="true">
      {#each skeletonWidths(rows) as width, index (index)}
        <div class="row" aria-hidden="true">
          <span class="bar" style:width="{width}%"></span>
        </div>
      {/each}
      <span class="label">{$t("common.loading")}</span>
    </div>
  {/if}
{/if}

<style>
  .skeleton {
    display: grid;
    padding: var(--sp-2) 0;
  }
  /* The density of the table it stands in for: 32 px rows. */
  .row {
    height: var(--row-h);
    display: flex;
    align-items: center;
    padding: 0 var(--cell-pad);
    border-bottom: 1px solid var(--hair);
  }
  /* An opacity pulse on a flat bar. Never a sweeping gradient: the gradient
     belongs to the ambient ring alone. */
  .bar {
    display: block;
    height: 10px;
    border-radius: var(--radius-control);
    background: var(--surface-2);
    animation: skeleton-pulse var(--dur-pulse) var(--ease-out) infinite;
  }
  .compact {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
  }
  .compact .bar {
    width: 6em;
  }
  /* Said to a screen reader, not twice to the eye while the bars pulse. */
  .label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: reduce) {
    .bar {
      animation: none;
    }
    /* Still bars say nothing about progress: the word does. */
    .label {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip-path: none;
      color: var(--text-faint);
      font-size: var(--fs-caption);
      padding: var(--sp-1) var(--cell-pad);
    }
    .compact .label {
      padding: 0;
    }
  }
</style>
