<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import type { IconName } from "../icons";
  import type { ViewId } from "../registry";
  import { activeProject, allSessions, pending, projects, sessions, switchProject } from "../stores";
  import { agentLabelOf, groupSessions, projectName as leafOf } from "../tree";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  let {
    view,
    onNavigate,
    onPickProject,
    onOpenSession,
  }: {
    view: ViewId;
    onNavigate: (view: ViewId) => void;
    onPickProject: () => void;
    onOpenSession: (sessionId: string) => void;
  } = $props();

  /** Collapsed project nodes, by root. The tree opens expanded. */
  let collapsed = $state(new Set<string>());

  // Project -> Sessions, aggregated in the client from the global session list
  // joined with the project registry (design D7).
  const tree = $derived(groupSessions($projects, $allSessions));

  function toggle(root: string) {
    const next = new Set(collapsed);
    if (!next.delete(root)) next.add(root);
    collapsed = next;
  }

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

  const projectName = $derived($activeProject ? leafOf($activeProject) : null);

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

  <div class="tree" role="tree" aria-label={$t("nav.tree")}>
    {#each tree as group (group.root)}
      {@const open = !collapsed.has(group.root)}
      {@const current = group.root === $activeProject}
      <div class="group" role="treeitem" aria-expanded={open} aria-selected={current}>
        <div class="groupRow" class:current>
          <button
            class="twisty ghost"
            aria-label={$t(open ? "nav.tree.collapse" : "nav.tree.expand", {
              project: group.name,
            })}
            onclick={() => toggle(group.root)}
          >
            <Icon name={open ? "chevronDown" : "chevronRight"} size={12} />
          </button>
          <button
            class="groupName ghost"
            title={group.root}
            onclick={() => switchProject(group.root)}
          >
            <span class="name">{group.name}</span>
            {#if !group.exists}
              <span class="pill danger">{$t("projects.absent")}</span>
            {:else if group.live > 0}
              <span class="pill ok">{group.live}</span>
            {:else}
              <span class="count">{group.sessions.length}</span>
            {/if}
          </button>
        </div>

        {#if open}
          <ul role="group">
            {#each group.sessions.slice(0, 8) as session (session.sessionId)}
              <li>
                <button class="leaf ghost" onclick={() => onOpenSession(session.sessionId)}>
                  <Avatar id={agentLabelOf(session)} size={16} />
                  <span class="agent">{agentLabelOf(session)}</span>
                  {#if session.profile}
                    <span class="pill sub">{session.profile}</span>
                  {/if}
                  <span class="dot" data-state={session.state} aria-hidden="true"></span>
                </button>
              </li>
            {/each}
            {#if group.sessions.length === 0}
              <li class="hint">{$t("nav.tree.empty")}</li>
            {:else if group.sessions.length > 8}
              <li class="hint">
                <button class="ghost more" onclick={() => { switchProject(group.root); onNavigate("sessions"); }}>
                  {$t("sessions.showAll", { n: String(group.sessions.length) })}
                </button>
              </li>
            {/if}
          </ul>
        {/if}
      </div>
    {/each}
  </div>

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
  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border-top: 1px solid var(--hair);
    padding-top: var(--sp-2);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .groupRow {
    display: flex;
    align-items: center;
    height: var(--row-h);
    border-radius: var(--radius-control);
  }
  .groupRow.current {
    background: var(--surface-2);
  }
  .twisty {
    flex: none;
    padding: 0 2px;
    color: var(--text-faint);
  }
  .groupName {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 0 4px;
    font-size: var(--fs-dense);
    font-weight: 500;
    color: var(--text);
  }
  .groupName .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  ul[role="group"] {
    list-style: none;
    margin: 0;
    padding: 0 0 0 18px;
  }
  .leaf {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 28px;
    padding: 0 4px;
    border-radius: var(--radius-control);
    text-align: left;
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .leaf .agent {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pill.sub {
    flex: none;
    max-width: 74px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    margin-left: auto;
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    background: var(--text-faint);
  }
  .dot[data-state="active"],
  .dot[data-state="starting"] {
    background: var(--tint-ok);
  }
  .dot[data-state="waiting_permission"] {
    background: var(--tint-warn);
  }
  .dot[data-state="interrupted"] {
    background: var(--tint-danger);
  }
  .hint {
    font-size: var(--fs-caption);
    color: var(--text-faint);
    padding: 2px 4px;
  }
  .more {
    font: inherit;
    color: var(--accent);
    padding: 0;
  }
  .bottom {
    margin-top: auto;
    border-top: 1px solid var(--hair);
    padding-top: var(--sp-2);
    display: flex;
    flex-direction: column;
  }
</style>
