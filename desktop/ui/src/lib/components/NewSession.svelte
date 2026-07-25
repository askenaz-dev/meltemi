<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { untrack } from "svelte";
  import { get } from "svelte/store";
  import { request } from "../daemon";
  import { t } from "../i18n";
  import {
    activeProject,
    allSessions,
    fleet,
    projects,
    pushNotice,
    refreshFleet,
    refreshProjects,
    refreshSessions,
    scopedTo,
  } from "../stores";
  import { projectName } from "../tree";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  let {
    onClose,
    initialSession = null,
  }: { onClose: () => void; initialSession?: string | null } = $props();

  /** The launcher composes existing methods only — no new capability. */
  type Mode = "explore" | "propose" | "dispatch" | "direct";

  let mode: Mode = $state(untrack(() => initialSession) ? "direct" : "propose");
  let agent = $state("");
  let text = $state("");
  let change = $state("");
  let task = $state("");
  let sessionId = $state(untrack(() => initialSession) ?? "");
  /** The project this launch targets: the active one, changeable here (D6). */
  let root = $state(untrack(() => get(activeProject)) ?? "");
  let running = $state(false);
  let firstField: HTMLElement | undefined = $state();

  const MODES: { id: Mode; method: string }[] = [
    { id: "propose", method: "propose" },
    { id: "explore", method: "sdd/explore" },
    { id: "dispatch", method: "worktree/dispatch" },
    { id: "direct", method: "session/direct" },
  ];

  const launchable = $derived(
    $fleet.filter((entry) => entry.detected || entry.source === "profile"),
  );
  // Directing a session only makes sense inside the project it belongs to, so
  // the candidates follow the selected project, not the active one.
  const resumable = $derived(
    scopedTo(root, $allSessions).filter(
      (session) =>
        session.resumable ||
        session.state === "active" ||
        session.state === "waiting_permission",
    ),
  );
  /** The known projects, plus the selected root when it is not registered yet. */
  const options = $derived.by(() => {
    const roots = $projects.map((project) => project.root);
    return roots.includes(root) || root === "" ? roots : [root, ...roots];
  });

  $effect(() => {
    if ($fleet.length === 0) void refreshFleet().catch(() => {});
    void refreshProjects().catch(() => {});
  });

  $effect(() => {
    firstField?.focus();
  });

  const ready = $derived.by(() => {
    if (mode === "direct") return sessionId.trim() !== "" && text.trim() !== "";
    if (mode === "dispatch") {
      return change.trim() !== "" && task.trim() !== "" && agent.trim() !== "";
    }
    return text.trim() !== "";
  });

  async function launch() {
    if (!ready || running) return;
    if (!root) {
      pushNotice($t("session.new.noProject"), "danger");
      return;
    }
    running = true;
    try {
      if (mode === "propose") {
        await request("propose", { projectRoot: root, idea: text.trim() });
      } else if (mode === "explore") {
        await request("sdd/explore", { projectRoot: root, topic: text.trim() });
      } else if (mode === "dispatch") {
        await request("worktree/dispatch", {
          projectRoot: root,
          change: change.trim(),
          task: task.trim(),
          agent: agent.trim(),
        });
      } else {
        await request("session/direct", {
          projectRoot: root,
          sessionId: sessionId.trim(),
          instruction: text.trim(),
        });
      }
      pushNotice($t("session.new.launched"), "info");
      onClose();
    } catch (raw) {
      const e = raw as { message?: string; detail?: string };
      pushNotice(`${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`, "danger");
    } finally {
      running = false;
      void refreshSessions().catch(() => {});
    }
  }

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    } else if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void launch();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="scrim" role="presentation" onkeydown={onKeydown}>
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="new-session-title">
    <h2 id="new-session-title">{$t("session.new.title")}</h2>
    <p class="hint">{$t("session.new.hint")}</p>

    <label>
      {$t("nav.project")}
      <select bind:value={root} onchange={() => (sessionId = "")}>
        {#if options.length === 0}
          <option value="">{$t("nav.noProject")}</option>
        {/if}
        {#each options as option (option)}
          <option value={option}>{projectName(option)} — {option}</option>
        {/each}
      </select>
    </label>

    <div class="modes" role="group" aria-label={$t("session.new.mode")}>
      {#each MODES as option (option.id)}
        <button
          class:sel={mode === option.id}
          aria-pressed={mode === option.id}
          onclick={() => (mode = option.id)}
        >
          {$t(("session.mode." + option.id) as never)}
        </button>
      {/each}
    </div>
    <p class="method mono">
      {MODES.find((option) => option.id === mode)?.method}
    </p>

    {#if mode === "dispatch"}
      <label>
        {$t("session.new.change")}
        <input bind:this={firstField} bind:value={change} />
      </label>
      <label>
        {$t("session.new.task")}
        <input bind:value={task} />
      </label>
    {/if}

    {#if mode === "direct"}
      <label>
        {$t("session.new.session")}
        <select bind:value={sessionId}>
          <option value=""></option>
          {#each resumable as session (session.sessionId)}
            <option value={session.sessionId}>
              {session.sessionId.slice(0, 8)} · {$t(("state." + session.state) as never)}
            </option>
          {/each}
        </select>
      </label>
    {/if}

    {#if mode === "dispatch"}
      <div class="agents" role="group" aria-label={$t("session.new.agent")}>
        <span class="label">{$t("session.new.agent")}</span>
        {#if launchable.length === 0}
          <p class="hint">{$t("session.new.noAgents")}</p>
        {:else}
          <div class="agentList">
            {#each launchable as entry (entry.id)}
              <button
                class="agentBtn"
                class:sel={agent === entry.id}
                aria-pressed={agent === entry.id}
                onclick={() => (agent = entry.id)}
              >
                <Avatar
                  id={entry.underlyingAgent ?? entry.id}
                  name={entry.underlyingAgent ?? entry.displayName}
                  size={20}
                />
                <span class="aname">{entry.underlyingAgent ?? entry.displayName}</span>
                {#if entry.source === "profile"}
                  <span class="pill" title={$t("sessions.subscription")}>{entry.displayName}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <label>
      {mode === "direct"
        ? $t("session.new.instruction")
        : mode === "explore"
          ? $t("session.new.topic")
          : $t("session.new.idea")}
      <textarea bind:value={text} rows="3" spellcheck="false"></textarea>
    </label>

    <div class="actions">
      <button class="ghost" onclick={onClose}>{$t("common.cancel")}</button>
      <button class="primary" disabled={!ready || running} onclick={() => void launch()}>
        <Icon name="plus" size={14} />
        {running ? $t("common.loading") : $t("session.new.launch")}
      </button>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    justify-items: center;
    align-items: start;
    padding-top: 8vh;
    z-index: 35;
  }
  .sheet {
    width: min(560px, 92vw);
    max-height: 80vh;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--panel-pad);
    display: grid;
    gap: var(--sp-3);
    align-content: start;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
  .modes {
    display: flex;
    gap: var(--sp-1);
    flex-wrap: wrap;
  }
  .modes .sel {
    border-color: var(--accent);
    color: var(--accent);
  }
  .method {
    margin: 0;
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  label {
    display: grid;
    gap: var(--sp-1);
    font-size: var(--fs-dense);
    color: var(--text-muted);
  }
  .label {
    font-size: var(--fs-dense);
    color: var(--text-muted);
  }
  .agents {
    display: grid;
    gap: var(--sp-1);
  }
  .agentList {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1);
  }
  .agentBtn {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-dense);
  }
  .agentBtn.sel {
    border-color: var(--accent);
  }
  .aname {
    max-width: 18ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
</style>
