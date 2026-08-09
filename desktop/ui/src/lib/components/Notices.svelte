<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { locale, t } from "../i18n";
  import { relativeTime } from "../time";
  import { dismissAllNotices, dismissNotice, holdNotice, notices, releaseNotice } from "../stores";
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
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="notice tone-{notice.tone}"
        class:transient={notice.tone === "info"}
        role="status"
        onmouseenter={() => holdNotice(notice.id)}
        onmouseleave={() => releaseNotice(notice.id, notice.tone)}
        onfocusin={() => holdNotice(notice.id)}
        onfocusout={() => releaseNotice(notice.id, notice.tone)}
      >
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
    /* In flow and at its natural height, as the shell's layout rule requires:
       these are bars, not an overlay. What changes is that each notice is a
       card inside the region instead of an edge-to-edge tinted band. */
    display: grid;
    gap: var(--sp-1);
    padding: var(--sp-1) var(--sp-3);
    max-height: 30vh;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .notice {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-3);
    font-size: var(--fs-dense);
    background: var(--surface);
    border: 1px solid var(--hair);
    /* The tone is a leading edge, not a wash: the text keeps the surface's own
       contrast and the colour is never the only carrier — the words are. */
    border-left: 3px solid var(--text-faint);
    border-radius: var(--radius-control);
  }
  .text {
    overflow-wrap: anywhere;
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
    border-left-color: var(--warn);
  }
  .tone-danger {
    border-left-color: var(--danger);
  }
  .tone-info {
    border-left-color: var(--info);
  }
  .more {
    display: flex;
    gap: var(--sp-3);
    padding: 0 var(--sp-1);
    font-size: var(--fs-caption);
  }
</style>
