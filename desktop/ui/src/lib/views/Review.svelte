<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { request } from "../daemon";
  import { fileSections, hunksOf } from "../diff";
  import { openWith } from "../editor/files";
  import { pushNotice } from "../stores";
  import EmptyState from "../components/EmptyState.svelte";

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

  interface CompetitorDiff {
    agent: string;
    path: string;
    changedFiles: string[];
    diff: string;
  }

  let worktrees: Worktree[] = $state([]);
  let loading = $state(true);
  let picked: { change: string; task: string } | null = $state(null);
  let baseRev = $state("");
  let competitors: CompetitorDiff[] = $state([]);
  let activeAgent: string | null = $state(null);

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

  const activeDiff = $derived(
    competitors.find((c) => c.agent === activeAgent) ?? null,
  );

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
    try {
      const result = await request<{
        baseRev: string;
        competitors: CompetitorDiff[];
      }>("worktree/diff", { projectRoot: root, change, task });
      baseRev = result.baseRev;
      competitors = result.competitors;
      activeAgent = result.competitors[0]?.agent ?? null;
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    }
  }

  async function openExternally(worktreePath: string, file: string, line: number | null) {
    const editor = await openWith(worktreePath, file, line ?? undefined);
    pushNotice($t("editor.openedWith", { editor }), "info");
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

    {#if picked && competitors.length > 0}
      <p class="muted">
        {$t("review.base")}: <code>{baseRev.slice(0, 10)}</code>
      </p>
      <div class="tabs" role="tablist">
        {#each competitors as competitor (competitor.agent)}
          <button
            role="tab"
            aria-selected={competitor.agent === activeAgent}
            class:active={competitor.agent === activeAgent}
            onclick={() => (activeAgent = competitor.agent)}
          >
            {competitor.agent} ({competitor.changedFiles.length})
          </button>
        {/each}
      </div>

      {#if activeDiff}
        {#if activeDiff.diff.trim() === ""}
          <p class="muted">{$t("review.nodiff")}</p>
        {:else}
          {#each fileSections(activeDiff.diff) as section (section.file)}
            <article>
              <div class="fileHead">
                <code>{section.file}</code>
                <span class="fileActions">
                  <button
                    onclick={() =>
                      picked &&
                      onEditWorktree(
                        activeDiff.path,
                        {
                          change: picked.change,
                          task: picked.task,
                          agent: activeDiff.agent,
                        },
                        section.file,
                      )}
                  >
                    {$t("review.edit")}
                  </button>
                  <button
                    onclick={() =>
                      void openExternally(activeDiff.path, section.file, null)}
                  >
                    {$t("review.openFile")}
                  </button>
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
                            activeDiff.path,
                            {
                              change: picked.change,
                              task: picked.task,
                              agent: activeDiff.agent,
                            },
                            section.file,
                            hunk.startLine ?? undefined,
                          )}
                      >
                        {$t("review.editHunk")}
                      </button>
                      <button
                        onclick={() =>
                          void openExternally(
                            activeDiff.path,
                            section.file,
                            hunk.startLine,
                          )}
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
                                    activeDiff.path,
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
            </article>
          {/each}
        {/if}
      {/if}
    {/if}
  {/if}
</section>

<style>
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
  .picker button.active,
  .tabs button.active {
    border-color: var(--accent);
  }
  .tabs {
    display: flex;
    gap: var(--sp-2);
  }
  article {
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    background: var(--surface);
    overflow: hidden;
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
