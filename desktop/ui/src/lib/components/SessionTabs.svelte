<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The session vocabulary, kept out of both `TabStrip` and the shell: this is the
  only place that knows a tab stands for a session. The list is the first tab
  and is never closable — an empty selection is invalid in a tablist, and the
  list is where Escape and the last close both land.
-->
<script lang="ts">
  import { t } from "../i18n";
  import { allSessions } from "../stores";
  import { agentLabelOf } from "../tree";
  import type { SessionTab } from "../session-tabs";
  import { groupOf, type GroupState } from "../tab-groups";
  import StatusBadge from "./StatusBadge.svelte";
  import TabStrip, { type TabItem } from "./TabStrip.svelte";

  let {
    tabs,
    active,
    groups,
    onSelect,
    onClose,
    onToggleGroup,
    onCreateGroup,
    onJoinGroup,
    onLeaveGroup,
  }: {
    tabs: SessionTab[];
    /** `null` means the list is in front. */
    active: string | null;
    groups: GroupState;
    onSelect: (sessionId: string | null) => void;
    onClose: (sessionId: string) => void;
    onToggleGroup: (groupId: string, collapsed: boolean) => void;
    onCreateGroup: (sessionId: string, name: string) => void;
    onJoinGroup: (sessionId: string, groupId: string) => void;
    onLeaveGroup: (sessionId: string) => void;
  } = $props();

  /** The per-tab actions menu, open for one tab at a time. */
  let menuFor: string | null = $state(null);
  let newName = $state("");

  /** The id the strip uses for the list. Sessions are UUIDs; this is not one. */
  const LIST = "__list__";

  // Resolved against the FULL listing, not the project-scoped one: a tab holding
  // a session from another project must still render its agent and its state
  // rather than going blank after a project switch.
  const infoOf = $derived((sessionId: string) =>
    $allSessions.find((session) => session.sessionId === sessionId),
  );

  const items: TabItem[] = $derived([
    { id: LIST, label: $t("sessions.tabs.list"), closable: false },
    ...tabs.map((tab) => {
      const info = infoOf(tab.sessionId);
      const agent = info ? agentLabelOf(info) : $t("sessions.tabs.gone");
      const group = groupOf(groups, tab.sessionId);
      return {
        id: tab.sessionId,
        label: `${agent} ${tab.sessionId.slice(0, 8)}`,
        // The full story the label had to shorten, project scope included, so
        // the strip never lies about where a session lives.
        title: info?.projectRoot
          ? `${agent} · ${tab.sessionId} · ${info.projectRoot}`
          : `${agent} · ${tab.sessionId}`,
        closable: true,
        group: group
          ? {
              id: group.id,
              name: group.name,
              color: group.color,
              collapsed: group.collapsed,
              size: group.members.length,
            }
          : undefined,
      };
    }),
  ]);

  function unreadOf(id: string): number {
    return tabs.find((tab) => tab.sessionId === id)?.unread ?? 0;
  }
</script>

<TabStrip
  {items}
  activeId={active ?? LIST}
  label={$t("sessions.tabs")}
  closeLabel={$t("sessions.tabs.close")}
  scrollLeftLabel={$t("tabs.scrollLeft")}
  scrollRightLabel={$t("tabs.scrollRight")}
  onToggleGroup={(id, collapsed) => onToggleGroup(id, collapsed)}
  collapsedLabel={(g) => $t("tabs.group.collapsed", { name: g.name, n: String(g.size) })}
  onSelect={(id) => onSelect(id === LIST ? null : id)}
  onClose={(id) => onClose(id)}
>
  {#snippet menu(item)}
    {#if item.id !== LIST}
      <button
        class="dots ghost"
        aria-haspopup="menu"
        aria-expanded={menuFor === item.id}
        aria-label={$t("tabs.group.menu")}
        title={$t("tabs.group.menu")}
        onclick={() => ((menuFor = menuFor === item.id ? null : item.id), (newName = ""))}
      >
        ⋯
      </button>
    {/if}
  {/snippet}
  {#snippet mark(item)}
    {#if item.id !== LIST}
      {@const info = infoOf(item.id)}
      {#if info}
        <StatusBadge state={info.state} />
      {/if}
      {#if unreadOf(item.id) > 0}
        <span
          class="pill unread"
          aria-label={$t("sessions.tabs.unread", { n: String(unreadOf(item.id)) })}
        >
          {unreadOf(item.id)}
        </span>
      {/if}
    {/if}
  {/snippet}
</TabStrip>

<style>
  .dots {
    padding: 0 4px;
    line-height: 1;
    color: var(--text-faint);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: fixed;
    top: 96px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 41;
    display: grid;
    gap: var(--sp-1);
    min-width: 240px;
    padding: var(--sp-2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
  }
  .menu button {
    justify-content: flex-start;
    font-size: var(--fs-dense);
  }
  .newGroup {
    display: flex;
    gap: var(--sp-1);
  }
  .newGroup input {
    flex: 1;
    min-width: 0;
  }
  .unread {
    /* The count is a number with a name, never a coloured dot alone. */
    background: var(--tint-warn);
    color: var(--surface);
  }
</style>

{#if menuFor}
  {@const target = menuFor}
  <div
    class="scrim"
    role="presentation"
    onclick={() => (menuFor = null)}
    onkeydown={(event) => {
      if (event.key === "Escape") menuFor = null;
    }}
  ></div>
  <div class="menu" role="menu" aria-label={$t("tabs.group.menu")}>
    <form
      class="newGroup"
      onsubmit={(event) => {
        event.preventDefault();
        if (!newName.trim()) return;
        onCreateGroup(target, newName.trim());
        menuFor = null;
      }}
    >
      <input
        bind:value={newName}
        placeholder={$t("tabs.group.newPlaceholder")}
        aria-label={$t("tabs.group.new")}
      />
      <button type="submit" disabled={!newName.trim()}>{$t("tabs.group.new")}</button>
    </form>
    {#each groups.groups as group (group.id)}
      <button
        role="menuitem"
        class="ghost"
        onclick={() => {
          onJoinGroup(target, group.id);
          menuFor = null;
        }}
      >
        {$t("tabs.group.join", { name: group.name })}
      </button>
    {/each}
    {#if groupOf(groups, target)}
      <button
        role="menuitem"
        class="ghost"
        onclick={() => {
          onLeaveGroup(target);
          menuFor = null;
        }}
      >
        {$t("tabs.group.leave")}
      </button>
    {/if}
  </div>
{/if}
