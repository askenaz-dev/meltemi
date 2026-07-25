<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../i18n";
  import { pending } from "../stores";
  import Icon from "./Icon.svelte";

  let {
    title,
    meta,
    onOpenPalette,
    onNewSession,
    onOpenPermissions,
    urgent = false,
    children,
  }: {
    title: string;
    meta?: string;
    onOpenPalette: () => void;
    onNewSession: () => void;
    onOpenPermissions: () => void;
    /**
     * True when a pending permission is the TOP signal — that is, when the
     * daemon is reachable. With the daemon down its banner outranks the tray,
     * so the tray must not also shout (shell design D4).
     */
    urgent?: boolean;
    children?: Snippet;
  } = $props();
</script>

<header>
  <h1>{title}</h1>
  {#if meta}
    <span class="meta">{meta}</span>
  {/if}

  <div class="right">
    {#if children}{@render children()}{/if}

    <button class="search ghost" onclick={onOpenPalette}>
      <Icon name="search" size={14} />
      <span>{$t("palette.open")}</span>
      <kbd>Ctrl K</kbd>
    </button>

    <!-- The always-visible permission signal: symbol + counter + word. It
         announces itself only while it is the top signal. -->
    <button
      class="tray ghost"
      class:waiting={$pending.length > 0}
      aria-live={urgent ? "polite" : "off"}
      onclick={onOpenPermissions}
    >
      {#if $pending.length > 0}
        <span aria-hidden="true">●</span>
        <span class="pill warn counter">{$pending.length}</span>
        <span>{$t("permissions.waitingWord")}</span>
      {:else}
        <span aria-hidden="true">○</span>
        <span>{$t("nav.permissions").toLowerCase()}</span>
      {/if}
    </button>

    <button class="primary" onclick={onNewSession}>
      <Icon name="plus" size={14} />
      {$t("session.new")}
    </button>
  </div>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-4);
    border-bottom: 1px solid var(--hair);
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .meta {
    font-size: var(--fs-dense);
    color: var(--text-muted);
  }
  .right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .search,
  .tray,
  .primary {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-dense);
  }
  .search {
    border-color: var(--border);
    background: var(--surface);
    color: var(--text-muted);
  }
  .tray {
    border-color: var(--border);
    color: var(--text-muted);
  }
  .tray.waiting {
    color: var(--warn);
    border-color: var(--warn);
    font-weight: 500;
  }
  kbd {
    font: inherit;
    font-size: var(--fs-caption);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    color: var(--text-faint);
  }
</style>
