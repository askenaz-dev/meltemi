<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import type { MessageKey } from "../messages";
  import type { SessionState } from "../stores";

  let {
    state,
    /**
     * Compact form: the glyph alone, with the word carried as the accessible
     * name. For places where the word would be repeated once per row and eat
     * the width the row's own label needs — a tab strip of six sessions of the
     * same agent (piel-de-pestanas design D6). The rule is symbol plus label,
     * visible OR accessible; never colour alone, which the glyph already
     * prevents.
     */
    compact = false,
  }: { state: SessionState; compact?: boolean } = $props();

  const glyphs: Record<SessionState, string> = {
    starting: "◌",
    active: "▸",
    waiting_permission: "●",
    waiting_instruction: "❯",
    ended: "■",
    interrupted: "▲",
  };
  const tones: Record<SessionState, string> = {
    starting: "info",
    active: "ok",
    waiting_permission: "warn",
    // Alive and yours: not a warning (nothing is owed to the agent) and not
    // muted (the session has not ended). Its own tone, so a glance separates
    // "waiting on you to decide" from "waiting on you to type".
    waiting_instruction: "accent",
    ended: "muted",
    interrupted: "danger",
  };
</script>

<!-- Symbol + word, never color alone (design system). -->
{#if compact}
  <span
    class="badge tone-{tones[state]}"
    aria-label={$t(("state." + state) as MessageKey)}
    title={$t(("state." + state) as MessageKey)}
  >
    <span aria-hidden="true">{glyphs[state]}</span>
  </span>
{:else}
  <span class="badge tone-{tones[state]}">
    <span aria-hidden="true">{glyphs[state]}</span>
    {$t(("state." + state) as MessageKey)}
  </span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    white-space: nowrap;
  }
  .tone-ok {
    color: var(--ok);
  }
  .tone-warn {
    color: var(--warn);
  }
  .tone-danger {
    color: var(--danger);
  }
  .tone-info {
    color: var(--info);
  }
  .tone-muted {
    color: var(--text-muted);
  }
</style>
