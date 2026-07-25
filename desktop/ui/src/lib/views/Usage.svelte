<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The usage panel (analitica-consumo-local design D7): dense tables of the
  design system, no chart dependency, tabular numerals so figures do not jump
  when they refresh. Two rules are visible in the markup: an absent counter is
  rendered with symbol + word and its reason — never as a zero — and the
  honesty disclosure sits next to the numbers, with no interaction to reveal it.
-->
<script lang="ts">
  import { request } from "../daemon";
  import { locale, t } from "../i18n";
  import type { MessageKey } from "../messages";
  import { activeProject, projects, refreshProjects } from "../stores";
  import { projectName } from "../tree";
  import EmptyState from "../components/EmptyState.svelte";
  import Icon from "../components/Icon.svelte";

  type Granularity = "day" | "week" | "month" | "total";

  interface Tokens {
    input?: number;
    output?: number;
    cachedInput?: number;
    reasoning?: number;
    total?: number;
  }

  interface Activity {
    sessions: number;
    sessionsClosed: number;
    sessionsOpen: number;
    activeSeconds: number;
    prompts: number;
    turnsByStopReason?: Record<string, number>;
    permissionsRequested: number;
    permissionsApproved: number;
    permissionsDenied: number;
    permissionsExpired: number;
    humanEdits: number;
    commits: number;
    checkpoints: number;
    errors: number;
  }

  type UnreportedKind =
    | "protocol_carries_no_usage"
    | "level_runs_no_process"
    | "stream_declared_no_counters";

  interface Coverage {
    measuredSessions: number;
    unreportedSessions: number;
    reasons?: { kind: UnreportedKind; sessions: number }[];
  }

  interface Cell {
    projectKey: string;
    projectRoot: string;
    agent: string;
    agentId?: string;
    profile?: string;
    level?: number;
    period: string;
    activity: Activity;
    tokens?: Tokens;
    coverage: Coverage;
  }

  interface Unattributed {
    projectKey: string;
    projectRoot: string;
    period: string;
    humanEdits: number;
  }

  interface UsageResult {
    cells: Cell[];
    unattributed?: Unattributed[];
    totals: { cells: number; activity: Activity; tokens?: Tokens; coverage: Coverage };
    truncated: boolean;
    disclosure: string[];
  }

  const PERIODS: Granularity[] = ["day", "week", "month", "total"];

  let granularity: Granularity = $state("month");
  let allProjects = $state(false);
  let agentFilter = $state("");
  let profileFilter = $state("");
  let result = $state<UsageResult | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  $effect(() => {
    void refreshProjects().catch(() => {});
  });

  // Refetch whenever a control changes; the daemon does the aggregation.
  $effect(() => {
    const params: Record<string, unknown> = { granularity };
    if (!allProjects && $activeProject) params.projectRoot = $activeProject;
    if (agentFilter) params.agent = agentFilter;
    if (profileFilter) params.profile = profileFilter;
    loading = true;
    void request<UsageResult>("analytics/usage", params)
      .then((answer) => {
        result = answer;
        error = null;
      })
      .catch((raw: { detail?: string; message?: string }) => {
        error = raw?.detail ?? raw?.message ?? String(raw);
      })
      .finally(() => (loading = false));
  });

  /** The agents and subscriptions present in the answer, for the filters. */
  const agents = $derived([...new Set((result?.cells ?? []).map((cell) => cell.agent))].sort());
  const profiles = $derived(
    [...new Set((result?.cells ?? []).flatMap((cell) => (cell.profile ? [cell.profile] : [])))].sort(),
  );

  const numbers = $derived(new Intl.NumberFormat($locale));

  function count(value: number): string {
    return numbers.format(value);
  }

  /** `1h 12m` / `4m 05s` / `12s`, from whole seconds. */
  function duration(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ${String(seconds % 60).padStart(2, "0")}s`;
    return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
  }

  function reasonText(kind: UnreportedKind): string {
    return $t(("usage.notReported." + kind) as MessageKey);
  }

  /** The one-line reason a cell has no measured tokens. */
  function absenceReason(cell: Cell): string {
    const reasons = cell.coverage.reasons ?? [];
    if (reasons.length === 0) return $t("usage.notReported");
    return reasons.map((reason) => reasonText(reason.kind)).join(" · ");
  }
</script>

<div class="usage">
  <header class="controls">
    <div class="group" role="group" aria-label={$t("usage.period")}>
      <span class="label">{$t("usage.period")}</span>
      {#each PERIODS as period (period)}
        <button
          class:sel={granularity === period}
          aria-pressed={granularity === period}
          onclick={() => (granularity = period)}
        >
          {$t(("usage.period." + period) as MessageKey)}
        </button>
      {/each}
    </div>

    <div class="group">
      <button
        class:sel={!allProjects}
        aria-pressed={!allProjects}
        onclick={() => (allProjects = false)}
        disabled={!$activeProject}
      >
        {$activeProject ? projectName($activeProject) : $t("usage.scope.project")}
      </button>
      <button
        class:sel={allProjects}
        aria-pressed={allProjects}
        onclick={() => (allProjects = true)}
      >
        {$t("usage.scope.all")} ({$projects.length})
      </button>
    </div>

    <label class="pick">
      {$t("usage.filter.agent")}
      <select bind:value={agentFilter}>
        <option value="">{$t("usage.filter.any")}</option>
        {#each agents as agent (agent)}
          <option value={agent}>{agent}</option>
        {/each}
      </select>
    </label>

    <label class="pick">
      {$t("usage.filter.profile")}
      <select bind:value={profileFilter}>
        <option value="">{$t("usage.filter.any")}</option>
        {#each profiles as profile (profile)}
          <option value={profile}>{profile}</option>
        {/each}
      </select>
    </label>

    {#if loading}
      <span class="faint">{$t("common.loading")}</span>
    {/if}
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if result && result.cells.length === 0 && !error}
    <EmptyState
      glyph="analytics"
      title={$t("usage.empty.title")}
      hint={$t("usage.empty.hint")}
    />
  {:else if result}
    {#if result.truncated}
      <p class="notice">{$t("usage.truncated")}</p>
    {/if}
    <div class="tableWrap">
      <table class="dense">
        <thead>
          <tr>
            <th scope="col">{$t("usage.col.period")}</th>
            {#if allProjects}
              <th scope="col">{$t("usage.col.project")}</th>
            {/if}
            <th scope="col">{$t("usage.col.agent")}</th>
            <th scope="col">{$t("usage.col.profile")}</th>
            <th scope="col" class="num">{$t("usage.col.sessions")}</th>
            <th scope="col" class="num">{$t("usage.col.activeTime")}</th>
            <th scope="col" class="num">{$t("usage.col.prompts")}</th>
            <th scope="col" class="num">{$t("usage.col.permissions")}</th>
            <th scope="col" class="num">{$t("usage.col.edits")}</th>
            <th scope="col" class="num">{$t("usage.col.commits")}</th>
            <th scope="col" class="num">{$t("usage.col.tokensIn")}</th>
            <th scope="col" class="num">{$t("usage.col.tokensOut")}</th>
            <th scope="col">{$t("usage.col.coverage")}</th>
          </tr>
        </thead>
        <tbody>
          {#each result.cells as cell (cell.projectKey + cell.period + cell.agent + (cell.profile ?? ""))}
            <tr>
              <td class="mono">{cell.period}</td>
              {#if allProjects}
                <td title={cell.projectRoot}>{projectName(cell.projectRoot)}</td>
              {/if}
              <td>{cell.agentId ?? cell.agent}</td>
              <td>
                {#if cell.profile}
                  <span class="pill">{cell.profile}</span>
                {:else}
                  <span class="faint">—</span>
                {/if}
              </td>
              <td class="num">{count(cell.activity.sessions)}</td>
              <td class="num">{duration(cell.activity.activeSeconds)}</td>
              <td class="num">{count(cell.activity.prompts)}</td>
              <td class="num">{count(cell.activity.permissionsRequested)}</td>
              <td class="num">{count(cell.activity.humanEdits)}</td>
              <td class="num">{count(cell.activity.commits)}</td>
              <!-- An absent counter is symbol + word with its reason, never a 0. -->
              <td class="num">
                {#if cell.tokens?.input !== undefined}
                  {count(cell.tokens.input)}
                {:else}
                  <span class="absent" title={absenceReason(cell)}>
                    <span aria-hidden="true">○</span> {$t("usage.notReported")}
                  </span>
                {/if}
              </td>
              <td class="num">
                {#if cell.tokens?.output !== undefined}
                  {count(cell.tokens.output)}
                {:else}
                  <span class="absent" title={absenceReason(cell)}>
                    <span aria-hidden="true">○</span> {$t("usage.notReported")}
                  </span>
                {/if}
              </td>
              <td class="faint">
                {$t("usage.coverage.measured", { n: String(cell.coverage.measuredSessions) })}
                {#if cell.coverage.unreportedSessions > 0}
                  · {$t("usage.coverage.unreported", {
                    n: String(cell.coverage.unreportedSessions),
                  })}
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row">{$t("usage.totals")}</th>
            {#if allProjects}<td></td>{/if}
            <td colspan="2"></td>
            <td class="num">{count(result.totals.activity.sessions)}</td>
            <td class="num">{duration(result.totals.activity.activeSeconds)}</td>
            <td class="num">{count(result.totals.activity.prompts)}</td>
            <td class="num">{count(result.totals.activity.permissionsRequested)}</td>
            <td class="num">{count(result.totals.activity.humanEdits)}</td>
            <td class="num">{count(result.totals.activity.commits)}</td>
            <td class="num">
              {#if result.totals.tokens?.input !== undefined}
                {count(result.totals.tokens.input)}
              {:else}
                <span class="absent"><span aria-hidden="true">○</span> {$t("usage.notReported")}</span>
              {/if}
            </td>
            <td class="num">
              {#if result.totals.tokens?.output !== undefined}
                {count(result.totals.tokens.output)}
              {:else}
                <span class="absent"><span aria-hidden="true">○</span> {$t("usage.notReported")}</span>
              {/if}
            </td>
            <td class="faint">
              {$t("usage.coverage.measured", {
                n: String(result.totals.coverage.measuredSessions),
              })}
              {#if result.totals.coverage.unreportedSessions > 0}
                · {$t("usage.coverage.unreported", {
                  n: String(result.totals.coverage.unreportedSessions),
                })}
              {/if}
            </td>
          </tr>
        </tfoot>
      </table>
    </div>

    {#if (result.unattributed ?? []).length > 0}
      <section class="panel" aria-label={$t("usage.unattributed")}>
        <h3>{$t("usage.unattributed")}</h3>
        <p class="faint">{$t("usage.unattributed.hint")}</p>
        <table class="dense">
          <tbody>
            {#each result.unattributed ?? [] as row (row.projectKey + row.period)}
              <tr>
                <td class="mono">{row.period}</td>
                <td title={row.projectRoot}>{projectName(row.projectRoot)}</td>
                <td class="num">{count(row.humanEdits)}</td>
                <td>{$t("usage.col.edits")}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  {/if}

  <!-- The disclosure sits with the numbers: no click reveals it (design D6). -->
  {#if result}
    <section class="disclosure" aria-label={$t("usage.disclosure.title")}>
      <h3><Icon name="analytics" size={12} /> {$t("usage.disclosure.title")}</h3>
      <ul>
        {#each result.disclosure as key (key)}
          <li>{$t(("usage.disclosure." + key) as MessageKey)}</li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .usage {
    display: flex;
    flex-direction: column;
    gap: var(--panel-pad);
    padding: var(--panel-pad);
    min-height: 0;
    overflow-y: auto;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }
  .group {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
  }
  .group .label,
  .pick {
    font-size: var(--fs-caption);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tableWrap {
    overflow-x: auto;
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
  }
  /* Tabular numerals: figures keep their columns when they refresh. */
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-feature-settings: "tnum" 1;
  }
  .absent {
    white-space: nowrap;
    color: var(--text-faint);
  }
  tfoot th,
  tfoot td {
    border-top: 1px solid var(--border);
    font-weight: 500;
  }
  .panel,
  .disclosure {
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
    padding: var(--panel-pad);
  }
  .panel h3,
  .disclosure h3 {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-dense);
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .disclosure ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .notice {
    margin: 0;
    font-size: var(--fs-caption);
    color: var(--tint-warn);
  }
  .error {
    margin: 0;
    font-size: var(--fs-dense);
    color: var(--tint-danger);
  }
  p.faint {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-caption);
  }
</style>
