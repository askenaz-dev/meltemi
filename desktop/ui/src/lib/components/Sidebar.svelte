<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import type { IconName } from "../icons";
  import type { ViewId } from "../registry";
  import {
    activeProject,
    allSessions,
    forgetProject,
    pending,
    pickAndRegisterProject,
    projects,
    pushNotice,
    sessions,
    switchProject,
  } from "../stores";
  import { agentLabelOf, groupSessions, projectName as leafOf } from "../tree";
  import Avatar from "./Avatar.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";
  import { setNavCollapsed, uiState } from "../ui-state";
  import { MIN_NAV_PX, MIN_TREE_PX, STEP_PX, clampNavHeight, stepNavHeight } from "../nav-split";

  let {
    view,
    onNavigate,
    onPickProject,
    onOpenSession,
    onNewSessionIn,
  }: {
    view: ViewId;
    onNavigate: (view: ViewId) => void;
    onPickProject: () => void;
    onOpenSession: (sessionId: string) => void;
    /** Start work in this project: to the composer, with it already chosen. */
    onNewSessionIn: (root: string) => void;
  } = $props();

  /** Collapsed project nodes, by root. The tree opens expanded. */
  let collapsed = $state(new Set<string>());
  /** Whether the whole bar is folded to its rail, remembered across runs. */
  let folded = $state($uiState.navCollapsed);

  /** Folds or unfolds the bar and remembers the choice beside the theme. */
  function toggleFold() {
    folded = !folded;
    setNavCollapsed(folded);
  }

  /**
   * How the bar's height is split. `null` means "as the browser lays it out",
   * which is what a profile that never touched the divider gets.
   */
  let navSplit: number | null = $state(null);
  let navEl: HTMLElement | undefined = $state();
  let treeEl: HTMLElement | undefined = $state();
  /** Plain, not reactive: only the pointer handlers read it, mid-drag. */
  let dragFrom: { y: number; nav: number; available: number } | null = null;

  /** The height the two zones have to share right now. */
  function availableHeight(): number {
    return (navEl?.offsetHeight ?? 0) + (treeEl?.offsetHeight ?? 0);
  }

  function startDrag(event: PointerEvent) {
    if (!navEl) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragFrom = { y: event.clientY, nav: navEl.offsetHeight, available: availableHeight() };
    event.preventDefault();
  }

  function onDragMove(event: PointerEvent) {
    if (!dragFrom) return;
    navSplit = clampNavHeight(dragFrom.nav + (event.clientY - dragFrom.y), dragFrom.available);
  }

  function endDrag() {
    if (!dragFrom) return;
    dragFrom = null;
  }

  /**
   * The divider is a control, so it answers to the keyboard: a step each way,
   * and the two ends by name. No global key is minted for it.
   */
  function onSplitKeys(event: KeyboardEvent) {
    const available = availableHeight();
    const current = navSplit ?? navEl?.offsetHeight ?? MIN_NAV_PX;
    let next: number | null = null;
    if (event.key === "ArrowUp") next = stepNavHeight(current, -STEP_PX, available);
    else if (event.key === "ArrowDown") next = stepNavHeight(current, STEP_PX, available);
    else if (event.key === "Home") next = clampNavHeight(MIN_NAV_PX, available);
    else if (event.key === "End") next = clampNavHeight(available, available);
    if (next === null) return;
    event.preventDefault();
    event.stopPropagation();
    navSplit = next;
  }
  /** The node whose forget is awaiting confirmation. */
  let forgetTarget: { root: string; name: string } | null = $state(null);

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

  /**
   * "Open folder…": the client's dialog, then the contract's `project/register`
   * BEFORE anything else happens. Opening a folder from the nav also means
   * going there, so the scope follows — unlike the composer's chip, which only
   * aims a launch.
   */
  async function openFolder() {
    try {
      const root = await pickAndRegisterProject();
      if (!root) return; // dismissed: nothing chosen, nothing said
      switchProject(root);
      pushNotice($t("projects.opened", { project: leafOf(root) }), "info");
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      const detail = e?.detail ?? e?.message ?? String(raw);
      pushNotice(
        e?.remedy ? `${detail} — ${e.remedy}` : `${$t("common.error")}: ${detail}`,
        "danger",
      );
    }
  }

  /**
   * Forgetting is a listing decision, so the confirmation says exactly that and
   * nothing stronger: no file, no session and no log is touched, and the
   * project returns the moment it is used again.
   */
  async function forget(root: string, name: string) {
    forgetTarget = null;
    try {
      await forgetProject(root);
      pushNotice($t("projects.forgotten", { project: name }), "info");
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null };
      pushNotice(`${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`, "danger");
    }
  }

  function toggle(root: string) {
    const next = new Set(collapsed);
    if (!next.delete(root)) next.add(root);
    collapsed = next;
  }

  const ITEMS: { id: ViewId; icon: IconName; key?: string }[] = [
    // The composer sits first and unnumbered: it is where work starts, and the
    // numbered keys belong to the five first-level views they always did.
    { id: "home", icon: "plus" },
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

<aside class:folded aria-label={$t("nav.viewLabel")}>
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
      <!-- Folding is a visible control, never a hover reveal: a control that
           appears under the pointer is one the keyboard cannot reach. -->
      <button
        class="fold ghost"
        onclick={toggleFold}
        aria-expanded={!folded}
        title={$t(folded ? "nav.fold.expand" : "nav.fold.collapse")}
        aria-label={$t(folded ? "nav.fold.expand" : "nav.fold.collapse")}
      >
        <Icon name={folded ? "chevronRight" : "chevronLeft"} size={14} />
      </button>
  </div>

  <!-- Folded, this is a caret alone: it says what it opens, and its title says
       which project it is standing for. -->
  <button
    class="project ghost"
    onclick={onPickProject}
    aria-label={$t("projects.switch")}
    title={projectName ?? $t("nav.noProject")}
  >
    <span class="name">{projectName ?? $t("nav.noProject")}</span>
    <Icon name="chevronDown" size={14} />
  </button>

  <nav
    id="nav-entries"
    bind:this={navEl}
    style:height={folded || navSplit === null ? null : navSplit + "px"}
  >
    {#each ITEMS as item (item.id)}
      {@const counter = counterFor(item.id)}
      <button
        class="item ghost"
        class:current={view === item.id}
        aria-current={view === item.id ? "page" : undefined}
        onclick={() => onNavigate(item.id)}
        aria-label={$t(("nav." + item.id) as never)}
        title={$t(("nav." + item.id) as never)}
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

  <!-- The line that divides the two zones is the control that moves them. It
       carries the hairline the projects header used to draw, because a rule
       that now divides should read as a divider and not as a heading. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="split"
    role="separator"
    tabindex="0"
    aria-orientation="horizontal"
    aria-controls="nav-entries"
    aria-label={$t("nav.split.label")}
    aria-valuenow={Math.round(navSplit ?? navEl?.offsetHeight ?? 0)}
    aria-valuemin={MIN_NAV_PX}
    aria-valuemax={Math.max(MIN_NAV_PX, availableHeight() - MIN_TREE_PX)}
    onpointerdown={startDrag}
    onpointermove={onDragMove}
    onpointerup={endDrag}
    onpointercancel={endDrag}
    onkeydown={onSplitKeys}
  ></div>

  <!-- The projects section is permanent chrome, not something a modal reveals:
       it is where the shell says what it knows about, and it stays put. -->
  <div class="section">
    <span class="sectionTitle">{$t("projects.title")}</span>
    <button class="ghost open" onclick={() => void openFolder()}>
      <Icon name="plus" size={12} />
      {$t("projects.open")}
    </button>
  </div>

  <div class="tree" bind:this={treeEl} role="tree" aria-label={$t("nav.tree")}>
    {#if tree.length === 0}
      <p class="hint empty">{$t("projects.empty")}</p>
    {/if}
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
          <!-- Always rendered, never revealed on hover: a control that appears
               under the pointer is a control the keyboard cannot reach. -->
          <button
            class="quick ghost"
            aria-label={$t("nav.tree.newSession", { project: group.name })}
            title={$t("nav.tree.newSession", { project: group.name })}
            onclick={() => onNewSessionIn(group.root)}
          >
            <Icon name="plus" size={12} />
          </button>
          <button
            class="quick ghost"
            aria-label={$t("projects.forget.action", { project: group.name })}
            title={$t("projects.forget.action", { project: group.name })}
            onclick={() => (forgetTarget = group)}
          >
            <Icon name="close" size={12} />
          </button>
        </div>

        {#if !group.exists}
          <!-- A root that vanished keeps its node: the registry is the daemon's
               and a surface that hides rows decides what it holds. -->
          <p class="hint remedy">{$t("projects.absent.remedy")}</p>
        {/if}

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
      aria-label={$t("nav.settings")}
      title={$t("nav.settings")}
    >
      <Icon name="settings" size={16} />
      <span class="label">{$t("nav.settings")}</span>
    </button>
  </div>
</aside>

{#if forgetTarget}
  <ConfirmDialog
    title={$t("projects.forget.title", { project: forgetTarget.name })}
    message={$t("projects.forget.warning")}
    confirmLabel={$t("projects.forget")}
    onConfirm={() => forgetTarget && void forget(forgetTarget.root, forgetTarget.name)}
    onCancel={() => (forgetTarget = null)}
  />
{/if}

<style>
  aside.folded {
    width: 52px;
    padding-inline: var(--sp-1);
  }
  /* Folded: the words go, the reach stays — every entry keeps its icon, its
     focus ring and its accessible name, and the permission counter keeps its
     place because it is the one signal the shell must never hide. */
  aside.folded .brand-wordmark,
  aside.folded .label,
  aside.folded kbd,
  aside.folded .project .name,
  aside.folded .section {
    display: none;
  }
  /* The project tree is a panel of content, not a navigation entry: at 52px it
     would be text broken into single-letter columns. It goes as a whole, and
     nothing goes with it — its projects and sessions stay reachable through the
     Sessions entry and the project switcher, both of which remain on the rail. */
  aside.folded .tree,
  aside.folded .split {
    display: none;
  }
  aside.folded .item,
  aside.folded .project {
    justify-content: center;
  }
  aside.folded .identity {
    flex-direction: column;
    gap: var(--sp-1);
    align-items: center;
    padding-inline: 0;
  }
  aside.folded .project {
    padding-inline: 0;
  }
  /* The auto margin that pushes the control to the far end of the header row
     would push it off-centre once the header becomes a column. */
  aside.folded .fold {
    margin-left: 0;
  }
  /* Without the tree there is nothing to take the leftover height, so the
     settings entry is pushed to the foot where it sits when unfolded. */
  aside.folded nav {
    flex: 1;
  }
  .fold {
    margin-left: auto;
    padding: 2px;
    line-height: 0;
  }
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
    /* Both are mandatory once the divider sets an explicit height: a column
       flex item defaults to min-height auto and cannot shrink below its own
       buttons, so without these the entries spill out of the box. */
    min-height: 0;
    overflow-y: auto;
  }
  /* A 12px grab area with its hairline centred in it. The area is the target;
     the line is what the eye follows. */
  .split {
    flex: none;
    height: var(--sp-3);
    margin-bottom: calc(-1 * var(--sp-1));
    position: relative;
    cursor: row-resize;
  }
  .split::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 5px;
    height: 1px;
    background: var(--hair);
  }
  .split:hover::after,
  .split:focus-visible::after {
    background: var(--accent);
  }
  .item {
    display: flex;
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
  .section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    /* The hairline moved to `.split`, which is now what divides. */
    padding: 0 var(--sp-2) 0;
    margin-bottom: calc(-1 * var(--sp-2));
  }
  .sectionTitle {
    font-size: var(--fs-caption);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
  }
  .open {
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 4px;
    padding: 0 4px;
    font-size: var(--fs-caption);
    color: var(--accent);
  }
  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-top: var(--sp-2);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  /* The hard rule that the tree never animates its layout is enforced by
     absence, not by a suppressing declaration: `scenarios_multiproyecto.rs`
     forbids the animation keywords in this file outright, which is stricter
     than any override could be — there is nothing here to override. */
  .empty {
    margin: 0;
  }
  .remedy {
    color: var(--text-muted);
    white-space: normal;
    padding-left: 22px;
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
  .twisty,
  .quick {
    flex: none;
    padding: 0 2px;
    color: var(--text-faint);
  }
  .quick:hover:not(:disabled) {
    color: var(--accent);
  }
  .groupName {
    display: flex;
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
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
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
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
