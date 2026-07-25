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

  /** The state glyph of the design system's vocabulary (never colour alone). */
  function stateGlyph(state: string): string {
    switch (state) {
      case "active":
        return "▶";
      case "starting":
        return "◔";
      case "waiting_permission":
        return "‖";
      case "interrupted":
        return "▲";
      default:
        return "■";
    }
  }

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
    { id: "analytics", icon: "analytics", key: "5" },
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
  <div class="identity">
    <svg
        class="mark"
        viewBox="0 0 256 256"
        role="img"
        aria-hidden="true"
        focusable="false"
      >
        <!-- The mark of brand/meltemi-mark-gradient.svg, inlined so the chrome
             needs no asset fetch and the gradient can fall back to the system
             text color under forced colors. -->
        <defs>
          <linearGradient
            id="melMark"
            x1="38"
            y1="222"
            x2="219"
            y2="54"
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0" stop-color="var(--mel-aegean)" />
            <stop offset="1" stop-color="var(--mel-wind)" />
          </linearGradient>
        </defs>
        <path
          class="stroke"
          d="M43 178C43 125 57 100 78 100C99 100 109 127 109 176C109 105 132 67 158 67C185 67 199 97 196 132C194 154 185 169 172 177C189 176 205 166 217 152C224 144 230 133 234 122"
          fill="none"
          stroke="url(#melMark)"
          stroke-width="25"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          class="hull"
          d="M45 199C87 211 145 215 202 189C195 204 177 216 151 220C106 226 67 215 45 199Z"
          fill="url(#melMark)"
        />
      </svg>
      <span class="brand-wordmark">{$t("app.title")}</span>
  </div>

  <button class="project ghost" onclick={onPickProject}>
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
                  <span class="leafState" data-state={session.state}>
                    <span aria-hidden="true">{stateGlyph(session.state)}</span>
                    <span class="sr">{$t(("state." + session.state) as never)}</span>
                  </span>
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
  .identity {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 var(--sp-1);
  }
  .identity .mark {
    width: 24px;
    height: 24px;
    flex: none;
  }
  .brand-wordmark {
    font-size: var(--fs-section);
    font-weight: 600;
    letter-spacing: 0.01em;
    background: linear-gradient(135deg, var(--mel-aegean), var(--mel-wind));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
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
  /* The leaf's state is a glyph with an accessible word — the design system's
     rule is that colour is never the only carrier. */
  .leafState {
    margin-left: auto;
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  .leafState[data-state="active"],
  .leafState[data-state="starting"] {
    color: var(--ok);
  }
  .leafState[data-state="waiting_permission"] {
    color: var(--warn);
  }
  .leafState[data-state="interrupted"] {
    color: var(--danger);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
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
