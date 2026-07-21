<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { conn, startConnListener } from "./lib/daemon";
  import { t } from "./lib/i18n";
  import type { ViewId } from "./lib/registry";
  import {
    dismissNotice,
    loadProjectRoot,
    notices,
    pending,
    refreshPending,
    startIncomingRouter,
  } from "./lib/stores";
  import Sessions from "./lib/views/Sessions.svelte";
  import SessionDetail from "./lib/views/SessionDetail.svelte";
  import Project from "./lib/views/Project.svelte";
  import Permissions from "./lib/views/Permissions.svelte";
  import Fleet from "./lib/views/Fleet.svelte";
  import Palette from "./lib/components/Palette.svelte";
  import Onboarding from "./lib/components/Onboarding.svelte";
  import { get } from "svelte/store";

  const VIEWS: ViewId[] = ["sessions", "project", "permissions", "fleet"];

  let view: ViewId = $state("sessions");
  let detailSession: string | null = $state(null);
  let paletteOpen = $state(false);
  let onboardingOpen = $state(false);

  const overlayOpen = $derived(paletteOpen || onboardingOpen);

  onMount(() => {
    const cleanups: Promise<() => void>[] = [];
    cleanups.push(startConnListener());
    cleanups.push(startIncomingRouter((key, vars) => get(t)(key, vars)));
    void loadProjectRoot();
    void invoke<boolean>("onboarding_seen").then((seen) => {
      if (!seen) onboardingOpen = true;
    });
    // Seed the tray from the daemon's queue once connected (survives
    // reconnection: the count lives in the daemon, not per-connection).
    const seed = conn.subscribe((state) => {
      if (state.state === "connected") {
        void refreshPending().catch(() => {});
      }
    });
    return () => {
      seed();
      for (const pending of cleanups) {
        void pending.then((unlisten) => unlisten());
      }
    };
  });

  function navigate(target: ViewId) {
    view = target;
    detailSession = null;
  }

  function isTextEntry(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return (
      tag === "INPUT" ||
      tag === "TEXTAREA" ||
      tag === "SELECT" ||
      target.isContentEditable
    );
  }

  function onKeydown(event: KeyboardEvent) {
    // Overlays trap their own keys (announced way out: Esc).
    if (overlayOpen || isTextEntry(event.target)) return;

    if (event.key >= "1" && event.key <= "4") {
      navigate(VIEWS[Number(event.key) - 1]);
      return;
    }
    if (event.key === ":" || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k")) {
      event.preventDefault();
      paletteOpen = true;
      return;
    }
    if (event.key === "a") {
      // Jump to the tray AND focus a pending request (tui-shell parity).
      navigate("permissions");
      setTimeout(() => {
        document.querySelector<HTMLElement>("[data-autofocus]")?.focus();
      }, 50);
      return;
    }
    if (event.key === "?") {
      onboardingOpen = true;
      return;
    }
    if (event.key === "Escape" && detailSession) {
      detailSession = null;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="chrome">
  <header>
    <span class="brand">{$t("app.title")}</span>

    <nav aria-label={$t("nav.viewLabel")}>
      {#each VIEWS as id, index (id)}
        <button
          aria-current={view === id && !detailSession ? "page" : undefined}
          class:current={view === id}
          onclick={() => navigate(id)}
        >
          <span class="key" aria-hidden="true">{index + 1}</span>
          {$t(("nav." + id) as never)}
        </button>
      {/each}
    </nav>

    <div class="signals">
      <!-- Permission indicator: symbol + counter + word, always visible. -->
      <button
        class="tray"
        class:waiting={$pending.length > 0}
        onclick={() => navigate("permissions")}
      >
        {#if $pending.length > 0}
          ● {$t("permissions.waiting", { n: $pending.length })}
        {:else}
          ○ {$t("nav.permissions").toLowerCase()}
        {/if}
      </button>

      <span class="conn" role="status">
        {#if $conn.state === "connecting"}
          <span class="info">◌ {$t("conn.connecting")}</span>
        {:else if $conn.state === "connected"}
          <span class="ok">▸ {$t("conn.connected")}</span>
          <span class="muted">
            v{$conn.version} · {$t("conn.sessions", { n: $conn.sessions })}
          </span>
        {:else}
          <span class="danger">▲ {$t("conn.unreachable")}</span>
        {/if}
      </span>
    </div>
  </header>

  {#if detailSession}
    <div class="breadcrumb" aria-label={$t("nav.breadcrumb")}>
      <button class="link" onclick={() => (detailSession = null)}>
        {$t("nav.sessions")}
      </button>
      <span aria-hidden="true">›</span>
      <code>{detailSession.slice(0, 8)}</code>
    </div>
  {/if}

  <!-- Signal priority: daemon down outranks everything (assertive). -->
  {#if $conn.state === "unreachable"}
    <div class="banner" role="alert">
      <strong>▲ {$t("banner.daemonDown")}</strong>
      <span>{$conn.detail}</span>
      <span class="mono">{$t("conn.endpoint")}: {$conn.endpoint}</span>
      <span>{$t("conn.willDeny")}</span>
      <span class="muted">{$t("banner.retrying")} · {$t("conn.sshHint")}</span>
    </div>
  {/if}

  {#if $notices.length > 0}
    <div class="notices" role="region" aria-label={$t("notices.title")} aria-live="polite">
      {#each $notices as notice (notice.id)}
        <div class="notice tone-{notice.tone}">
          <span>{notice.text}</span>
          <button class="link" onclick={() => dismissNotice(notice.id)}>
            {$t("notices.dismiss")}
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <main>
    {#if detailSession}
      <SessionDetail
        sessionId={detailSession}
        onBack={() => (detailSession = null)}
      />
    {:else if view === "sessions"}
      <Sessions
        onOpen={(sessionId) => (detailSession = sessionId)}
        onNavigate={navigate}
      />
    {:else if view === "project"}
      <Project />
    {:else if view === "permissions"}
      <Permissions />
    {:else}
      <Fleet />
    {/if}
  </main>

  <footer>
    <span class="muted">{$t("help.keys")}</span>
  </footer>
</div>

{#if paletteOpen}
  <Palette onClose={() => (paletteOpen = false)} onNavigate={navigate} />
{/if}

{#if onboardingOpen}
  <Onboarding onClose={() => (onboardingOpen = false)} />
{/if}

<style>
  .chrome {
    height: 100vh;
    display: grid;
    grid-template-rows: auto auto auto auto 1fr auto;
    background: var(--bg);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-2) var(--sp-4);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .brand {
    font-weight: 600;
    background: linear-gradient(45deg, var(--mel-aegean), var(--mel-wind));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  nav {
    display: flex;
    gap: var(--sp-1);
  }
  nav button {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  nav button.current {
    color: var(--text);
    border-color: var(--border);
    background: var(--surface-2);
  }
  .key {
    display: inline-block;
    font-size: 0.6875rem;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    margin-right: 2px;
  }
  .signals {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }
  .tray {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }
  .tray.waiting {
    color: var(--warn);
    border-color: var(--warn);
    font-weight: 600;
  }
  .conn {
    display: inline-flex;
    gap: var(--sp-2);
    align-items: baseline;
    font-size: 0.8125rem;
  }
  .breadcrumb {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
    padding: var(--sp-2) var(--sp-4);
    border-bottom: 1px solid var(--border);
    font-size: 0.8125rem;
  }
  .banner {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-3);
    align-items: baseline;
    padding: var(--sp-2) var(--sp-4);
    background: var(--danger);
    color: #fff;
  }
  .banner .muted {
    color: rgb(255 255 255 / 0.8);
  }
  .notices {
    display: grid;
    gap: 1px;
  }
  .notice {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: var(--sp-1) var(--sp-4);
    font-size: 0.8125rem;
    background: var(--surface-2);
  }
  .tone-warn {
    color: var(--warn);
  }
  .tone-danger {
    color: var(--danger);
  }
  .tone-info {
    color: var(--info);
  }
  main {
    padding: var(--sp-4);
    overflow: auto;
    min-height: 0;
  }
  footer {
    padding: var(--sp-1) var(--sp-4);
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
  }
  .muted {
    color: var(--text-muted);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .ok {
    color: var(--ok);
  }
  .info {
    color: var(--info);
  }
  .danger {
    color: var(--danger);
  }
  .link {
    font: inherit;
    border: 0;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    padding: 0;
  }
</style>
