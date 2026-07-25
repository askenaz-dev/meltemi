<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { binaryName } from "../agents";
  import { locale, t } from "../i18n";
  import { absoluteTime, durationLabel, relativeTime } from "../time";
  import {
    refreshSessions,
    sessions,
    type SessionInfo,
    type SessionState,
  } from "../stores";
  import type { ViewId } from "../registry";
  import Avatar from "../components/Avatar.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Icon from "../components/Icon.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";

  let {
    onOpen,
    onNavigate,
    onNewSession,
    onDirect,
  }: {
    onOpen: (sessionId: string) => void;
    onNavigate: (view: ViewId) => void;
    onNewSession: () => void;
    onDirect: (sessionId: string) => void;
  } = $props();

  type SortKey = "started" | "agent" | "state";

  let filter = $state("");
  let filterOpen = $state(false);
  let sortKey: SortKey = $state("started");
  let ascending = $state(false);
  let stateFilter: SessionState | null = $state(null);
  let cancelTarget: string | null = $state(null);
  let filterInput: HTMLInputElement | undefined = $state();
  let tick = $state(0);

  $effect(() => {
    void refreshSessions().catch(() => {});
    const timer = setInterval(() => {
      void refreshSessions().catch(() => {});
      tick += 1; // keeps relative times honest without re-fetching
    }, 5000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    if (filterOpen) filterInput?.focus();
  });

  const counts = $derived.by(() => {
    const map = new Map<SessionState, number>();
    for (const session of $sessions) {
      map.set(session.state, (map.get(session.state) ?? 0) + 1);
    }
    return map;
  });

  const rows = $derived.by(() => {
    void tick;
    const needle = filter.trim().toLowerCase();
    let list = $sessions.filter((session) => {
      if (stateFilter && session.state !== stateFilter) return false;
      if (!needle) return true;
      return (
        session.sessionId.toLowerCase().includes(needle) ||
        binaryName(session.agentCommand).toLowerCase().includes(needle) ||
        session.projectRoot.toLowerCase().includes(needle)
      );
    });
    const direction = ascending ? 1 : -1;
    list = [...list].sort((a, b) => {
      if (sortKey === "agent") {
        return direction * binaryName(a.agentCommand).localeCompare(binaryName(b.agentCommand));
      }
      if (sortKey === "state") return direction * a.state.localeCompare(b.state);
      return direction * a.startedAt.localeCompare(b.startedAt);
    });
    return list;
  });

  function sortBy(key: SortKey) {
    if (sortKey === key) ascending = !ascending;
    else {
      sortKey = key;
      ascending = key !== "started";
    }
  }

  function isLive(session: SessionInfo): boolean {
    return (
      session.state === "active" ||
      session.state === "starting" ||
      session.state === "waiting_permission"
    );
  }

  function cancel(sessionId: string) {
    cancelTarget = null;
    void invoke("daemon_notify", { method: "session/cancel", params: { sessionId } });
    setTimeout(() => void refreshSessions().catch(() => {}), 500);
  }

  function ariaSort(key: SortKey): "ascending" | "descending" | "none" {
    if (sortKey !== key) return "none";
    return ascending ? "ascending" : "descending";
  }
</script>

<div class="view">
  {#if $sessions.length === 0}
    <EmptyState
      glyph="sessions"
      title={$t("sessions.empty.title")}
      hint={$t("sessions.empty.hint")}
    >
      <button class="primary" onclick={onNewSession}>
        <Icon name="plus" size={14} />
        {$t("session.new")}
      </button>
      <button onclick={() => onNavigate("fleet")}>{$t("sessions.empty.fleet")}</button>
    </EmptyState>
  {:else}
    <div class="toolbar">
      <div class="chips" role="group" aria-label={$t("sessions.filterByState")}>
        <button
          class="pill"
          class:sel={stateFilter === null}
          aria-pressed={stateFilter === null}
          onclick={() => (stateFilter = null)}
        >
          {$t("sessions.all")} <span class="counter">{$sessions.length}</span>
        </button>
        {#each [...counts] as [state, count] (state)}
          <button
            class="pill"
            class:sel={stateFilter === state}
            aria-pressed={stateFilter === state}
            onclick={() => (stateFilter = stateFilter === state ? null : state)}
          >
            <StatusBadge {state} />
            <span class="counter">{count}</span>
          </button>
        {/each}
      </div>

      <div class="filter">
        {#if filterOpen}
          <input
            bind:this={filterInput}
            bind:value={filter}
            placeholder={$t("sessions.filterPlaceholder")}
            aria-label={$t("sessions.filterPlaceholder")}
            onkeydown={(event) => {
              event.stopPropagation();
              if (event.key === "Escape") {
                filter = "";
                filterOpen = false;
              }
            }}
          />
        {:else}
          <button class="ghost" onclick={() => (filterOpen = true)}>
            <Icon name="search" size={14} />
            <kbd>/</kbd>
          </button>
        {/if}
      </div>
    </div>

    <div class="tableWrap">
      <table class="dense">
        <thead>
          <tr>
            <th scope="col">{$t("sessions.col.session")}</th>
            <th scope="col" aria-sort={ariaSort("agent")}>
              <button class="ghost sortBtn" onclick={() => sortBy("agent")}>
                {$t("sessions.col.agent")}
              </button>
            </th>
            <th scope="col" aria-sort={ariaSort("state")}>
              <button class="ghost sortBtn" onclick={() => sortBy("state")}>
                {$t("sessions.col.state")}
              </button>
            </th>
            <th scope="col">{$t("sessions.col.duration")}</th>
            <th scope="col" aria-sort={ariaSort("started")}>
              <button class="ghost sortBtn" onclick={() => sortBy("started")}>
                {$t("sessions.col.started")}
              </button>
            </th>
            <th scope="col" class="right">{$t("sessions.col.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as session (session.sessionId)}
            <tr>
              <td>
                <button class="link" onclick={() => onOpen(session.sessionId)}>
                  <code>{session.sessionId.slice(0, 8)}</code>
                </button>
              </td>
              <td>
                <span class="agent">
                  <Avatar
                    id={binaryName(session.agentCommand)}
                    name={binaryName(session.agentCommand)}
                    size={20}
                  />
                  {binaryName(session.agentCommand)}
                </span>
              </td>
              <td>
                <StatusBadge state={session.state} />
                {#if session.resumable && !isLive(session)}
                  <span class="pill info">{$t("sessions.resumable")}</span>
                {/if}
              </td>
              <td class="faint">{durationLabel(session.startedAt, session.endedAt)}</td>
              <td>
                <time
                  datetime={session.startedAt}
                  title={absoluteTime(session.startedAt, $locale)}
                  aria-label={absoluteTime(session.startedAt, $locale)}
                >
                  {relativeTime(session.startedAt, $locale)}
                </time>
              </td>
              <td class="right actions">
                {#if isLive(session)}
                  <button class="ghost" onclick={() => onDirect(session.sessionId)}>
                    {$t("sessions.direct")}
                  </button>
                  <button
                    class="ghost destructive"
                    onclick={() => (cancelTarget = session.sessionId)}
                  >
                    {$t("sessions.cancelShort")}
                  </button>
                {:else if session.resumable}
                  <button class="ghost" onclick={() => onDirect(session.sessionId)}>
                    {$t("sessions.resume")}
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if rows.length === 0}
        <p class="none">{$t("sessions.noMatch")}</p>
      {/if}
    </div>
  {/if}
</div>

{#if cancelTarget}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("sessions.cancel.warning")}
    confirmLabel={$t("sessions.cancel")}
    onConfirm={() => cancelTarget && cancel(cancelTarget)}
    onCancel={() => (cancelTarget = null)}
  />
{/if}

<style>
  .view {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-2) 0;
    flex-wrap: wrap;
  }
  .chips {
    display: flex;
    gap: var(--sp-1);
    flex-wrap: wrap;
  }
  .chips .pill {
    border: 1px solid transparent;
    cursor: pointer;
    display: inline-flex;
    gap: var(--sp-1);
    align-items: center;
  }
  .chips .pill.sel {
    border-color: var(--accent);
    color: var(--text);
  }
  .filter {
    margin-left: auto;
  }
  .filter button {
    display: inline-flex;
    gap: var(--sp-1);
    align-items: center;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-caption);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    color: var(--text-faint);
  }
  .tableWrap {
    overflow: auto;
    min-height: 0;
    padding: 0 var(--sp-2);
  }
  .sortBtn {
    font: inherit;
    font-size: var(--fs-caption);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-faint);
    padding: 0;
  }
  .agent {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  .right {
    text-align: right;
  }
  .actions {
    white-space: nowrap;
  }
  .actions button {
    font-size: var(--fs-caption);
  }
  .faint {
    color: var(--text-faint);
  }
  .none {
    margin: var(--sp-4);
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
</style>
