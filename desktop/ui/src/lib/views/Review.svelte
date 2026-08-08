<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
  The race board (tablero-de-carrera design D3): the review drill-in grown into
  what the daemon has been running all along. Lanes side by side, each with the
  provenance of the turn that ran it, its state in signal plus word, and its own
  diff against its own base. Nothing here is inferred: a lane the daemon said
  nothing about shows nothing, marked as unknown rather than filled in.
-->
<script lang="ts">
  import { t } from "../i18n";
  import { request } from "../daemon";
  import { fileSections, hunksOf } from "../diff";
  import { openWith } from "../editor/files";
  import { allSessions, onSessionEvent, pushNotice, type SessionInfo } from "../stores";
  import { REGISTRY } from "../registry";
  import type { MessageKey } from "../messages";
  import EmptyState from "../components/EmptyState.svelte";
  import MethodForm from "../components/MethodForm.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";

  let {
    root,
    onEditWorktree,
    onBack,
  }: {
    root: string;
    onEditWorktree: (
      worktreePath: string,
      target: { change: string; task: string; agent: string },
      file?: string,
      line?: number,
    ) => void;
    onBack: () => void;
  } = $props();

  interface Worktree {
    change: string;
    task: string;
    agent: string;
    path: string;
    branch: string;
    baseRev: string;
    competitor: boolean;
  }

  /** One lane of the race, as `worktree/diff` reports it. Every provenance
      field is optional on the wire: absent means the daemon has no dispatch on
      record for that lane, and the board says exactly that. */
  interface CompetitorDiff {
    agent: string;
    path: string;
    changedFiles: string[];
    diff: string;
    source?: "profile" | "catalog" | "configured";
    profile?: string;
    level?: number;
    sessionId?: string;
    committed?: boolean;
    sha?: string;
    baseRev?: string;
  }

  interface Checkpoint {
    change: string;
    task: string;
    agent: string;
    gitRef: string;
    createdAt: string;
    irreversible: string[];
  }

  let worktrees: Worktree[] = $state([]);
  let loading = $state(true);
  let picked: { change: string; task: string } | null = $state(null);
  let baseRev = $state("");
  let competitors: CompetitorDiff[] = $state([]);
  let checkpoints: Checkpoint[] = $state([]);
  /** The agents typed into the empty state's assign path. */
  let assignAgents = $state("");

  /** The race action being composed, if any: always the contract's own typed
      form, prefilled from the lane it was opened on. */
  let action: {
    method: string;
    titleKey: MessageKey;
    lane: string;
    values: Record<string, unknown>;
    raw: string;
    mode: "form" | "raw";
  } | null = $state(null);
  /** Raised for a method the registry marks destructive; nothing is sent while
      it stands, and cancelling it sends nothing at all. */
  let confirming = $state(false);
  let running = $state(false);

  const dangerous = $derived(
    action !== null &&
      (REGISTRY.find((entry) => entry.method === action?.method)?.dangerous ?? false),
  );

  /**
   * The sessions this board started. A dispatch answers only when the whole
   * turn is over, but its events reach THIS connection while it runs (the
   * daemon seals a session's stream to the connection that opened it), so the
   * board learns the session id from the stream rather than from the answer.
   */
  let ownTurns = $state(new Set<string>());

  /**
   * Follow the turns this board can follow. A turn that ends on a lane of the
   * open task re-reads the board, so the lane shows its new diff and commit
   * without the app reloading. A dispatch launched from another surface is
   * sealed to that surface's connection and never arrives here — hence the
   * refresh control and the note beside it, rather than a board that quietly
   * goes stale (design D5).
   */
  $effect(() =>
    onSessionEvent((message) => {
      if (!picked) return;
      const mine =
        ownTurns.has(message.sessionId) ||
        competitors.some((lane) => lane.sessionId === message.sessionId);
      if (!mine) {
        // A session that starts while this board is dispatching is the turn it
        // just asked for: adopt it, so its end is one this board reacts to.
        if (message.event.type === "session_started" && running) {
          ownTurns = new Set(ownTurns).add(message.sessionId);
        }
        return;
      }
      if (message.event.type === "session_ended") {
        void refreshBoard(picked.change, picked.task);
      }
    }),
  );

  const groups = $derived.by(() => {
    const map = new Map<string, { change: string; task: string; agents: string[] }>();
    for (const worktree of worktrees) {
      const key = `${worktree.change}/${worktree.task}`;
      const entry = map.get(key) ?? {
        change: worktree.change,
        task: worktree.task,
        agents: [],
      };
      entry.agents.push(worktree.agent);
      map.set(key, entry);
    }
    return [...map.values()];
  });

  $effect(() => {
    void request<{ worktrees: Worktree[] }>("worktree/list", {
      projectRoot: root,
    })
      .then((result) => (worktrees = result.worktrees))
      .catch(() => (worktrees = []))
      .finally(() => (loading = false));
  });

  async function pick(change: string, task: string) {
    picked = { change, task };
    competitors = [];
    checkpoints = [];
    await refreshBoard(change, task);
  }

  /** Re-reads the lanes of a task and the checkpoints of its change. */
  async function refreshBoard(change: string, task: string) {
    try {
      const result = await request<{
        baseRev: string;
        competitors: CompetitorDiff[];
      }>("worktree/diff", { projectRoot: root, change, task });
      baseRev = result.baseRev;
      competitors = result.competitors;
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    }
    try {
      const list = await request<{ checkpoints: Checkpoint[] }>("checkpoint/list", {
        projectRoot: root,
        change,
      });
      checkpoints = list.checkpoints;
    } catch {
      // A change with no checkpoints answers empty; a refusal leaves the mark
      // unknown rather than claiming there is none.
      checkpoints = [];
    }
  }

  /** The checkpoint of a lane, when the daemon recorded one. */
  function checkpointOf(agent: string): Checkpoint | null {
    if (!picked) return null;
    return (
      checkpoints.find(
        (c) => c.task === picked?.task && c.agent === agent && c.change === picked?.change,
      ) ?? null
    );
  }

  /** The live session of a lane, when its dispatch is still running. */
  function liveTurn(lane: CompetitorDiff): SessionInfo | null {
    if (!lane.sessionId) return null;
    const session = $allSessions.find((s) => s.sessionId === lane.sessionId);
    if (!session) return null;
    return session.state === "ended" || session.state === "interrupted" ? null : session;
  }

  /** Turn state as signal + word: running, finished, or never dispatched. */
  function turnState(lane: CompetitorDiff): { glyph: string; word: string; tone: string } {
    if (liveTurn(lane)) {
      return { glyph: "▸", word: $t("race.turn.running"), tone: "info" };
    }
    if (lane.sessionId) {
      return { glyph: "■", word: $t("race.turn.finished"), tone: "muted" };
    }
    return { glyph: "○", word: $t("race.turn.never"), tone: "muted" };
  }

  function short(rev: string | undefined): string {
    return rev ? rev.slice(0, 10) : "";
  }

  async function openExternally(worktreePath: string, file: string, line: number | null) {
    const editor = await openWith(worktreePath, file, line ?? undefined);
    pushNotice($t("editor.openedWith", { editor }), "info");
  }

  /**
   * Opens a race action on a lane. The parameters are the contract's, rendered
   * by the generated typed form; `confirm` is left FALSE where the contract has
   * one, so the daemon's own guard is a decision the user takes rather than a
   * default this surface quietly ticked for them.
   */
  function openAction(method: string, titleKey: MessageKey, lane: string, values: Record<string, unknown>) {
    action = {
      method,
      titleKey,
      lane,
      values,
      raw: JSON.stringify(values, null, 2),
      mode: "form",
    };
    confirming = false;
  }

  /** The send button: a destructive method raises the dialog first. */
  function submit() {
    if (!action || running) return;
    if (dangerous) {
      confirming = true;
      return;
    }
    void perform();
  }

  /** Actually sends the composed action, then re-reads the board. */
  async function perform() {
    if (!action || !picked) return;
    confirming = false;
    running = true;
    const { method } = action;
    try {
      const params = action.raw.trim() ? JSON.parse(action.raw) : {};
      await request(method, params);
      pushNotice($t("race.action.done", { method }), "info");
      action = null;
      await refreshBoard(picked.change, picked.task);
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      const detail = e?.detail ? `: ${e.detail}` : "";
      const remedy = e?.remedy ? ` — ${e.remedy}` : "";
      pushNotice(`${e?.message ?? String(raw)}${detail}${remedy}`, "danger");
    } finally {
      running = false;
    }
  }

  /** The lanes a file could be applied INTO: every other lane of the race. */
  function otherLanes(agent: string): CompetitorDiff[] {
    return competitors.filter((c) => c.agent !== agent);
  }

  /** The empty state's path out: assign this task to the agents just typed. */
  async function assignRace() {
    if (!picked) return;
    const agents = assignAgents
      .split(",")
      .map((a) => a.trim())
      .filter(Boolean);
    if (agents.length === 0) return;
    try {
      await request("worktree/assign", {
        projectRoot: root,
        tasks: [{ change: picked.change, task: picked.task, agents }],
      });
      assignAgents = "";
      const listed = await request<{ worktrees: Worktree[] }>("worktree/list", {
        projectRoot: root,
      });
      worktrees = listed.worktrees;
      await refreshBoard(picked.change, picked.task);
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    }
  }
</script>

<section aria-label={$t("review.title")}>
  <header>
    <h2>{$t("review.title")}</h2>
    <button onclick={onBack}>{$t("common.back")}</button>
  </header>

  {#if loading}
    <p class="muted">{$t("common.loading")}</p>
  {:else if worktrees.length === 0}
    <EmptyState
      glyph="diff"
      title={$t("review.empty.title")}
      hint={$t("review.empty.hint")}
    />
  {:else}
    <div class="picker">
      {#each groups as group (group.change + group.task)}
        <button
          class:active={picked?.change === group.change &&
            picked?.task === group.task}
          onclick={() => void pick(group.change, group.task)}
        >
          <code>{group.change}</code> · {group.task} — {group.agents.join(", ")}
        </button>
      {/each}
    </div>

    {#if picked && competitors.length === 0}
      <!-- A picked task with no lanes: say so and offer the way in, never a
           blank board (gui-shell "Carrera sin competidores"). -->
      <EmptyState
        glyph="diff"
        title={$t("race.empty.title")}
        hint={$t("race.empty.hint")}
      >
        <input
          type="text"
          bind:value={assignAgents}
          placeholder={$t("race.empty.agents")}
          aria-label={$t("race.empty.agents")}
        />
        <button class="primary" onclick={() => void assignRace()}>
          {$t("race.empty.assign")}
        </button>
      </EmptyState>
    {:else if picked}
      <div class="boardHead">
        <p class="muted">
          {$t("review.base")}: <code>{short(baseRev)}</code>
        </p>
        <!-- The board follows its own turns live. A dispatch started from
             another surface is sealed to that surface's stream, so this says
             so and offers the gesture, instead of pretending to be current. -->
        <p class="muted note">{$t("race.live.note")}</p>
        <button
          class="ghost"
          onclick={() => picked && void refreshBoard(picked.change, picked.task)}
        >
          {$t("race.live.refresh")}
        </button>
      </div>

      <div class="lanes" aria-label={$t("race.lanes")}>
        {#each competitors as lane (lane.agent)}
          <article class="lane">
            <header class="laneHead">
              <h3>{lane.agent}</h3>
              <!-- Provenance, as the daemon reported it. `race.unknown` is the
                   only thing that ever stands in for a fact it did not. -->
              <span class="prov">
                <span class="tag">
                  {$t("race.source")}:
                  {lane.source ? $t(("race.source." + lane.source) as never) : $t("race.unknown")}
                </span>
                <span class="tag">
                  {$t("race.profile")}: {lane.profile ?? $t("race.unknown")}
                </span>
                <span class="tag">
                  {$t("race.level")}: {lane.level ? "L" + lane.level : $t("race.unknown")}
                </span>
              </span>
            </header>

            <!-- State: signal plus word, never colour alone. -->
            <ul class="state">
              {#key lane.sessionId}
                <li class="tone-{turnState(lane).tone}">
                  <span aria-hidden="true">{turnState(lane).glyph}</span>
                  {turnState(lane).word}
                </li>
              {/key}
              <li class={lane.committed ? "tone-ok" : "tone-muted"}>
                <span aria-hidden="true">{lane.committed ? "◆" : "◇"}</span>
                {#if lane.committed}
                  {$t("race.commit.done")} <code>{short(lane.sha)}</code>
                {:else if lane.committed === false}
                  {$t("race.commit.none")}
                {:else}
                  {$t("race.unknown")}
                {/if}
              </li>
              <li class={checkpointOf(lane.agent) ? "tone-ok" : "tone-muted"}>
                <span aria-hidden="true">{checkpointOf(lane.agent) ? "⌖" : "○"}</span>
                {checkpointOf(lane.agent)
                  ? $t("race.checkpoint.have")
                  : $t("race.checkpoint.none")}
              </li>
              <li class="tone-muted">
                <span aria-hidden="true">≡</span>
                {lane.changedFiles.length}
                {$t("review.files")} · {$t("review.base")}
                <code>{short(lane.baseRev ?? baseRev)}</code>
              </li>
            </ul>

            <!-- The race's own actions, on the lane they act on. -->
            <div class="laneActions">
              <button
                onclick={() =>
                  picked &&
                  openAction("worktree/dispatch", "race.action.dispatch", lane.agent, {
                    projectRoot: root,
                    change: picked.change,
                    task: picked.task,
                    agent: lane.agent,
                  })}
              >
                {$t("race.action.dispatch")}
              </button>
              <button
                onclick={() =>
                  picked &&
                  openAction("checkpoint/revert", "race.action.revert", lane.agent, {
                    projectRoot: root,
                    change: picked.change,
                    task: picked.task,
                    agent: lane.agent,
                    confirm: false,
                  })}
              >
                {$t("race.action.revert")}
              </button>
              <button
                onclick={() =>
                  picked &&
                  openAction("commit/task", "race.action.commit", lane.agent, {
                    projectRoot: root,
                    change: picked.change,
                    task: picked.task,
                    agent: lane.agent,
                    title: "",
                    confirm: false,
                  })}
              >
                {$t("race.action.commit")}
              </button>
            </div>

            <div class="laneDiff">
              {#if lane.diff.trim() === ""}
                <p class="muted">{$t("review.nodiff")}</p>
              {:else}
                {#each fileSections(lane.diff) as section (section.file)}
                  <div class="file">
                    <div class="fileHead">
                      <code>{section.file}</code>
                      <span class="fileActions">
                        <button
                          onclick={() =>
                            picked &&
                            onEditWorktree(
                              lane.path,
                              {
                                change: picked.change,
                                task: picked.task,
                                agent: lane.agent,
                              },
                              section.file,
                            )}
                        >
                          {$t("review.edit")}
                        </button>
                        <button
                          onclick={() => void openExternally(lane.path, section.file, null)}
                        >
                          {$t("review.openFile")}
                        </button>
                        <!-- Assisted merge stays a per-file human decision:
                             take THIS lane's version of the file into another
                             lane, one file at a time. -->
                        {#each otherLanes(lane.agent) as target (target.agent)}
                          <button
                            onclick={() =>
                              openAction("worktree/merge-file", "race.action.merge", lane.agent, {
                                projectRoot: root,
                                target: target.path,
                                source: lane.path,
                                file: section.file,
                                confirm: false,
                              })}
                          >
                            {$t("race.action.mergeInto", { agent: target.agent })}
                          </button>
                        {/each}
                      </span>
                    </div>
                    {#each hunksOf(section) as hunk, h (h)}
                      <div class="hunk">
                        <div class="hunkHead">
                          <code class="hunkRange">{hunk.header || section.file}</code>
                          <span class="hunkActions">
                            <button
                              onclick={() =>
                                picked &&
                                onEditWorktree(
                                  lane.path,
                                  {
                                    change: picked.change,
                                    task: picked.task,
                                    agent: lane.agent,
                                  },
                                  section.file,
                                  hunk.startLine ?? undefined,
                                )}
                            >
                              {$t("review.editHunk")}
                            </button>
                            <button
                              onclick={() =>
                                void openExternally(lane.path, section.file, hunk.startLine)}
                            >
                              {$t("editor.openWith")}
                            </button>
                          </span>
                        </div>
                        <table class="diff">
                          <tbody>
                            {#each hunk.lines as line, i (i)}
                              <tr class="line {line.kind}">
                                <td class="gutter">
                                  {#if line.newLine !== null}
                                    <button
                                      class="lineNo"
                                      title={$t("review.openLine", {
                                        line: String(line.newLine),
                                      })}
                                      aria-label={$t("review.openLine", {
                                        line: String(line.newLine),
                                      })}
                                      onclick={() =>
                                        void openExternally(
                                          lane.path,
                                          section.file,
                                          line.newLine,
                                        )}>{line.newLine}</button
                                    >
                                  {:else}
                                    <span aria-hidden="true">·</span>
                                  {/if}
                                </td>
                                <td class="code"><pre>{line.text}</pre></td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                    {/each}
                  </div>
                {/each}
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {/if}
  {/if}

  {#if action}
    <!-- The composed action, in the contract's own typed form. -->
    <div class="actionPanel" aria-label={$t(action.titleKey)}>
      <div class="actionHead">
        <strong>{$t(action.titleKey)}</strong>
        <code>{action.lane}</code>
        <span class="muted">{action.method}</span>
      </div>
      <MethodForm
        method={action.method}
        values={action.values}
        bind:raw={action.raw}
        bind:mode={action.mode}
      />
      <div class="actionButtons">
        <button class="ghost" onclick={() => (action = null)}>{$t("common.cancel")}</button>
        <button class="primary" disabled={running} onclick={submit}>
          {running ? $t("common.loading") : $t("race.action.send")}
        </button>
      </div>
    </div>
  {/if}
</section>

{#if confirming && action}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("race.confirm.message", { action: $t(action.titleKey), lane: action.lane })}
    confirmLabel={$t(action.titleKey)}
    onConfirm={() => void perform()}
    onCancel={() => (confirming = false)}
  />
{/if}

<style>
  .boardHead {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    flex-wrap: wrap;
  }
  .boardHead .note {
    flex: 1;
    min-width: 12rem;
    font-size: var(--fs-dense);
  }
  .lanes {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(360px, 1fr);
    gap: var(--sp-3);
    align-items: start;
    overflow-x: auto;
  }
  .lane {
    display: grid;
    gap: var(--sp-2);
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    background: var(--surface);
    overflow: hidden;
  }
  .laneHead {
    display: grid;
    gap: var(--sp-1);
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .laneHead h3 {
    margin: 0;
    font-size: var(--fs-dense);
    font-weight: 600;
  }
  .prov {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1);
  }
  .tag {
    padding: 1px var(--sp-2);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text-muted);
    font-size: var(--fs-caption);
    white-space: nowrap;
  }
  ul.state {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1) var(--sp-3);
    margin: 0;
    padding: 0 var(--sp-3);
    list-style: none;
    font-size: var(--fs-caption);
  }
  ul.state li {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
  }
  .laneActions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1);
    padding: 0 var(--sp-3);
  }
  .actionPanel {
    display: grid;
    gap: var(--sp-2);
    padding: var(--sp-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-panel);
    background: var(--surface);
  }
  .actionHead {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    flex-wrap: wrap;
    font-size: var(--fs-dense);
  }
  .actionButtons {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
  .tone-ok {
    color: var(--ok);
  }
  .tone-info {
    color: var(--info);
  }
  .tone-muted {
    color: var(--text-muted);
  }
  .laneDiff {
    max-height: 55vh;
    overflow: auto;
    border-top: 1px solid var(--hair);
  }
  .file {
    border-bottom: 1px solid var(--border);
  }
  .hunk {
    border-top: 1px solid var(--hair);
  }
  .hunkHead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: 2px var(--cell-pad);
    background: var(--panel);
  }
  .hunkRange {
    font-size: var(--fs-caption);
    color: var(--text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hunkActions {
    display: flex;
    gap: var(--sp-1);
    flex: none;
  }
  table.diff {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  table.diff td {
    padding: 0;
    vertical-align: top;
  }
  table.diff .gutter {
    width: 5ch;
    text-align: right;
    padding-right: var(--sp-2);
    color: var(--text-faint);
    user-select: none;
  }
  .lineNo {
    font: inherit;
    color: inherit;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }
  .lineNo:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  table.diff .code pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }
  section {
    display: grid;
    gap: var(--sp-3);
    align-content: start;
    min-height: 0;
    padding: var(--panel-pad);
    overflow: auto;
    height: 100%;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-section);
    font-weight: 500;
  }

  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    align-items: stretch;
  }
  .picker button {
    text-align: left;
    background: var(--surface);
  }
  .picker button.active {
    border-color: var(--accent);
  }
  .fileHead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .fileActions {
    display: flex;
    gap: var(--sp-2);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
  pre {
    margin: 0;
    padding: var(--sp-2) 0;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 0.8125rem;
    line-height: 1.5;
  }
  .line {
    display: block;
    padding: 0 var(--sp-3);
    white-space: pre;
  }
  .line.add {
    background: rgb(21 128 61 / 0.12);
    color: var(--ok);
  }
  .line.del {
    background: rgb(185 28 28 / 0.12);
    color: var(--danger);
  }
  .line.hunk {
    color: var(--info);
  }
  .line.meta {
    color: var(--text-muted);
  }
  .muted {
    color: var(--text-muted);
  }
</style>
