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
  import StatusBadge from "./StatusBadge.svelte";
  import TabStrip, { type TabItem } from "./TabStrip.svelte";

  let {
    tabs,
    active,
    onSelect,
    onClose,
  }: {
    tabs: SessionTab[];
    /** `null` means the list is in front. */
    active: string | null;
    onSelect: (sessionId: string | null) => void;
    onClose: (sessionId: string) => void;
  } = $props();

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
      return {
        id: tab.sessionId,
        label: `${agent} ${tab.sessionId.slice(0, 8)}`,
        // The full story the label had to shorten, project scope included, so
        // the strip never lies about where a session lives.
        title: info?.projectRoot
          ? `${agent} · ${tab.sessionId} · ${info.projectRoot}`
          : `${agent} · ${tab.sessionId}`,
        closable: true,
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
  onSelect={(id) => onSelect(id === LIST ? null : id)}
  onClose={(id) => onClose(id)}
>
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
  .unread {
    /* The count is a number with a name, never a coloured dot alone. */
    background: var(--tint-warn);
    color: var(--surface);
  }
</style>
