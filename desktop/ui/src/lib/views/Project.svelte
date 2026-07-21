<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { get } from "svelte/store";
  import { request } from "../daemon";
  import { t } from "../i18n";
  import { projectRoot, type ChangeInfo, type SpecInfo } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

  let changes: ChangeInfo[] = $state([]);
  let specs: SpecInfo[] = $state([]);
  let isProject = $state(true);
  let loading = $state(true);
  let validating = $state(false);
  let validateSummary: string | null = $state(null);

  async function load() {
    const root = get(projectRoot);
    if (!root) {
      isProject = false;
      loading = false;
      return;
    }
    try {
      const [changesResult, specsResult] = await Promise.all([
        request<{ changes: ChangeInfo[] }>("change/list", {
          projectRoot: root,
        }),
        request<{ specs: SpecInfo[] }>("spec/list", { projectRoot: root }),
      ]);
      changes = changesResult.changes;
      specs = specsResult.specs;
      isProject = true;
    } catch {
      // The daemon refuses when the directory is not a `.meltemi/` project;
      // the view shows the initialization path, everything else stays usable.
      isProject = false;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void load();
  });

  async function validate() {
    const root = get(projectRoot);
    if (!root || validating) return;
    validating = true;
    validateSummary = null;
    try {
      const result = await request<Record<string, unknown>>("sdd/validate", {
        projectRoot: root,
      });
      const findings = Array.isArray(result.findings)
        ? (result.findings as unknown[]).length
        : 0;
      validateSummary =
        findings === 0
          ? $t("project.validate.clean")
          : $t("project.validate.findings", { n: findings });
    } catch (raw) {
      const e = raw as { message?: string };
      validateSummary = `${$t("common.error")}: ${e?.message ?? String(raw)}`;
    } finally {
      validating = false;
    }
  }
</script>

{#if loading}
  <p class="muted">{$t("common.loading")}</p>
{:else if !isProject}
  <EmptyState
    glyph="⌂"
    title={$t("project.empty.title")}
    hint={$t("project.empty.hint")}
  />
{:else}
  <div class="panels">
    <section aria-labelledby="changes-title">
      <header>
        <h2 id="changes-title">{$t("project.changes")}</h2>
        <div class="validate">
          <button disabled={validating} onclick={() => void validate()}>
            {validating ? $t("common.loading") : $t("project.validate")}
          </button>
          {#if validateSummary}
            <span aria-live="polite">{validateSummary}</span>
          {/if}
        </div>
      </header>
      <table>
        <thead>
          <tr>
            <th scope="col">{$t("project.col.change")}</th>
            <th scope="col">{$t("project.col.state")}</th>
            <th scope="col">{$t("project.col.tasks")}</th>
            <th scope="col">{$t("project.col.review")}</th>
            <th scope="col">{$t("project.col.verify")}</th>
          </tr>
        </thead>
        <tbody>
          {#each changes as change (change.name + String(change.archived))}
            <tr>
              <td><code>{change.name}</code></td>
              <td class:muted={change.archived}>
                {change.archived
                  ? `■ ${$t("project.archived")}`
                  : `▸ ${$t("project.active")}`}
              </td>
              <td>{change.tasksDone}/{change.tasksTotal}</td>
              <td>{change.reviewDecided}/{change.reviewTotal}</td>
              <td>{change.verified}/{change.verifyTotal}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <section aria-labelledby="specs-title">
      <h2 id="specs-title">{$t("project.specs")}</h2>
      <table>
        <thead>
          <tr>
            <th scope="col">{$t("project.col.capability")}</th>
            <th scope="col">{$t("project.col.requirements")}</th>
            <th scope="col">{$t("project.col.scenarios")}</th>
          </tr>
        </thead>
        <tbody>
          {#each specs as spec (spec.capability)}
            <tr>
              <td><code>{spec.capability}</code></td>
              <td>{spec.requirements}</td>
              <td>{spec.scenarios}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  </div>
{/if}

<style>
  .panels {
    display: grid;
    gap: var(--sp-6);
    align-content: start;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }
  h2 {
    margin: 0 0 var(--sp-2);
    font-size: 1rem;
  }
  .validate {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
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
  .muted {
    color: var(--text-muted);
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
