<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { conn, startConnListener } from "./lib/daemon";
  import { setLocale, t } from "./lib/i18n";
  import type { ViewId } from "./lib/registry";
  import {
    activeProject,
    initProjectScope,
    pending,
    pushNotice,
    refreshChanges,
    refreshPending,
    refreshProjects,
    refreshSessions,
    startIncomingRouter,
  } from "./lib/stores";
  import { loadUiState, setLastView } from "./lib/ui-state";
  import { dirtyFiles, requestSaveAll } from "./lib/editor/dirty";
  import {
    EMPTY_GROUPS,
    createGroup,
    forgetTab,
    joinGroup,
    leaveGroup,
    setCollapsed,
    type GroupState,
  } from "./lib/tab-groups";
  import {
    MAX_SESSION_TABS,
    clearUnread,
    closeTab,
    markUnread,
    openTab,
    type SessionTab,
  } from "./lib/session-tabs";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import SessionTabs from "./lib/components/SessionTabs.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import Notices from "./lib/components/Notices.svelte";
  import Palette from "./lib/components/Palette.svelte";
  import ProjectSwitcher from "./lib/components/ProjectSwitcher.svelte";
  import Onboarding from "./lib/components/Onboarding.svelte";
  import ConfirmDialog from "./lib/components/ConfirmDialog.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import Home from "./lib/views/Home.svelte";
  import Sessions from "./lib/views/Sessions.svelte";
  import SessionDetail from "./lib/views/SessionDetail.svelte";
  import Project from "./lib/views/Project.svelte";
  import Permissions from "./lib/views/Permissions.svelte";
  import Fleet from "./lib/views/Fleet.svelte";
  import Usage from "./lib/views/Usage.svelte";
  import Editor from "./lib/views/Editor.svelte";
  import Review from "./lib/views/Review.svelte";
  import Settings from "./lib/views/Settings.svelte";

  const KEYED_VIEWS: ViewId[] = ["sessions", "project", "permissions", "fleet", "analytics"];

  /**
   * The arrival view is the composer (lanzador-conversacional). It is also a
   * remembered view like any other, so the living promise that the last view is
   * restored on open keeps holding: a fresh profile lands on the composer, and
   * anyone who was reading a transcript comes back to where they were.
   */
  let view = $state<ViewId>("home");
  /**
   * The sessions open as tabs, in the order they were opened, and which one is
   * in front. `activeSession === null` means the list is in front — that is
   * what keeps exactly one tab selected, which a tablist requires.
   *
   * Deliberately NOT persisted (design D7): restoring eight tabs at launch
   * would cost eight log reads and eight watches against the startup budget,
   * and stale ids would greet the user with a notice about tabs vanishing.
   */
  let openSessions: SessionTab[] = $state([]);
  let activeSession: string | null = $state(null);
  /**
   * Tab groups. Not persisted, for the same reason the tabs are not: a group of
   * tabs that no longer exist has nothing to restore (D7 of the tab change).
   */
  let tabGroups: GroupState = $state(EMPTY_GROUPS);
  /** The shell only speaks of a session while the sessions view is on screen. */
  const inSession = $derived(view === "sessions" && activeSession !== null);

  /** Open a session, or bring it to the front if it is already open. */
  function openSessionTab(sessionId: string) {
    view = "sessions";
    const next = openTab(openSessions, sessionId);
    if ("full" in next && next.full) {
      // Refuses rather than evicting: a background tab can hold an unsent
      // draft. The notice names the remedy, because a limit without one is
      // just a wall.
      pushNotice($t("sessions.tabs.full", { n: String(MAX_SESSION_TABS) }), "warn");
      return;
    }
    openSessions = next.tabs;
    activeSession = next.active;
  }

  function closeSessionTab(sessionId: string) {
    const next = closeTab(openSessions, activeSession, sessionId);
    openSessions = next.tabs;
    activeSession = next.active;
    // A closed tab leaves its group, and a group left empty stops existing.
    tabGroups = forgetTab(tabGroups, sessionId);
  }

  function toggleTabGroup(groupId: string, collapsed: boolean) {
    const out = setCollapsed(
      tabGroups,
      groupId,
      collapsed,
      activeSession,
      openSessions.map((t) => t.sessionId),
    );
    tabGroups = out.state;
    activeSession = out.active;
  }
  let reviewOpen = $state(false);
  let editorContext: {
    root: string;
    target: { change: string; task: string; agent: string } | null;
    initialFile: string | null;
    initialLine: number | null;
  } | null = $state(null);

  let paletteOpen = $state(false);
  let switcherOpen = $state(false);
  /** The mode the composer opens on: free unless a caller asked for another. */
  let composerMode: "free" | "propose" | "explore" = $state("free");
  /** The project the composer opens on, when the caller named one. */
  let composerProject: string | null = $state(null);
  let onboardingOpen = $state(false);
  /** Pending navigation held by the unsaved-work guard. */
  let guard: { kind: "close" } | { kind: "leave"; go: () => void } | null = $state(null);

  /**
   * The signal that outranks the others right now (design D4 of the shell): the
   * daemon being unreachable beats a pending permission, which beats everything
   * else. Computed rather than implied, so the order cannot drift with markup.
   */
  const topSignal = $derived.by<"daemon" | "permission" | "none">(() => {
    if ($conn.state === "unreachable") return "daemon";
    if ($pending.length > 0) return "permission";
    return "none";
  });

  const overlayOpen = $derived(
    paletteOpen || onboardingOpen || switcherOpen || guard !== null,
  );

  /**
   * The drill-in trail: the first-level view, then where inside it the user is.
   * A single-entry trail means "not drilled in" and renders as the plain title.
   */
  const breadcrumb = $derived.by<string[]>(() => {
    const root = $t(("nav." + view) as never);
    if (editorContext) return [root, $t("editor.title")];
    if (reviewOpen) return [root, $t("review.title")];
    if (inSession && activeSession) {
      return [root, `${$t("sessions.detail.transcript")} · ${activeSession.slice(0, 8)}`];
    }
    return [root];
  });

  const viewTitle = $derived.by(() => {
    if (editorContext) return $t("editor.title");
    if (reviewOpen) return $t("review.title");
    if (inSession) return $t("sessions.detail.transcript");
    return $t(("nav." + view) as never);
  });

  onMount(() => {
    const cleanups: Promise<() => void>[] = [];
    cleanups.push(startConnListener());
    cleanups.push(
      startIncomingRouter((key, vars) => get(t)(key, vars)),
    );

    void (async () => {
      const state = await loadUiState();
      if (state.locale) setLocale(state.locale);
      await initProjectScope(state.activeProject);
      if (state.lastView && ["home", ...KEYED_VIEWS, "editor", "settings"].includes(state.lastView)) {
        view = state.lastView as ViewId;
      }
      const seen = await invoke<boolean>("onboarding_seen");
      if (!seen) onboardingOpen = true;
    })();

    // Regaining the focus is the answer to the attention request: the signal is
    // cleared and the OS title goes back to the product name (shell design D4).
    const onFocus = () => {
      void invoke("request_attention", {
        pending: 0,
        title: get(t)("window.title", {}),
      }).catch(() => {});
    };
    window.addEventListener("focus", onFocus);
    cleanups.push(Promise.resolve(() => window.removeEventListener("focus", onFocus)));

    // Closing the window asks the surface first when work is unsaved.
    cleanups.push(
      listen("app:close-requested", () => {
        if (get(dirtyFiles).length > 0) guard = { kind: "close" };
        else void invoke("close_confirmed");
      }),
    );

    // On connect, seed everything the always-visible chrome renders: the
    // permission tray, the session list and the PROJECT REGISTRY. Without the
    // registry the sidebar tree has no projects to group under, so every
    // session shows up as its own inferred node — a worktree session looked
    // like a separate project until the switcher happened to be opened.
    const seed = conn.subscribe((state) => {
      if (state.state !== "connected") return;
      void refreshPending().catch(() => {});
      void refreshProjects().catch(() => {});
      void refreshSessions().catch(() => {});
      void refreshChanges().catch(() => {});
    });

    return () => {
      seed();
      for (const pendingCleanup of cleanups) {
        void pendingCleanup.then((unlisten) => unlisten());
      }
    };
  });

  /**
   * Flushes every dirty editor buffer through the daemon and then continues.
   * The editor exposes the request through a store so the shell does not need a
   * handle on the component.
   */
  async function saveDirtyThen(go: () => void) {
    try {
      await requestSaveAll();
    } catch {
      // A refused save keeps the work: the guard closed, but nothing was lost,
      // and the editor already reported why.
      return;
    }
    go();
  }

  /** Navigation that respects unsaved editor work. */
  function leaveEditor(go: () => void) {
    if (editorContext && get(dirtyFiles).length > 0) {
      guard = { kind: "leave", go };
      return;
    }
    go();
  }

  function navigate(target: ViewId) {
    leaveEditor(() => {
      view = target;
      // The tabs — and which one is in front — survive: approving a permission
      // must not cost three transcripts.
      reviewOpen = false;
      editorContext = target === "editor" ? editorContext : null;
      if (target === "editor") openProjectEditor();
      // Landing on the tray puts the focus on a pending request, whichever way
      // the user got there: the indicator, the sidebar, the palette or `a`.
      if (target === "permissions") focusPendingRequest();
      setLastView(target);
    });
  }

  /**
   * The one way into the composer. Every entry point — the chrome's primary
   * action, Ctrl+N, an empty state, the Project view's Propose — comes through
   * here, so there is exactly one place that decides what "start work" means.
   */
  function openComposer(
    mode: "free" | "propose" | "explore" = "free",
    project: string | null = null,
  ) {
    leaveEditor(() => {
      view = "home";
      reviewOpen = false;
      editorContext = null;
      composerMode = mode;
      composerProject = project;
      setLastView("home");
    });
  }

  /** Focuses the first pending request of the tray, once it has rendered. */
  function focusPendingRequest() {
    setTimeout(() => {
      document.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    }, 50);
  }

  function openProjectEditor(file: string | null = null) {
    const root = get(activeProject);
    if (!root) {
      pushNotice($t("editor.noProject"), "warn");
      return;
    }
    view = "editor";
    editorContext = { root, target: null, initialFile: file, initialLine: null };
    setLastView("editor");
  }

  function isTextEntry(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return (
      tag === "INPUT" ||
      tag === "TEXTAREA" ||
      tag === "SELECT" ||
      target.isContentEditable ||
      // CodeMirror's editable surface is a contenteditable div.
      target.closest(".cm-editor") !== null
    );
  }

  function onKeydown(event: KeyboardEvent) {
    if (overlayOpen || isTextEntry(event.target)) return;

    if (event.key >= "1" && event.key <= "5") {
      navigate(KEYED_VIEWS[Number(event.key) - 1]);
      return;
    }
    if (event.key === ":" || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k")) {
      event.preventDefault();
      paletteOpen = true;
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
      event.preventDefault();
      openComposer();
      return;
    }
    if (event.key === "a") {
      navigate("permissions");
      return;
    }
    if (event.key === "?") {
      onboardingOpen = true;
      return;
    }
    // `/` opens the filter of the focused list, the same key the terminal
    // surface uses (core parity of the interaction, not only of the methods).
    if (event.key === "/" && view === "sessions" && !activeSession) {
      event.preventDefault();
      window.dispatchEvent(new CustomEvent("meltemi:filter"));
      return;
    }
    if (event.key === "Escape") {
      if (editorContext) leaveEditor(() => (editorContext = null));
      else if (reviewOpen) reviewOpen = false;
      else if (inSession) activeSession = null;
    }
  }

  async function copyDiagnostics() {
    const state = $conn;
    const text =
      state.state === "unreachable"
        ? `state=unreachable\nendpoint=${state.endpoint}\ndetail=${state.detail}`
        : `state=${state.state}`;
    try {
      await navigator.clipboard.writeText(text);
      pushNotice($t("banner.copied"), "info");
    } catch {
      pushNotice($t("banner.copyFailed"), "danger");
    }
  }

  async function retryNow() {
    try {
      await invoke("daemon_request", { method: "status", params: {} });
    } catch {
      // The bridge reports the outcome through the connection state.
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <Sidebar
    {view}
    onNavigate={navigate}
    onPickProject={() => (switcherOpen = true)}
    onNewSessionIn={(root) => openComposer("free", root)}
    onOpenSession={(sessionId) =>
      leaveEditor(() => {
        editorContext = null;
        reviewOpen = false;
        openSessionTab(sessionId);
      })}
  />

  {#if switcherOpen}
    <ProjectSwitcher onClose={() => (switcherOpen = false)} />
  {/if}

  <div class="main">
    <!-- Signal 1: the daemon being unreachable outranks everything. -->
    {#if $conn.state === "unreachable"}
      <div class="banner" role="alert">
        <strong><span aria-hidden="true">▲</span> {$t("banner.daemonDown")}</strong>
        <span>{$conn.detail}</span>
        <span class="mono">{$t("conn.endpoint")}: {$conn.endpoint}</span>
        <span>{$t("conn.willDeny")}</span>
        <span>{$t("conn.sshHint")}</span>
        <span class="bannerActions">
          <button class="ghost" onclick={() => void retryNow()}>
            <Icon name="refresh" size={12} />
            {$t("banner.retryNow")}
          </button>
          <button class="ghost" onclick={() => void copyDiagnostics()}>
            <Icon name="copy" size={12} />
            {$t("banner.copyDiagnostics")}
          </button>
        </span>
      </div>
    {/if}

    <TopBar
      title={viewTitle}
      trail={breadcrumb}
      onOpenPalette={() => (paletteOpen = true)}
      onNewSession={() => openComposer()}
      onOpenPermissions={() => navigate("permissions")}
      urgent={topSignal === "permission"}
    >
      {#if inSession || reviewOpen || editorContext}
        <button class="ghost" onclick={() => onKeydown(new KeyboardEvent("keydown", { key: "Escape" }))}>
          {$t("common.back")}
        </button>
      {/if}
    </TopBar>

    <Notices />

    <main>
      {#if editorContext}
        {#key editorContext.root + (editorContext.initialFile ?? "") + (editorContext.initialLine ?? "")}
          <Editor
            root={editorContext.root}
            target={editorContext.target}
            initialFile={editorContext.initialFile}
            initialLine={editorContext.initialLine}
            onBack={() => leaveEditor(() => (editorContext = null))}
          />
        {/key}
      {:else if reviewOpen && $activeProject}
        <Review
          root={$activeProject}
          onEditWorktree={(worktreePath, target, file, line) => {
            view = "editor";
            editorContext = {
              root: worktreePath,
              target,
              initialFile: file ?? null,
              initialLine: line ?? null,
            };
          }}
          onBack={() => (reviewOpen = false)}
        />
      {:else if view === "home"}
        <Home
          initialMode={composerMode}
          initialProject={composerProject}
          onOpenSession={(sessionId) => openSessionTab(sessionId)}
          onOpenFleet={() => navigate("fleet")}
        />
      {:else if view === "sessions"}
        <!-- The list and every open session are peers here: the list is the
             first tab, each session is a mounted panel, and the ones not in
             front are hidden rather than unmounted — which is what keeps a
             transcript, a search and an unsent draft alive (design D6). -->
        <div class="sessionSurface">
          {#if openSessions.length > 0}
            <SessionTabs
              tabs={openSessions}
              active={activeSession}
              groups={tabGroups}
              onToggleGroup={toggleTabGroup}
              onCreateGroup={(id, name) => (tabGroups = createGroup(tabGroups, id, name))}
              onJoinGroup={(id, groupId) => (tabGroups = joinGroup(tabGroups, id, groupId))}
              onLeaveGroup={(id) => (tabGroups = leaveGroup(tabGroups, id))}
              onSelect={(id) => {
                activeSession = id;
                if (id !== null) openSessions = clearUnread(openSessions, id);
              }}
              onClose={closeSessionTab}
            />
          {/if}
          <div
            class="panel"
            role={openSessions.length > 0 ? "tabpanel" : undefined}
            id="panel-__list__"
            aria-labelledby={openSessions.length > 0 ? "tab-__list__" : undefined}
            hidden={activeSession !== null}
          >
            <Sessions
              onOpen={(sessionId) => openSessionTab(sessionId)}
              onNavigate={navigate}
              onNewSession={() => openComposer()}
            />
          </div>
          {#each openSessions as tab (tab.sessionId)}
            <div
              class="panel"
              role="tabpanel"
              id="panel-{tab.sessionId}"
              aria-labelledby="tab-{tab.sessionId}"
              hidden={tab.sessionId !== activeSession}
            >
              <SessionDetail
                sessionId={tab.sessionId}
                active={tab.sessionId === activeSession}
                onBack={() => (activeSession = null)}
                onOpenSession={(id) => openSessionTab(id)}
                onActivity={() => (openSessions = markUnread(openSessions, tab.sessionId))}
              />
            </div>
          {/each}
        </div>
      {:else if view === "project"}
        <Project
          onOpenEditor={() => openProjectEditor()}
          onOpenReview={() => (reviewOpen = true)}
          onPropose={() => openComposer("propose")}
        />
      {:else if view === "permissions"}
        <Permissions />
      {:else if view === "fleet"}
        <Fleet />
      {:else if view === "analytics"}
        <Usage />
      {:else}
        <Settings onEditFile={(file) => openProjectEditor(file)} />
      {/if}
    </main>

    <StatusBar onNavigate={navigate} />
  </div>
</div>

{#if paletteOpen}
  <Palette onClose={() => (paletteOpen = false)} onNavigate={navigate} />
{/if}

{#if onboardingOpen}
  <Onboarding onClose={() => (onboardingOpen = false)} />
{/if}

{#if guard}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("editor.guard.message", { files: $dirtyFiles.join(", ") })}
    confirmLabel={$t("editor.guard.discard")}
    extraLabel={$t("editor.guard.save")}
    onExtra={() => {
      // The third path the requirement names: save, then continue. The editor
      // owns the save, so it is asked to flush before the guard releases.
      const pendingGuard = guard;
      guard = null;
      void saveDirtyThen(() => {
        if (pendingGuard?.kind === "close") void invoke("close_confirmed");
        else pendingGuard?.go();
      });
    }}
    onConfirm={() => {
      const pendingGuard = guard;
      guard = null;
      if (pendingGuard?.kind === "close") void invoke("close_confirmed");
      else pendingGuard?.go();
    }}
    onCancel={() => (guard = null)}
  />
{/if}

<style>
  .shell {
    display: flex;
    height: 100vh;
    background: var(--bg);
  }
  .main {
    flex: 1;
    min-width: 0;
    /* A column of bars whose count varies (the daemon banner and the notices
       are conditional): every bar keeps its natural height and the routed
       view takes the remainder. A fixed grid row template cannot express
       that — when a conditional bar is absent, auto-placement shifts the
       view out of its 1fr track and the shell stops filling the window. */
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .main > :global(:not(main)) {
    flex: 0 0 auto;
  }
  main {
    flex: 1 1 0;
    overflow: hidden;
    min-height: 0;
  }
  /* The strip sits above the panels and every panel takes the rest. `hidden`
     needs the explicit `display: none` because a flex child's display would
     otherwise win over the attribute's UA default. */
  .sessionSurface {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
    gap: var(--sp-2);
  }
  .sessionSurface .panel {
    flex: 1 1 0;
    min-height: 0;
  }
  .sessionSurface .panel[hidden] {
    display: none;
  }
  .banner {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-3);
    align-items: baseline;
    padding: var(--sp-2) var(--sp-4);
    background: var(--danger);
    color: #fff;
    font-size: var(--fs-dense);
  }
  .banner .mono {
    font-family: var(--font-mono);
  }
  .bannerActions {
    margin-left: auto;
    display: flex;
    gap: var(--sp-2);
  }
  .bannerActions button {
    color: #fff;
    border-color: rgb(255 255 255 / 0.5);
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: var(--sp-1);
    font-size: var(--fs-caption);
  }
</style>
