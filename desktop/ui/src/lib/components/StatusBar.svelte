<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { conn } from "../daemon";
  import { t } from "../i18n";
  import { pending, sessions } from "../stores";

  // Running, not total: a session waiting on a permission is still running,
  // and the label says "running" so the number can never contradict the table.
  const live = $derived(
    $sessions.filter(
      (s) => s.state === "active" || s.state === "starting" || s.state === "waiting_permission",
    ).length,
  );
</script>

<footer>
  {#if $conn.state === "connected"}
    <span class="ok"><span aria-hidden="true">▸</span> {$t("conn.connected")}</span>
    <span class="muted">v{$conn.version}</span>
    <span class="mono endpoint" title={$t("conn.endpoint")}>{$conn.endpoint ?? ""}</span>
  {:else if $conn.state === "connecting"}
    <span class="info"><span aria-hidden="true">◌</span> {$t("conn.connecting")}</span>
  {:else}
    <span class="danger"><span aria-hidden="true">▲</span> {$t("conn.unreachable")}</span>
    <span class="mono endpoint">{$conn.endpoint}</span>
  {/if}

  <span class="right muted">
    {$t("status.sessions", { n: live })}
    {#if $pending.length > 0}
      · <span class="warn">{$t("status.permissions", { n: $pending.length })}</span>
    {/if}
  </span>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-1) var(--sp-4);
    border-top: 1px solid var(--hair);
    background: var(--panel);
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .right {
    margin-left: auto;
  }
  .endpoint {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40ch;
    color: var(--text-faint);
  }
  .ok {
    color: var(--ok);
  }
  .info {
    color: var(--info);
  }
  .danger {
    color: var(--danger);
  }
  .warn {
    color: var(--warn);
  }
  .muted {
    color: var(--text-muted);
  }
</style>
