<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { fleet, refreshFleet } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

  $effect(() => {
    void refreshFleet().catch(() => {});
  });
</script>

{#if $fleet.length === 0}
  <EmptyState
    glyph="⛵"
    title={$t("fleet.empty.title")}
    hint={$t("fleet.empty.hint")}
  />
{:else}
  <table>
    <thead>
      <tr>
        <th scope="col">{$t("fleet.col.agent")}</th>
        <th scope="col">{$t("fleet.col.source")}</th>
        <th scope="col">{$t("fleet.col.level")}</th>
        <th scope="col">{$t("fleet.col.detected")}</th>
        <th scope="col">{$t("fleet.col.configured")}</th>
      </tr>
    </thead>
    <tbody>
      {#each $fleet as agent (agent.id)}
        <tr>
          <td>
            {agent.displayName}
            {#if agent.underlyingAgent}
              <span class="muted">
                {$t("fleet.underlying", { agent: agent.underlyingAgent })}
              </span>
            {/if}
          </td>
          <td>{$t(("fleet.source." + agent.source) as never)}</td>
          <td>
            {agent.verifiedLevel ?? agent.integrationLevel}
            <span class="muted">
              ({agent.verifiedLevel
                ? $t("fleet.level.verified")
                : $t("fleet.level.declared")})
            </span>
          </td>
          <td>
            {#if agent.detected}
              <span class="ok">▸ {$t("common.yes")}</span>
            {:else}
              <span class="muted">■ {$t("common.no")}</span>
            {/if}
          </td>
          <td>
            {#if agent.configured}
              <span class="ok">▸ {$t("common.yes")}</span>
            {:else}
              <span class="muted">—</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  table {
    width: 100%;
    border-collapse: collapse;
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
  .muted {
    color: var(--text-muted);
    font-size: 0.8125rem;
  }
  .ok {
    color: var(--ok);
  }
</style>
