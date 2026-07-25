<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import type { IconName } from "../icons";
  import type { ViewId } from "../registry";
  import { activeProject, pending, sessions } from "../stores";
  import Icon from "./Icon.svelte";

  let {
    view,
    onNavigate,
    onPickProject,
  }: {
    view: ViewId;
    onNavigate: (view: ViewId) => void;
    onPickProject: () => void;
  } = $props();

  const ITEMS: { id: ViewId; icon: IconName; key?: string }[] = [
    { id: "sessions", icon: "sessions", key: "1" },
    { id: "project", icon: "project", key: "2" },
    { id: "permissions", icon: "permissions", key: "3" },
    { id: "fleet", icon: "fleet", key: "4" },
    { id: "editor", icon: "editor" },
  ];

  const liveSessions = $derived(
    $sessions.filter(
      (session) =>
        session.state === "active" ||
        session.state === "starting" ||
        session.state === "waiting_permission",
    ).length,
  );

  const projectName = $derived.by(() => {
    const root = $activeProject;
    if (!root) return null;
    const parts = root.replaceAll("\\", "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? root;
  });

  function counterFor(id: ViewId): { value: number; warn: boolean } | null {
    if (id === "sessions" && liveSessions > 0) {
      return { value: liveSessions, warn: false };
    }
    if (id === "permissions" && $pending.length > 0) {
      return { value: $pending.length, warn: true };
    }
    return null;
  }
</script>

<aside aria-label={$t("nav.viewLabel")}>
  <button class="project ghost" onclick={onPickProject}>
    <span class="mark" aria-hidden="true"></span>
    <span class="name">{projectName ?? $t("nav.noProject")}</span>
    <Icon name="chevronDown" size={14} />
  </button>

  <nav>
    {#each ITEMS as item (item.id)}
      {@const counter = counterFor(item.id)}
      <button
        class="item ghost"
        class:current={view === item.id}
        aria-current={view === item.id ? "page" : undefined}
        onclick={() => onNavigate(item.id)}
      >
        <Icon name={item.icon} size={16} />
        <span class="label">{$t(("nav." + item.id) as never)}</span>
        {#if counter}
          <span class="pill counter" class:warn={counter.warn}>{counter.value}</span>
        {:else if item.key}
          <kbd>{item.key}</kbd>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="bottom">
    <button
      class="item ghost"
      class:current={view === "settings"}
      aria-current={view === "settings" ? "page" : undefined}
      onclick={() => onNavigate("settings")}
    >
      <Icon name="settings" size={16} />
      <span class="label">{$t("nav.settings")}</span>
    </button>
  </div>
</aside>

<style>
  aside {
    width: 216px;
    flex: none;
    background: var(--panel);
    border-right: 1px solid var(--hair);
    display: flex;
    flex-direction: column;
    padding: var(--sp-3) var(--sp-2) var(--sp-2);
    gap: var(--sp-3);
    min-height: 0;
  }
  .project {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    background: var(--surface-2);
    border-color: var(--border);
    border-radius: var(--radius-panel);
    padding: var(--sp-2);
    font-weight: 500;
    text-align: left;
  }
  .project .mark {
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: var(--radius-control);
    background: linear-gradient(135deg, var(--mel-aegean), var(--mel-wind));
  }
  .project .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-dense);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: 6px var(--sp-2);
    border-radius: var(--radius-control);
    color: var(--text-muted);
    text-align: left;
    font-size: var(--fs-dense);
  }
  .item.current {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 500;
  }
  .label {
    flex: 1;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-caption);
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
  }
  .bottom {
    margin-top: auto;
    border-top: 1px solid var(--hair);
    padding-top: var(--sp-2);
    display: flex;
    flex-direction: column;
  }
</style>
