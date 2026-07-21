<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import type { MessageKey } from "../messages";
  import type { SessionState } from "../stores";

  let { state }: { state: SessionState } = $props();

  const glyphs: Record<SessionState, string> = {
    starting: "◌",
    active: "▸",
    waiting_permission: "●",
    ended: "■",
    interrupted: "▲",
  };
  const tones: Record<SessionState, string> = {
    starting: "info",
    active: "ok",
    waiting_permission: "warn",
    ended: "muted",
    interrupted: "danger",
  };
</script>

<!-- Symbol + word, never color alone (design system). -->
<span class="badge tone-{tones[state]}">
  <span aria-hidden="true">{glyphs[state]}</span>
  {$t(("state." + state) as MessageKey)}
</span>

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
