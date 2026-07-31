<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The conversational home (lanzador-conversacional): the arrival view is a
  composer with its context as chips inside it — project, agent or profile, and
  mode — and the free mode is the default. The method is not a toll: proposing
  and exploring are two chips away in the same composer, and the contract method
  each mode dispatches is written next to the send button before anything runs.
-->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { get } from "svelte/store";
  import { request, type AgentCandidate, type DaemonError } from "../daemon";
  import { t } from "../i18n";
  import {
    activeProject,
    allSessions,
    fleet,
    onSessionEvent,
    projects,
    pushNotice,
    refreshFleet,
    refreshProjects,
    refreshSessions,
    scopedTo,
  } from "../stores";
  import { agentLabelOf, projectName } from "../tree";
  import Avatar from "../components/Avatar.svelte";
  import Chip from "../components/Chip.svelte";
  import Icon from "../components/Icon.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";

  /** The three modes of the composer. Free is the default; the rest are offers. */
  type Mode = "free" | "propose" | "explore";

  let {
    onOpenSession,
    initialMode = "free",
  }: {
    onOpenSession: (sessionId: string) => void;
    /** The mode the caller asked for: Propose routes here with it chosen. */
    initialMode?: Mode;
  } = $props();

  const METHOD: Record<Mode, string> = {
    free: "session/start",
    propose: "propose",
    explore: "sdd/explore",
  };

  const MODES: Mode[] = ["free", "propose", "explore"];

  // Read once: after arriving, the mode belongs to the user's chip, not to
  // whoever opened the composer.
  let mode: Mode = $state(untrack(() => initialMode));
  /** The chosen fleet entry id, or "" for the project's configured agent. */
  let agent = $state("");
  let text = $state("");
  let running = $state(false);
  let box: HTMLTextAreaElement | undefined = $state();
  /**
   * A refused agent resolution, with the fleet candidates the daemon attached.
   * It is NOT a banner: the refusal moves into the agent chip, which is the
   * control that fixes it, and the chip opens itself so the remedy is where the
   * user is already looking.
   */
  let refusal: { remedy: string | null; candidates: AgentCandidate[] } | null = $state(null);
  let agentMenuOpen = $state(false);

  /**
   * The project this launch targets. It follows the active one until the user
   * names another in the chip, and naming another does NOT move the surface's
   * scope: choosing where to work is not the same as going there.
   */
  let picked: string | null = $state(null);
  const root = $derived(picked ?? $activeProject ?? "");

  /** Agents that can actually be launched: detected, or a declared profile. */
  const launchable = $derived(
    $fleet.filter((entry) => entry.detected || entry.source === "profile"),
  );

  const chosen = $derived(launchable.find((entry) => entry.id === agent));

  const agentLabel = $derived(
    refusal
      ? $t("home.agent.unresolved")
      : chosen
        ? (chosen.displayName ?? chosen.id)
        : $t("home.agent.default"),
  );

  /** One row of the agent menu, whichever list it came from. */
  interface AgentOption {
    id: string;
    name: string;
    profile: string | null;
    avatar: string;
    detected: boolean;
    installState: string;
    remedy: string | null;
  }

  /** What the agent menu offers: the refusal's candidates once there is one. */
  const offered = $derived.by<AgentOption[]>(() => {
    const refused = refusal;
    if (refused) {
      return refused.candidates.map((candidate) => ({
        id: candidate.id,
        name: candidate.id,
        profile: null,
        avatar: candidate.id,
        detected: candidate.detected,
        installState: candidate.installState,
        remedy: candidate.remedy ?? null,
      }));
    }
    return launchable.map((entry) => ({
      id: entry.id,
      name: entry.underlyingAgent ?? entry.displayName,
      profile: entry.source === "profile" ? entry.displayName : null,
      avatar: entry.underlyingAgent ?? entry.id,
      detected: true,
      installState: entry.installState ?? "ready",
      remedy: null,
    }));
  });

  /** The most recent sessions of this project: a conversation to walk back into. */
  const recent = $derived(root ? scopedTo(root, $allSessions).slice(0, 4) : []);

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
    // Every start verb blocks until its turn ends, so waiting for the result to
    // navigate would leave the user staring at the composer for a whole turn.
    // The daemon publishes `session_started` to the connection that launched —
    // no subscription asked for — and it carries the identifier before the
    // agent's first token (design D3). That is what we walk in on; the request
    // stays in flight and still reports its outcome from inside the session.
    let entered = false;
    const stop = onSessionEvent((message) => {
      if (message.event.type !== "session_started") return;
      stop();
      entered = true;
      text = "";
      running = false;
      void refreshSessions().catch(() => {});
      onOpenSession(message.sessionId);
    });
    try {
      await request(METHOD[dispatched], paramsFor(dispatched));
      if (!entered) {
        // No arrival event reached us: say what happened rather than pretend a
        // conversation was opened.
        pushNotice($t("session.new.launched"), "info");
        text = "";
      }
    } catch (raw) {
      const e = raw as Partial<DaemonError>;
      if (e?.candidates) {
        // The daemon refused to resolve an agent and said which ones it can
        // see. That is a choice, not a lament: it goes into the chip.
        refusal = { remedy: e.remedy ?? null, candidates: e.candidates };
        agentMenuOpen = true;
      } else {
        pushNotice(`${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`, "danger");
      }
    } finally {
      stop();
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
                  picked = project.root;
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

        <Chip
          bind:open={agentMenuOpen}
          label={$t("session.new.agent")}
          value={agentLabel}
          title={chosen?.id ?? ""}
          tone={refusal ? "warn" : "plain"}
        >
          {#snippet lead()}
            {#if chosen && !refusal}
              <Avatar
                id={chosen.underlyingAgent ?? chosen.id}
                name={chosen.underlyingAgent ?? chosen.displayName}
                size={16}
              />
            {/if}
          {/snippet}
          {#snippet menu(close)}
            {#if refusal}
              <p class="refused">{$t("home.agent.refused")}</p>
              {#if refusal.remedy}
                <p class="none">{refusal.remedy}</p>
              {/if}
            {:else}
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
            {/if}
            {#if offered.length === 0}
              <p class="none">{$t("session.new.noAgents")}</p>
              <p class="none">{$t("fleet.remedy.hint")}</p>
            {/if}
            {#each offered as entry (entry.id)}
              <button
                class="item"
                disabled={!entry.detected}
                aria-current={agent === entry.id ? "true" : undefined}
                onclick={() => {
                  agent = entry.id;
                  refusal = null;
                  close();
                }}
              >
                <Avatar id={entry.avatar} name={entry.name} size={18} />
                <span class="itemName">{entry.name}</span>
                {#if entry.profile}
                  <span class="pill" title={$t("sessions.subscription")}>{entry.profile}</span>
                {/if}
                {#if !entry.detected}
                  <span class="pill warn">
                    {$t(("fleet.state." + entry.installState) as never)}
                  </span>
                {/if}
                {#if entry.remedy && !entry.detected}
                  <span class="itemMeta">{entry.remedy}</span>
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
  .refused {
    margin: 0;
    padding: var(--sp-2) var(--sp-2) 0;
    color: var(--warn);
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
