<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { request } from "../daemon";
  import { t } from "../i18n";
  import {
    pending,
    pushNotice,
    refreshPending,
    type PermissionRule,
  } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

  let persistRules: Record<string, boolean> = $state({});

  $effect(() => {
    void refreshPending().catch(() => {});
    const timer = setInterval(() => {
      void refreshPending().catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  });

  function ruleLabel(rule: PermissionRule): string {
    const match = rule.tool ?? rule.commandPrefix ?? rule.pathPrefix ?? "*";
    return `${rule.effect}: ${match}`;
  }

  async function decide(
    requestId: string,
    optionId: string,
    persistRule?: PermissionRule,
  ) {
    try {
      await request("permission/decide", {
        requestId,
        optionId,
        persistRule,
      });
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
    glyph="permissions"
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
          {:else if item.expiresInSeconds !== undefined}
            <span class="timer">
              {$t("permissions.expiresIn", { s: item.expiresInSeconds })}
            </span>
          {:else}
            <span class="timer">{$t("permissions.waitingHuman")}</span>
          {/if}
        </div>
        <p class="meta">
          {$t("permissions.tool")}: <code>{item.tool}</code> ·
          {$t("permissions.session")}: <code>{item.sessionId.slice(0, 8)}</code> ·
          {$t("permissions.waitingFor", { s: item.waitingSeconds })}
        </p>
        {#if !item.expired}
          {#if item.suggestedRule}
            <label class="rule">
              <input type="checkbox" bind:checked={persistRules[item.requestId]} />
              {$t("permissions.persistRule")}
              <code>{ruleLabel(item.suggestedRule)}</code>
            </label>
          {/if}
          <div class="options">
            {#each item.options as option (option.optionId)}
              <button
                data-autofocus={index === 0 ? "true" : undefined}
                onclick={() =>
                  void decide(
                    item.requestId,
                    option.optionId,
                    persistRules[item.requestId] ? item.suggestedRule : undefined,
                  )}
              >
                {option.name}
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
    padding: var(--panel-pad);
    display: grid;
    gap: var(--sp-3);
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  li {
    background: var(--surface);
    border: 1px solid var(--hair);
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
  .rule {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
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
  /* Hard rule: the tray never animates its layout — nothing moves under
     the cursor while a permission is being decided. */
  li,
  .options {
    transition: none !important;
  }
  .rule input {
    width: auto;
  }
</style>
