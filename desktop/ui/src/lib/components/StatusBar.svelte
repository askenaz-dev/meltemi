<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The status bar answers two questions without changing view: what am I working
  on, and what is waiting for me. It says nothing it cannot measure — a zero
  where nothing was reported would be a claim about an agent that does not
  count (barra-de-estado-agentica design D3).
-->
<script lang="ts">
  import { conn } from "../daemon";
  import { t } from "../i18n";
  import {
    activeProject,
    changes,
    gateWaiting,
    pending,
    sessions,
    usageToday,
  } from "../stores";
  import { projectName } from "../tree";
  import type { ViewId } from "../registry";

  let { onNavigate }: { onNavigate?: (view: ViewId) => void } = $props();

  // Split in two, because they mean different things to the person reading:
  // one is the agent working, the other is the agent waiting on them. The old
  // single "running" count folded both, which is why a bar full of running
  // sessions could mean nobody was doing anything.
  const working = $derived(
    $sessions.filter((s) => s.state === "active" || s.state === "starting").length,
  );
  const waiting = $derived($sessions.filter((s) => s.state === "waiting_permission").length);
  // A third figure rather than a third meaning folded into an existing one.
  // A session between turns is also waiting on you, but it owes you nothing —
  // it is waiting for you to type, not to decide. Folding it into `waiting`
  // would repeat exactly the mistake the split above was made to undo.
  const idle = $derived(
    $sessions.filter((s) => s.state === "waiting_instruction").length,
  );

  // The change the bar names is the one asking for a decision; the criterion
  // lives in the store so both readers share it (design D2).
  const gate = $derived(gateWaiting($changes));
  const activeChanges = $derived($changes.filter((c) => !c.archived).length);

  const go = (view: ViewId) => onNavigate?.(view);
</script>

<footer>
  {#if $conn.state === "connected"}
    <span class="ok"><span aria-hidden="true">▸</span> {$t("conn.connected")}</span>
    <span class="muted version">v{$conn.version}</span>
    <span class="mono endpoint" title={$t("conn.endpoint")}>{$conn.endpoint ?? ""}</span>
  {:else if $conn.state === "connecting"}
    <span class="info"><span aria-hidden="true">◌</span> {$t("conn.connecting")}</span>
  {:else}
    <span class="danger"><span aria-hidden="true">▲</span> {$t("conn.unreachable")}</span>
    <span class="mono endpoint">{$conn.endpoint}</span>
  {/if}

  <!-- Context: where the work is happening. -->
  {#if $activeProject}
    <button class="seg project" title={$activeProject} onclick={() => go("project")}>
      {projectName($activeProject)}
    </button>
  {/if}

  <!-- The gate belongs to a change, never to a session: it is what a person
       can act on right now, so it earns a segment of its own. -->
  {#if gate}
    <button class="seg gate warn" onclick={() => go("project")}>
      {gate.name} · {$t("status.gate", { artifact: gate.gateArtifact ?? "" })}
    </button>
  {:else if activeChanges > 0}
    <button class="seg" onclick={() => go("project")}>
      {$t("status.changes", { n: activeChanges })}
    </button>
  {/if}

  <span class="right muted">
    <!-- Measured or silent. A zero here would be a claim about an agent that
         does not count its tokens, and ACP carries no usage for levels 1 and 2
         — which is most of the fleet (design D3). -->
    {#if $usageToday?.total}
      <button class="seg usage" onclick={() => go("analytics")}>
        {$t("status.usage", { tokens: String($usageToday.total) })}
      </button>
      ·
    {:else if $usageToday && $usageToday.unreported > 0}
      <button class="seg usage faint" onclick={() => go("analytics")}>
        {$t("status.usage.unreported")}
      </button>
      ·
    {/if}
    <button class="seg" onclick={() => go("sessions")}>
      {$t("status.working", { n: working })}
      {#if waiting > 0}
        · {$t("status.waiting", { n: waiting })}
      {/if}
      {#if idle > 0}
        · {$t("status.idle", { n: idle })}
      {/if}
    </button>
    {#if $pending.length > 0}
      · <button class="seg warn" onclick={() => go("permissions")}>
        {$t("status.permissions", { n: $pending.length })}
      </button>
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
    display: flex;
    align-items: center;
    gap: var(--sp-1);
  }
  /* A segment reads as text and behaves as a control: it names something with
     a view of its own, so activating it goes there (design D5). */
  .seg {
    border: 0;
    background: transparent;
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .seg:hover {
    color: var(--text);
  }
  .endpoint {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40ch;
    color: var(--text-faint);
  }
  /* The declared order in which width is given up: the endpoint first, then
     the version, then the project's name. Connection and pending decisions
     never yield — the same priority the terminal's size floor applies
     (design D6). */
  @media (max-width: 1100px) {
    .endpoint {
      display: none;
    }
  }
  @media (max-width: 980px) {
    .version {
      display: none;
    }
  }
  @media (max-width: 940px) {
    .usage {
      display: none;
    }
  }
  @media (max-width: 900px) {
    .project {
      display: none;
    }
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
  .faint {
    color: var(--text-faint);
  }
</style>
