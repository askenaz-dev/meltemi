<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The project switcher (multiproyecto-suscripciones design D6): the known
  projects from `project/list`, most recently used first. A root that vanished
  from disk stays listed with its remedy — the history is never dropped just
  because a folder moved.
-->
<script lang="ts">
  import { locale, t } from "../i18n";
  import { relativeTime } from "../time";
  import { activeProject, projects, refreshProjects, switchProject } from "../stores";
  import { projectName } from "../tree";
  import Icon from "./Icon.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let panel: HTMLDivElement | null = $state(null);

  $effect(() => {
    void refreshProjects().catch(() => {});
    panel?.querySelector<HTMLButtonElement>("button")?.focus();
  });

  function pick(root: string, exists: boolean) {
    // An absent root is still switchable: the session history is readable and
    // the remedy is the user's to apply.
    if (!exists) return;
    switchProject(root);
    onClose();
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
  }}
/>

<div class="scrim" role="presentation" onclick={onClose}></div>

<div class="panel" bind:this={panel} role="dialog" aria-label={$t("projects.switch")}>
  <header>
    <span>{$t("projects.title")}</span>
    <button class="ghost" onclick={onClose} aria-label={$t("common.close")}>
      <Icon name="close" size={12} />
    </button>
  </header>

  {#if $projects.length === 0}
    <p class="empty">{$t("projects.empty")}</p>
  {:else}
    <ul>
      {#each $projects as project (project.projectKey)}
        {@const current = project.root === $activeProject}
        <li>
          <button
            class="row ghost"
            class:current
            class:absent={!project.exists}
            disabled={!project.exists}
            aria-current={current ? "true" : undefined}
            onclick={() => pick(project.root, project.exists)}
          >
            <span class="mark" aria-hidden="true"></span>
            <span class="text">
              <span class="name">
                {projectName(project.root)}
                {#if current}<span class="pill">{$t("projects.current")}</span>{/if}
                {#if !project.exists}
                  <span class="pill danger">{$t("projects.absent")}</span>
                {/if}
              </span>
              <span class="meta mono">{project.root}</span>
              <span class="meta">
                {$t("projects.sessions", { n: String(project.sessionsTotal) })}
                {#if project.resumableSessions > 0}
                  · {$t("projects.resumable", { n: String(project.resumableSessions) })}
                {/if}
                · {$t("projects.lastSeen")} {relativeTime(project.lastSeenAt, $locale)}
              </span>
              {#if !project.exists}
                <span class="remedy">{$t("projects.absent.remedy")}</span>
              {/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 24%);
    z-index: 40;
  }
  .panel {
    position: fixed;
    top: 52px;
    left: var(--sp-2);
    width: 380px;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    z-index: 41;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--cell-pad) var(--panel-pad);
    border-bottom: 1px solid var(--hair);
    font-size: var(--fs-caption);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--cell-pad);
    width: 100%;
    text-align: left;
    padding: var(--cell-pad);
    border-radius: var(--radius-control);
    color: var(--text);
  }
  .row.current {
    background: var(--surface-2);
  }
  .row.absent {
    opacity: 0.75;
    cursor: not-allowed;
  }
  .mark {
    width: 20px;
    height: 20px;
    flex: none;
    margin-top: 2px;
    border-radius: var(--radius-control);
    background: linear-gradient(135deg, var(--mel-aegean), var(--mel-wind));
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 500;
    font-size: var(--fs-dense);
  }
  .meta {
    font-size: var(--fs-caption);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remedy {
    font-size: var(--fs-caption);
    color: var(--tint-danger);
    white-space: normal;
  }
  .empty {
    margin: 0;
    padding: var(--panel-pad);
    font-size: var(--fs-dense);
    color: var(--text-muted);
  }
</style>
