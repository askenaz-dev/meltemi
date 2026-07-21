<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { request } from "../daemon";
  import { t } from "../i18n";
  import { pending, pushNotice, refreshPending } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

  $effect(() => {
    void refreshPending().catch(() => {});
    const timer = setInterval(() => {
      void refreshPending().catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  });

  async function decide(requestId: string, optionId: string) {
    try {
      await request("permission/decide", { requestId, optionId });
      pushNotice($t("permissions.decided"), "info");
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    }
    await refreshPending().catch(() => {});
  }
</script>

{#if $pending.length === 0}
  <EmptyState
    glyph="●"
    title={$t("permissions.empty.title")}
    hint={$t("permissions.empty.hint")}
  />
{:else}
  <ul aria-label={$t("permissions.title")}>
    {#each $pending as item, index (item.requestId)}
      <li class:expired={item.expired}>
        <div class="head">
          <strong>{item.summary}</strong>
          {#if item.expired}
            <span class="expiredTag">! {$t("permissions.expired")}</span>
          {:else}
            <span class="timer">
              {$t("permissions.expiresIn", { s: item.expiresInSeconds })}
            </span>
          {/if}
        </div>
        <p class="meta">
          {$t("permissions.tool")}: <code>{item.tool}</code> ·
          {$t("permissions.session")}: <code>{item.sessionId.slice(0, 8)}</code> ·
          {$t("permissions.waitingFor", { s: item.waitingSeconds })}
        </p>
        {#if !item.expired}
          <div class="options">
            {#each item.options as option (option.optionId)}
              <button
                data-autofocus={index === 0 ? "true" : undefined}
                onclick={() => void decide(item.requestId, option.optionId)}
              >
                {option.label ?? option.optionId}
              </button>
            {/each}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--sp-3);
    align-content: start;
  }
  li {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: var(--sp-4);
    display: grid;
    gap: var(--sp-2);
  }
  li.expired {
    border-color: var(--warn);
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-3);
    align-items: baseline;
  }
  .expiredTag {
    color: var(--warn);
    white-space: nowrap;
  }
  .timer {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .meta {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.8125rem;
  }
  code {
    font-family: var(--font-mono);
  }
  .options {
    display: flex;
    gap: var(--sp-2);
    flex-wrap: wrap;
  }
  button {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
</style>
