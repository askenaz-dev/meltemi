<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { isMeltemiProject } from "../editor/files";
  import { get } from "svelte/store";
  import { request } from "../daemon";
  import { t } from "../i18n";
  import { activeProject, changes, refreshChanges, type SpecInfo } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

  let {
    onOpenEditor,
    onOpenReview,
    onPropose,
  }: {
    onOpenEditor: () => void;
    onOpenReview: () => void;
    /** Opens the session launcher on its propose mode: a tool, not the centre. */
    onPropose: () => void;
  } = $props();

  let specs: SpecInfo[] = $state([]);
  let isProject = $state(true);
  let loading = $state(true);
  let validating = $state(false);
  let validateSummary: string | null = $state(null);

  async function load() {
    const root = get(activeProject);
    if (!root) {
      isProject = false;
      loading = false;
      return;
    }
    try {
      // The daemon answers a directory without `.meltemi/` with empty lists, so
      // emptiness is NOT evidence of absence: ask for the marker directly. Only
      // this view degrades; every other one stays usable (shell design D2).
      const initialized = await isMeltemiProject(root);
      if (!initialized) {
        isProject = false;
        changes.set([]);
        specs = [];
        return;
      }
      // The changes come from the shared store, which the status bar reads
      // too: one source, two readers (barra-de-estado-agentica design D4).
      const [, specsResult] = await Promise.all([
        refreshChanges(),
        request<{ specs: SpecInfo[] }>("spec/list", { projectRoot: root }),
      ]);
      specs = specsResult.specs;
      isProject = true;
    } catch {
      // A refusal from the daemon lands here too: the view shows the
      // initialization path rather than a bare error.
      isProject = false;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void load();
  });

  async function validate() {
    const root = get(activeProject);
    if (!root || validating) return;
    validating = true;
    validateSummary = null;
    try {
      const result = await request<Record<string, unknown>>("sdd/validate", {
        projectRoot: root,
      });
      // `sdd/validate` answers with `diagnostics`; reading `findings` made every
      // change look clean (contract: proto/schemas/v1/validate.schema.json).
      const findings = Array.isArray(result.diagnostics)
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
    glyph="project"
    title={$t("project.empty.title")}
    hint={$t("project.empty.hint")}
  >
    <button class="primary" onclick={onPropose}>{$t("project.propose")}</button>
  </EmptyState>
{:else}
  <div class="panels">
    <section aria-labelledby="changes-title">
      <header>
        <h2 id="changes-title">{$t("project.changes")}</h2>
        <div class="validate">
          <button onclick={onPropose}>{$t("project.propose")}</button>
          <button onclick={onOpenEditor}>{$t("project.openEditor")}</button>
          <button onclick={onOpenReview}>{$t("project.openReview")}</button>
          <button disabled={validating} onclick={() => void validate()}>
            {validating ? $t("common.loading") : $t("project.validate")}
          </button>
          {#if validateSummary}
            <span aria-live="polite">{validateSummary}</span>
          {/if}
        </div>
      </header>
      <table class="dense">
        <thead>
          <tr>
            <th scope="col">{$t("project.col.change")}</th>
            <th scope="col">{$t("project.col.state")}</th>
            <th scope="col">{$t("project.col.gate")}</th>
            <th scope="col">{$t("project.col.tasks")}</th>
            <th scope="col">{$t("project.col.review")}</th>
            <th scope="col">{$t("project.col.verify")}</th>
          </tr>
        </thead>
        <tbody>
          {#each $changes as change (change.name + String(change.archived))}
            <tr>
              <td><code>{change.name}</code></td>
              <td class:muted={change.archived}>
                {change.archived
                  ? `■ ${$t("project.archived")}`
                  : `▸ ${$t("project.active")}`}
              </td>
              <td>
                {#if change.gatePending}
                  <span class="gate">
                    {change.gateArtifact
                      ? $t("project.gateOn", { artifact: change.gateArtifact })
                      : $t("project.gateWaiting")}
                  </span>
                {:else}
                  <span class="muted">—</span>
                {/if}
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
      <table class="dense">
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
    padding: var(--panel-pad);
    overflow: auto;
    height: 100%;
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
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .validate {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  .muted {
    color: var(--text-muted);
  }
  /* A pending gate is the one row state that asks something of the human:
     it carries weight and the warning hue, never color alone. */
  .gate {
    color: var(--warn);
    font-weight: 600;
  }

</style>
