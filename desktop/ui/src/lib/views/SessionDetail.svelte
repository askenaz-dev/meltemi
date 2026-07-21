<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { conn, request } from "../daemon";
  import { t } from "../i18n";
  import {
    onSessionEvent,
    refreshSessions,
    sessions,
    type SessionInfo,
  } from "../stores";
  import StatusBadge from "../components/StatusBadge.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";

  let { sessionId, onBack }: { sessionId: string; onBack: () => void } =
    $props();

  const session = $derived(
    $sessions.find((candidate) => candidate.sessionId === sessionId),
  );

  let lines: string[] = $state([]);
  let confirmCancel = $state(false);
  let gone = $state(false);
  let atBottom = $state(true);
  let newLines = $state(0);
  let scroller: HTMLDivElement | undefined = $state();
  let wasStreaming = false;
  let wasUnreachable = false;

  function summarize(raw: string): string {
    try {
      const event = JSON.parse(raw) as { ts?: string; type?: string };
      return `${event.ts ?? ""}  ${event.type ?? "event"}`;
    } catch {
      return raw;
    }
  }

  function append(line: string) {
    lines = [...lines, line];
    if (atBottom) {
      queueMicrotask(() => {
        scroller?.scrollTo({ top: scroller.scrollHeight });
      });
    } else {
      newLines += 1;
    }
  }

  // Initial transcript from the persistent log.
  $effect(() => {
    const info: SessionInfo | undefined = session;
    if (!info) return;
    void request<{ lines: string[] }>("session/log", {
      projectRoot: info.projectRoot,
      sessionId,
    })
      .then((result) => {
        lines = result.lines.map(summarize);
        queueMicrotask(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
      })
      .catch(() => {
        // A session mid-start may not have a log yet; live events fill in.
      });
  });

  // Live events for this session.
  $effect(() => {
    return onSessionEvent((message) => {
      if (message.sessionId !== sessionId) return;
      wasStreaming = true;
      append(`${new Date().toISOString().slice(0, 19)}  ${message.event.type}`);
    });
  });

  // Honest cut on daemon loss; honest "gone" after reconnection.
  $effect(() => {
    const state = $conn.state;
    if (state === "unreachable" && !wasUnreachable) {
      wasUnreachable = true;
      if (wasStreaming) {
        append($t("sessions.detail.cut"));
      }
    } else if (state === "connected" && wasUnreachable) {
      wasUnreachable = false;
      void refreshSessions().then(() => {
        if (!session) {
          gone = true;
        }
      });
    }
  });

  function onScroll() {
    if (!scroller) return;
    const nearBottom =
      scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 8;
    atBottom = nearBottom;
    if (nearBottom) newLines = 0;
  }

  function jumpDown() {
    scroller?.scrollTo({ top: scroller.scrollHeight });
    atBottom = true;
    newLines = 0;
  }

  function cancelSession() {
    confirmCancel = false;
    void invoke("daemon_notify", {
      method: "session/cancel",
      params: { sessionId },
    });
    setTimeout(() => void refreshSessions().catch(() => {}), 500);
  }
</script>

<section aria-label={$t("sessions.detail.transcript")}>
  <header>
    <div>
      <code>{sessionId}</code>
      {#if session}
        <StatusBadge state={session.state} />
      {/if}
    </div>
    <div class="actions">
      {#if session && (session.state === "active" || session.state === "starting" || session.state === "waiting_permission")}
        <button class="destructive" onclick={() => (confirmCancel = true)}>
          {$t("sessions.cancel")}
        </button>
      {/if}
      <button onclick={onBack}>{$t("common.back")}</button>
    </div>
  </header>

  {#if gone}
    <p class="danger" role="alert">{$t("sessions.detail.goneAfterReconnect")}</p>
  {/if}

  <div
    class="transcript"
    bind:this={scroller}
    onscroll={onScroll}
    role="log"
    aria-live="polite"
  >
    {#each lines as line, index (index)}
      <div class="line">{line}</div>
    {/each}
  </div>

  {#if !atBottom && newLines > 0}
    <button class="follow" onclick={jumpDown}>
      ↓ {newLines}
    </button>
  {/if}
</section>

{#if confirmCancel}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("sessions.cancel.warning")}
    confirmLabel={$t("sessions.cancel")}
    onConfirm={cancelSession}
    onCancel={() => (confirmCancel = false)}
  />
{/if}

<style>
  section {
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: var(--sp-3);
    height: 100%;
    position: relative;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-3);
  }
  header div {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .actions button {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .destructive {
    border-color: var(--danger) !important;
    color: var(--danger) !important;
  }
  .danger {
    margin: 0;
    color: var(--danger);
  }
  .transcript {
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: var(--sp-3);
    font-family: var(--font-mono);
    font-size: 0.8125rem;
    min-height: 0;
  }
  .line {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .follow {
    position: absolute;
    right: var(--sp-4);
    bottom: var(--sp-4);
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--accent);
    background: var(--surface);
    color: var(--accent);
    cursor: pointer;
  }
</style>
