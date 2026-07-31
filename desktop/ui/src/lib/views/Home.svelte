<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The conversational home (lanzador-conversacional): the arrival view is a
  composer with its context as chips inside it — project, agent or profile, and
  mode — and the free mode is the default. The method is not a toll: proposing
  and exploring are two chips away in the same composer, and the contract method
  each mode dispatches is written next to the send button before anything runs.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { request } from "../daemon";
  import { t } from "../i18n";
  import {
    activeProject,
    fleet,
    projects,
    pushNotice,
    refreshFleet,
    refreshProjects,
    refreshSessions,
    sessions,
    switchProject,
  } from "../stores";
  import { agentLabelOf, projectName } from "../tree";
  import Avatar from "../components/Avatar.svelte";
  import Chip from "../components/Chip.svelte";
  import Icon from "../components/Icon.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";

  let {
    onOpenSession,
  }: {
    onOpenSession: (sessionId: string) => void;
  } = $props();

  /** The three modes of the composer. Free is the default; the rest are offers. */
  type Mode = "free" | "propose" | "explore";

  const METHOD: Record<Mode, string> = {
    free: "session/start",
    propose: "propose",
    explore: "sdd/explore",
  };

  const MODES: Mode[] = ["free", "propose", "explore"];

  let mode: Mode = $state("free");
  /** The chosen fleet entry id, or "" for the project's configured agent. */
  let agent = $state("");
  let text = $state("");
  let running = $state(false);
  let box: HTMLTextAreaElement | undefined = $state();

  const root = $derived($activeProject ?? "");

  /** Agents that can actually be launched: detected, or a declared profile. */
  const launchable = $derived(
    $fleet.filter((entry) => entry.detected || entry.source === "profile"),
  );

  const chosen = $derived(launchable.find((entry) => entry.id === agent));

  const agentLabel = $derived(
    chosen ? (chosen.displayName ?? chosen.id) : $t("home.agent.default"),
  );

  /** The most recent sessions of this project: a conversation to walk back into. */
  const recent = $derived($sessions.slice(0, 4));

  const ready = $derived(text.trim() !== "" && root !== "" && !running);

  // Seeded once on arrival, not tracked: reading `$fleet` here would re-run the
  // effect on its own answer and refetch the rest for nothing.
  onMount(() => {
    if (get(fleet).length === 0) void refreshFleet().catch(() => {});
    void refreshProjects().catch(() => {});
    void refreshSessions().catch(() => {});
  });

  // The composer is what the user arrived for: it holds the caret.
  $effect(() => {
    box?.focus();
  });

  /** Grows with the instruction instead of scrolling a three-line slot. */
  function grow(): void {
    if (!box) return;
    box.style.height = "auto";
    box.style.height = `${Math.min(box.scrollHeight, 320)}px`;
  }

  function paramsFor(mode: Mode): Record<string, unknown> {
    const instruction = text.trim();
    const params: Record<string, unknown> =
      mode === "free"
        ? { projectRoot: root, instruction }
        : mode === "propose"
          ? { projectRoot: root, idea: instruction }
          : { projectRoot: root, topic: instruction };
    // Absent means "the project's configured agent", which is not the same as
    // an empty name: the contract's optional parameter stays absent.
    if (agent) params.agent = agent;
    return params;
  }

  async function send(): Promise<void> {
    if (!ready) return;
    if (!root) {
      pushNotice($t("session.new.noProject"), "danger");
      return;
    }
    running = true;
    const dispatched = mode;
    try {
      await request(METHOD[dispatched], paramsFor(dispatched));
      pushNotice($t("session.new.launched"), "info");
      text = "";
    } catch (raw) {
      const e = raw as { message?: string; detail?: string };
      pushNotice(`${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`, "danger");
    } finally {
      running = false;
      void refreshSessions().catch(() => {});
    }
  }

  function onBoxKeydown(event: KeyboardEvent): void {
    event.stopPropagation();
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void send();
    }
  }
</script>

<div class="home">
  <div class="stage">
    <h2>{$t("home.title")}</h2>
    <p class="promise">{$t("home.promise")}</p>

    <div class="composer" class:busy={running}>
      <textarea
        bind:this={box}
        bind:value={text}
        oninput={grow}
        onkeydown={onBoxKeydown}
        rows="3"
        spellcheck="false"
        aria-label={$t(("home.placeholder." + mode) as never)}
        placeholder={$t(("home.placeholder." + mode) as never)}
      ></textarea>

      <div class="chips">
        <Chip label={$t("nav.project")} value={root ? projectName(root) : $t("nav.noProject")} title={root} tone={root ? "plain" : "warn"}>
          {#snippet menu(close)}
            {#if $projects.length === 0}
              <p class="none">{$t("projects.empty")}</p>
            {/if}
            {#each $projects as project (project.projectKey)}
              <button
                class="item"
                aria-current={project.root === root ? "true" : undefined}
                onclick={() => {
                  switchProject(project.root);
                  close();
                }}
              >
                <span class="itemName">{projectName(project.root)}</span>
                {#if !project.exists}
                  <span class="pill danger">{$t("projects.absent")}</span>
                {/if}
                <span class="itemMeta mono">{project.root}</span>
              </button>
            {/each}
          {/snippet}
        </Chip>

        <Chip label={$t("session.new.agent")} value={agentLabel} title={chosen?.id ?? ""}>
          {#snippet lead()}
            {#if chosen}
              <Avatar
                id={chosen.underlyingAgent ?? chosen.id}
                name={chosen.underlyingAgent ?? chosen.displayName}
                size={16}
              />
            {/if}
          {/snippet}
          {#snippet menu(close)}
            <button
              class="item"
              aria-current={agent === "" ? "true" : undefined}
              onclick={() => {
                agent = "";
                close();
              }}
            >
              <span class="itemName">{$t("home.agent.default")}</span>
              <span class="itemMeta">{$t("home.agent.default.hint")}</span>
            </button>
            {#if launchable.length === 0}
              <p class="none">{$t("session.new.noAgents")}</p>
            {/if}
            {#each launchable as entry (entry.id)}
              <button
                class="item"
                aria-current={agent === entry.id ? "true" : undefined}
                onclick={() => {
                  agent = entry.id;
                  close();
                }}
              >
                <Avatar
                  id={entry.underlyingAgent ?? entry.id}
                  name={entry.underlyingAgent ?? entry.displayName}
                  size={18}
                />
                <span class="itemName">{entry.underlyingAgent ?? entry.displayName}</span>
                {#if entry.source === "profile"}
                  <span class="pill" title={$t("sessions.subscription")}>{entry.displayName}</span>
                {/if}
              </button>
            {/each}
          {/snippet}
        </Chip>

        <Chip label={$t("session.new.mode")} value={$t(("session.mode." + mode) as never)}>
          {#snippet menu(close)}
            {#each MODES as option (option)}
              <button
                class="item"
                aria-current={mode === option ? "true" : undefined}
                onclick={() => {
                  mode = option;
                  close();
                }}
              >
                <span class="itemName">{$t(("session.mode." + option) as never)}</span>
                <span class="itemMeta">{$t(("home.mode." + option + ".hint") as never)}</span>
                <code class="itemMethod">{METHOD[option]}</code>
              </button>
            {/each}
          {/snippet}
        </Chip>

        <span class="method" aria-label={$t("home.method")}>
          <span class="methodLabel">{$t("home.method")}</span>
          <code>{METHOD[mode]}</code>
        </span>

        <button
          class="primary send"
          disabled={!ready}
          title={$t("home.sendHint")}
          onclick={() => void send()}
        >
          <Icon name="plus" size={14} />
          {running ? $t("common.loading") : $t("home.send")}
        </button>
      </div>
    </div>

    {#if !root}
      <p class="warnLine">{$t("session.new.noProject")}</p>
    {/if}

    {#if recent.length > 0}
      <div class="recent">
        <span class="recentLabel">{$t("home.recent")}</span>
        {#each recent as session (session.sessionId)}
          <button class="ghost recentItem" onclick={() => onOpenSession(session.sessionId)}>
            <Avatar id={agentLabelOf(session)} size={16} />
            <span class="recentName">{agentLabelOf(session)}</span>
            <StatusBadge state={session.state} />
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .home {
    height: 100%;
    overflow-y: auto;
    display: grid;
    justify-items: center;
    align-content: center;
    padding: var(--sp-8) var(--sp-4);
  }
  .stage {
    width: min(760px, 100%);
    display: grid;
    gap: var(--sp-2);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-view);
    font-weight: 500;
  }
  .promise {
    margin: 0 0 var(--sp-2);
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
  /* The composer is one surface: the field and its context read as a single
     object, so the chips sit inside the frame rather than beside it. */
  .composer {
    display: grid;
    gap: var(--sp-2);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: var(--sp-2);
    box-shadow: var(--shadow-overlay);
  }
  .composer:focus-within {
    border-color: var(--accent);
  }
  .composer textarea {
    border: 0;
    background: transparent;
    resize: none;
    padding: var(--sp-2);
    font-family: var(--font-ui);
    font-size: var(--fs-body);
    line-height: 1.5;
    min-height: 72px;
  }
  .composer textarea:focus-visible {
    outline: none;
  }
  .chips {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-wrap: wrap;
  }
  .method {
    display: inline-flex;
    align-items: baseline;
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 6px;
    margin-left: auto;
    font-size: var(--fs-caption);
  }
  .methodLabel {
    color: var(--text-faint);
  }
  .method code {
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  .send {
    font-size: var(--fs-dense);
  }
  .warnLine {
    margin: 0;
    color: var(--warn);
    font-size: var(--fs-dense);
  }
  /* Menu items (rendered into Chip's popover through its snippet). */
  .item {
    display: flex;
    align-items: center;
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 6px;
    width: 100%;
    text-align: left;
    background: transparent;
    border-color: transparent;
    border-radius: var(--radius-control);
    padding: var(--sp-1) var(--sp-2);
    font-size: var(--fs-dense);
  }
  .item:hover {
    background: var(--surface-2);
    border-color: transparent;
  }
  .item[aria-current="true"] {
    background: var(--surface-2);
  }
  .item[aria-current="true"]::before {
    content: "▸";
    color: var(--accent);
  }
  .itemName {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .itemMeta {
    margin-left: auto;
    color: var(--text-faint);
    font-size: var(--fs-caption);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 22ch;
  }
  .itemMethod {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  .none {
    margin: 0;
    padding: var(--sp-2);
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .recent {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-wrap: wrap;
    margin-top: var(--sp-2);
  }
  .recentLabel {
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .recentItem {
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 6px;
    font-size: var(--fs-caption);
    border-color: var(--hair);
  }
  .recentName {
    max-width: 16ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
