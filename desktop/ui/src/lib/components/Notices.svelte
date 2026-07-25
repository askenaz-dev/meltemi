<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { locale, t } from "../i18n";
  import { relativeTime } from "../time";
  import { dismissAllNotices, dismissNotice, notices } from "../stores";
  import Icon from "./Icon.svelte";

  /** Visible cap: older notices collapse into a counter with history. */
  const VISIBLE = 3;

  let expanded = $state(false);

  const ordered = $derived([...$notices].reverse());
  const shown = $derived(expanded ? ordered : ordered.slice(0, VISIBLE));
  const hidden = $derived(Math.max(0, ordered.length - shown.length));
</script>

{#if ordered.length > 0}
  <div class="notices" role="region" aria-label={$t("notices.title")} aria-live="polite">
    {#each shown as notice (notice.id)}
      <div class="notice tone-{notice.tone}">
        <span class="text">{notice.text}</span>
        <time class="at" datetime={new Date(notice.at).toISOString()}>
          {relativeTime(new Date(notice.at).toISOString(), $locale)}
        </time>
        <button class="ghost" aria-label={$t("notices.dismiss")} onclick={() => dismissNotice(notice.id)}>
          <Icon name="close" size={12} />
        </button>
      </div>
    {/each}
    {#if hidden > 0 || expanded}
      <div class="more">
        <button class="link" onclick={() => (expanded = !expanded)}>
          {expanded ? $t("notices.collapse") : $t("notices.more", { n: hidden })}
        </button>
        <button class="link" onclick={dismissAllNotices}>{$t("notices.dismissAll")}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Signal priority 3: inline, dismissible, and never animated — nothing may
     move under the cursor while a permission is being decided. */
  .notices {
    display: grid;
    gap: 1px;
    background: var(--hair);
    max-height: 30vh;
    overflow: auto;
  }
  .notice {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-4);
    font-size: var(--fs-dense);
    background: var(--surface-2);
  }
  .text {
    flex: 1;
  }
  .at {
    color: var(--text-faint);
    font-size: var(--fs-caption);
    white-space: nowrap;
  }
  .tone-warn {
    color: var(--warn);
    background: var(--tint-warn);
  }
  .tone-danger {
    color: var(--danger);
    background: var(--tint-danger);
  }
  .tone-info {
    color: var(--info);
    background: var(--tint-info);
  }
  .more {
    display: flex;
    gap: var(--sp-3);
    padding: var(--sp-1) var(--sp-4);
    background: var(--panel);
    font-size: var(--fs-caption);
  }
</style>
