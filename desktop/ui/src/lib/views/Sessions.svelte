<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { refreshSessions, sessions } from "../stores";
  import StatusBadge from "../components/StatusBadge.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import type { ViewId } from "../registry";

  let {
    onOpen,
    onNavigate,
  }: { onOpen: (sessionId: string) => void; onNavigate: (view: ViewId) => void } =
    $props();

  $effect(() => {
    void refreshSessions().catch(() => {});
    const timer = setInterval(() => {
      void refreshSessions().catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  });

  function agentLabel(command: string[]): string {
    const program = command[0] ?? "?";
    const parts = program.replaceAll("\\", "/").split("/");
    return parts[parts.length - 1] ?? program;
  }

  function projectLabel(root: string): string {
    const parts = root.replaceAll("\\", "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? root;
  }
</script>

{#if $sessions.length === 0}
  <EmptyState
    glyph="◌"
    title={$t("sessions.empty.title")}
    hint={$t("sessions.empty.hint")}
  >
    <button onclick={() => onNavigate("fleet")}>
      {$t("sessions.empty.fleet")}
    </button>
  </EmptyState>
{:else}
  <table>
    <thead>
      <tr>
        <th scope="col">{$t("sessions.col.session")}</th>
        <th scope="col">{$t("sessions.col.agent")}</th>
        <th scope="col">{$t("sessions.col.state")}</th>
        <th scope="col">{$t("sessions.col.project")}</th>
        <th scope="col">{$t("sessions.col.started")}</th>
      </tr>
    </thead>
    <tbody>
      {#each $sessions as session (session.sessionId)}
        <tr>
          <td>
            <button class="link" onclick={() => onOpen(session.sessionId)}>
              <code>{session.sessionId.slice(0, 8)}</code>
            </button>
          </td>
          <td>{agentLabel(session.agentCommand)}</td>
          <td>
            <StatusBadge state={session.state} />
            {#if session.resumable && (session.state === "ended" || session.state === "interrupted")}
              <span class="resumable">{$t("sessions.resumable")}</span>
            {/if}
          </td>
          <td>{projectLabel(session.projectRoot)}</td>
          <td><time datetime={session.startedAt}>{session.startedAt.slice(0, 19).replace("T", " ")}</time></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }
  th {
    text-align: left;
    color: var(--text-muted);
    font-weight: 500;
    font-size: 0.8125rem;
    padding: var(--sp-2);
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: var(--sp-2);
    border-bottom: 1px solid var(--border);
    height: 32px;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  .link {
    font: inherit;
    border: 0;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    padding: 0;
  }
  .resumable {
    color: var(--info);
    font-size: 0.75rem;
    margin-left: var(--sp-2);
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
