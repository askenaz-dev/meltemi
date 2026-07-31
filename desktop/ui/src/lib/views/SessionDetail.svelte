<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { conn, request } from "../daemon";
  import { locale, t } from "../i18n";
  import { absoluteTime, clockTime, durationLabel } from "../time";
  import { binaryName } from "../agents";
  import {
    onSessionEvent,
    pending,
    pushNotice,
    refreshPending,
    refreshSessions,
    sessions,
  } from "../stores";
  import { agentLabelOf } from "../tree";
  import { fold } from "../conversation";
  import Avatar from "../components/Avatar.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Icon from "../components/Icon.svelte";
  import StatusBadge from "../components/StatusBadge.svelte";

  let {
    sessionId,
    onBack,
    onOpenSession,
  }: {
    sessionId: string;
    onBack: () => void;
    /** Resuming makes a NEW session; the conversation follows it there. */
    onOpenSession: (sessionId: string) => void;
  } = $props();

  interface Line {
    id: number;
    ts: string;
    kind: string;
    text: string;
    /** The full payload, revealed on expand. */
    full: string;
    /** The event's payload as received, which the fold reads (design D4). */
    payload?: unknown;
    cut?: boolean;
  }

  /** Glyph + tone per known event type; unknown types stay neutral. */
  const EVENT_STYLE: Record<string, { glyph: string; tone: string }> = {
    session_started: { glyph: "◌", tone: "info" },
    prompt_sent: { glyph: "▸", tone: "accent" },
    agent_update: { glyph: "·", tone: "text" },
    refs_expanded: { glyph: "·", tone: "faint" },
    permission_requested: { glyph: "●", tone: "warn" },
    permission_decided: { glyph: "●", tone: "warn" },
    mcp_injected: { glyph: "·", tone: "faint" },
    mcp_not_delivered: { glyph: "▲", tone: "warn" },
    turn_completed: { glyph: "■", tone: "ok" },
    checkpoint_created: { glyph: "·", tone: "info" },
    checkpoint_restored: { glyph: "·", tone: "info" },
    agent_resolved: { glyph: "·", tone: "faint" },
    task_started: { glyph: "▸", tone: "accent" },
    task_committed: { glyph: "■", tone: "ok" },
    instruction_queued: { glyph: "▸", tone: "info" },
    usage_reported: { glyph: "∑", tone: "info" },
    human_edit: { glyph: "✎", tone: "info" },
    session_cancelled: { glyph: "▲", tone: "danger" },
    session_ended: { glyph: "■", tone: "faint" },
    error: { glyph: "▲", tone: "danger" },
  };

  let lines: Line[] = $state([]);
  /**
   * Which reading is on screen. The log is the truth and the conversation is a
   * view of it (design D4), so the switch is always here and neither reading
   * ever drops an event.
   */
  let reading: "conversation" | "log" = $state("conversation");
  let showTimestamps = $state(true);
  let expanded: Record<number, boolean> = $state({});
  let search = $state("");
  let searchOpen = $state(false);
  /** Index into the ordered match list, for next/previous navigation. */
  let matchAt = $state(0);
  let confirmCancel = $state(false);
  let gone = $state(false);
  let atBottom = $state(true);
  let newLines = $state(0);
  let scroller: HTMLDivElement | undefined = $state();
  let seq = 0;
  let wasStreaming = false;
  let wasUnreachable = false;

  const session = $derived($sessions.find((s) => s.sessionId === sessionId));

  /**
   * The events actually received. The "cut" marker the surface injects when the
   * daemon drops is NOT one of them, so it is excluded from both the fold and
   * the count the invariant is about.
   */
  const events = $derived(lines.filter((line) => !line.cut));

  /** The conversational reading of exactly the events the log holds. */
  const conversation = $derived(
    fold(
      events.map((line) => ({
        id: line.id,
        ts: line.ts,
        type: line.kind,
        payload: line.payload,
      })),
    ),
  );

  // Walked in on from the composer, this session is younger than the last
  // `session/list` the shell asked for, so the header would render an id and
  // nothing else. Ask once — a plain variable, not state, so a session that
  // genuinely is not there does not turn into a refresh loop.
  let askedFor: string | null = null;
  $effect(() => {
    if (session || askedFor === sessionId) return;
    askedFor = sessionId;
    void refreshSessions().catch(() => {});
  });
  /** The matching line ids in transcript order: a Set cannot be navigated. */
  const matchList = $derived.by<number[]>(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return [];
    return lines
      .filter(
        (line) =>
          line.kind.toLowerCase().includes(needle) ||
          line.full.toLowerCase().includes(needle),
      )
      .map((line) => line.id);
  });

  $effect(() => {
    void search;
    matchAt = 0;
  });

  const matches = $derived.by(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return new Set<number>();
    return new Set(
      lines
        .filter(
          (line) =>
            line.kind.toLowerCase().includes(needle) ||
            line.full.toLowerCase().includes(needle),
        )
        .map((line) => line.id),
    );
  });

  function payloadText(payload: unknown): { short: string; full: string } {
    if (payload === undefined || payload === null) return { short: "", full: "" };
    if (typeof payload !== "object") {
      const value = String(payload);
      return { short: value, full: value };
    }
    const record = payload as Record<string, unknown>;
    for (const key of ["text", "message", "instruction", "detail", "title", "file", "reason"]) {
      const value = record[key];
      if (typeof value === "string" && value.trim()) {
        const flat = value.replaceAll(/\s+/g, " ").trim();
        return {
          short: flat.length > 140 ? `${flat.slice(0, 140)}…` : flat,
          full: value,
        };
      }
    }
    const json = JSON.stringify(payload);
    return { short: json.length > 140 ? `${json.slice(0, 140)}…` : json, full: JSON.stringify(payload, null, 2) };
  }

  function append(line: Omit<Line, "id">) {
    seq += 1;
    lines = [...lines, { ...line, id: seq }];
    if (atBottom) {
      queueMicrotask(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
    } else {
      newLines += 1;
    }
  }

  // The persisted log seeds the transcript.
  $effect(() => {
    const info = session;
    if (!info) return;
    void request<{ lines: string[] }>("session/log", {
      projectRoot: info.projectRoot,
      sessionId,
    })
      .then((result) => {
        seq = 0;
        lines = result.lines.map((raw, index) => {
          try {
            const event = JSON.parse(raw) as { ts?: string; type?: string; payload?: unknown };
            const { short, full } = payloadText(event.payload);
            return {
              id: index + 1,
              ts: event.ts ?? "",
              kind: event.type ?? "event",
              text: short,
              full: full || raw,
              payload: event.payload,
            };
          } catch {
            return { id: index + 1, ts: "", kind: "raw", text: raw, full: raw };
          }
        });
        seq = lines.length;
        queueMicrotask(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
      })
      .catch(() => {
        // A session mid-start may have no log yet; live events fill it in.
      });
  });

  // Ask the daemon for this session's live stream. The connection that
  // started the session already receives it; for any other — a second window,
  // a tunnelled phone — this is what turns the detail view live instead of a
  // snapshot (eventos-para-tardios).
  $effect(() => {
    const watched = sessionId;
    void request("session/watch", { sessionId: watched, watch: true }).catch(
      () => {
        // A daemon that cannot be reached will be retried by the shell; the
        // log already fetched above remains the honest fallback.
      },
    );
    return () => {
      void request("session/watch", {
        sessionId: watched,
        watch: false,
      }).catch(() => {});
    };
  });

  $effect(() =>
    onSessionEvent((message) => {
      if (message.sessionId !== sessionId) return;
      wasStreaming = true;
      const payload = (message.event as { payload?: unknown }).payload;
      const { short, full } = payloadText(payload);
      append({
        ts: new Date().toISOString(),
        kind: message.event.type,
        text: short,
        full,
        payload,
      });
    }),
  );

  // Honest cut on daemon loss, honest "gone" after reconnection.
  $effect(() => {
    const state = $conn.state;
    if (state === "unreachable" && !wasUnreachable) {
      wasUnreachable = true;
      if (wasStreaming) {
        append({ ts: new Date().toISOString(), kind: "cut", text: $t("sessions.detail.cut"), full: "", cut: true });
      }
    } else if (state === "connected" && wasUnreachable) {
      wasUnreachable = false;
      void refreshSessions().then(() => {
        if (!session) gone = true;
      });
    }
  });

  function onScroll() {
    if (!scroller) return;
    const nearBottom =
      scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 8;
    atBottom = nearBottom;
    if (nearBottom) newLines = 0;
  }

  /**
   * Switches reading without losing where the user was. Following the tail
   * keeps following it; a reader parked in the middle comes back to the same
   * offset rather than to the top, because a switch is a change of lens and not
   * a reload.
   */
  async function switchReading() {
    const wasAtBottom = atBottom;
    const offset = scroller?.scrollTop ?? 0;
    reading = reading === "log" ? "conversation" : "log";
    await tick();
    if (!scroller) return;
    if (wasAtBottom) scroller.scrollTo({ top: scroller.scrollHeight });
    else scroller.scrollTop = offset;
  }

  function jumpDown() {
    scroller?.scrollTo({ top: scroller.scrollHeight });
    atBottom = true;
    newLines = 0;
  }

  async function copyAll() {
    const text = lines
      .map((line) => `${line.ts} ${line.kind} ${line.full || line.text}`)
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
      pushNotice($t("transcript.copiedAll"), "info");
    } catch {
      pushNotice($t("banner.copyFailed"), "danger");
    }
  }

  async function copyLine(line: Line) {
    try {
      await navigator.clipboard.writeText(`${line.ts} ${line.kind} ${line.full || line.text}`);
      pushNotice($t("transcript.copiedLine"), "info");
    } catch {
      pushNotice($t("banner.copyFailed"), "danger");
    }
  }

  /** Moves to the next (or previous) match and brings it into view. */
  function stepMatch(delta: number) {
    if (matchList.length === 0) return;
    matchAt = (matchAt + delta + matchList.length) % matchList.length;
    focusMatch();
  }

  function focusMatch() {
    const id = matchList[matchAt];
    if (id === undefined) return;
    // Scrolls the transcript to the current hit only: no layout is animated and
    // nothing else on screen moves.
    document
      .querySelector(`[data-line="${id}"]`)
      ?.scrollIntoView({ block: "nearest", behavior: "auto" });
  }

  // ---- the persistent composer -------------------------------------------

  const LIVE = ["active", "starting", "waiting_permission"];

  let draft = $state("");
  let sending = $state(false);
  /** The daemon's own diagnosis when it refused to take direction. */
  let refused: { detail: string; remedy: string | null } | null = $state(null);
  /** What became of the last instruction, in the daemon's terms, never ours. */
  let outcome: { queuePosition: number } | null = $state(null);
  let field: HTMLTextAreaElement | undefined = $state();

  /** The permission this session is stopped on, when it is stopped on one. */
  const waitingOn = $derived($pending.find((item) => item.sessionId === sessionId));

  /**
   * Which card, if any, may still be acted on. The log's
   * `permission_requested` carries no request id — the proxy's queue owns
   * those — so the live card is the last undecided one WHILE the queue holds a
   * request for this session. Everything else renders resolved: a button that
   * decides nothing is a simulation of control.
   */
  const liveCard = $derived.by<number | null>(() => {
    if (!waitingOn || waitingOn.expired) return null;
    const open = conversation.filter(
      (item) => item.kind === "permission" && item.decided === null,
    );
    return open.length > 0 ? open[open.length - 1].id : null;
  });

  async function decide(optionId: string) {
    if (!waitingOn) return;
    try {
      // The very method the tray uses: the conversation is another view of the
      // same queue, never a second queue.
      await request("permission/decide", { requestId: waitingOn.requestId, optionId });
      pushNotice($t("permissions.decided"), "info");
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null };
      pushNotice(`${$t("common.error")}: ${e?.detail ?? e?.message ?? String(raw)}`, "danger");
    }
    await refreshPending().catch(() => {});
  }

  /**
   * Whether the surface may offer to send at all. A refusal the daemon already
   * gave outranks everything: a session that answered `session_not_directable`
   * will answer it again, and re-offering the button would be the surface
   * arguing with the daemon.
   */
  const canSend = $derived.by(() => {
    if (refused) return false;
    if (!session) return false;
    if (LIVE.includes(session.state)) return true;
    return session.resumable;
  });

  /** A terminated-but-resumable session is resumed, not sent to. */
  const resumes = $derived(
    session !== undefined && !LIVE.includes(session.state) && session.resumable,
  );

  // Arriving at a conversation puts the caret where the next instruction goes —
  // once per session, so a state change mid-read never steals the focus back.
  let focusedFor: string | null = null;
  $effect(() => {
    if (!field || focusedFor === sessionId) return;
    focusedFor = sessionId;
    field.focus();
  });

  async function direct() {
    const instruction = draft.trim();
    if (!instruction || sending || !canSend || !session) return;
    sending = true;
    outcome = null;
    // A resume starts a NEW session, and it runs a whole turn before the call
    // answers. Its `session_started` reaches this connection first (design D3),
    // so the conversation follows the resumed session instead of waiting.
    const stop = onSessionEvent((message) => {
      if (message.event.type !== "session_started") return;
      stop();
      draft = "";
      sending = false;
      void refreshSessions().catch(() => {});
      onOpenSession(message.sessionId);
    });
    try {
      const result = await request<{
        disposition: string;
        sessionId: string;
        queuePosition?: number;
      }>("session/direct", {
        projectRoot: session.projectRoot,
        sessionId,
        instruction,
      });
      draft = "";
      if (result.disposition === "queued") {
        // Queued is NOT attended: say so with its position and let the
        // transcript show it arrive.
        outcome = { queuePosition: result.queuePosition ?? 0 };
      }
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      const detail = e?.detail ?? e?.message ?? String(raw);
      refused = { detail, remedy: e?.remedy ?? null };
    } finally {
      stop();
      sending = false;
      void refreshSessions().catch(() => {});
    }
  }

  function onDraftKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void direct();
    }
  }

  function cancelSession() {
    confirmCancel = false;
    void invoke("daemon_notify", { method: "session/cancel", params: { sessionId } });
    setTimeout(() => void refreshSessions().catch(() => {}), 500);
  }
</script>

<section aria-label={$t("sessions.detail.transcript")}>
  <header>
    <div class="who">
      {#if session}
        <Avatar id={agentLabelOf(session)} name={agentLabelOf(session)} size={26} />
        <div class="ids">
          <code>{sessionId.slice(0, 8)}</code>
          <span class="faint">
            {agentLabelOf(session)} · <span class="mono">{binaryName(session.agentCommand)}</span>
          </span>
        </div>
        {#if session.profile}
          <span class="pill" title={$t("sessions.subscription")}>{session.profile}</span>
        {/if}
        <StatusBadge state={session.state} />
        <span class="faint">{durationLabel(session.startedAt, session.endedAt)}</span>
        <span
          class="faint"
          title={absoluteTime(session.startedAt, $locale)}>{clockTime(session.startedAt)}</span
        >
      {:else}
        <code>{sessionId.slice(0, 8)}</code>
      {/if}
    </div>

    <div class="tools">
      {#if searchOpen}
        <input
          bind:value={search}
          placeholder={$t("transcript.search")}
          aria-label={$t("transcript.search")}
          onkeydown={(event) => {
            event.stopPropagation();
            if (event.key === "Escape") {
              search = "";
              searchOpen = false;
            }
            if (event.key === "Enter") {
              event.preventDefault();
              stepMatch(event.shiftKey ? -1 : 1);
            }
          }}
        />
        <span class="faint count" aria-live="polite">
          {matchList.length === 0
            ? matches.size
            : $t("transcript.match", {
                n: String(matchAt + 1),
                total: String(matchList.length),
              })}
        </span>
        <button
          class="ghost"
          aria-label={$t("transcript.prev")}
          disabled={matchList.length === 0}
          onclick={() => stepMatch(-1)}>↑</button
        >
        <button
          class="ghost"
          aria-label={$t("transcript.next")}
          disabled={matchList.length === 0}
          onclick={() => stepMatch(1)}>↓</button
        >
      {:else}
        <button class="ghost" aria-label={$t("transcript.search")} onclick={() => (searchOpen = true)}>
          <Icon name="search" size={14} />
        </button>
      {/if}
      <button
        class="ghost"
        aria-pressed={reading === "log"}
        title={$t("conv.reading.hint")}
        onclick={() => void switchReading()}
      >
        {reading === "log" ? $t("conv.reading.log") : $t("conv.reading.conversation")}
      </button>
      <!-- The same number in both readings, on screen: the invariant is not a
           claim in a design note, it is something the user can count. -->
      <span class="faint count">{$t("conv.events", { n: String(events.length) })}</span>
      <button
        class="ghost"
        aria-pressed={showTimestamps}
        onclick={() => (showTimestamps = !showTimestamps)}
      >
        {$t("transcript.timestamps")}
      </button>
      <button class="ghost" onclick={() => void copyAll()}>
        <Icon name="copy" size={14} />
        {$t("transcript.copyAll")}
      </button>
      {#if session && LIVE.includes(session.state)}
        <button class="ghost destructive" onclick={() => (confirmCancel = true)}>
          {$t("sessions.cancelShort")}
        </button>
      {/if}
      <button class="ghost" onclick={onBack}>{$t("common.back")}</button>
    </div>
  </header>

  {#if gone}
    <p class="danger" role="alert">{$t("sessions.detail.goneAfterReconnect")}</p>
  {/if}

  <div
    class="transcript"
    class:log={reading === "log"}
    bind:this={scroller}
    onscroll={onScroll}
    role="log"
    aria-live="polite"
  >
    {#if reading === "conversation"}
      {#each conversation as item (item.id)}
        {#if item.kind === "human"}
          <div class="turn human">
            <span class="who">{$t("conv.you")}</span>
            <div class="bubble" class:pending={item.pending}>
              <p class="prose">{item.text}</p>
              {#if item.pending}
                <span class="pill info">{$t("conv.pendingTurn")}</span>
              {/if}
            </div>
          </div>
        {:else if item.kind === "agent"}
          <div class="turn agent">
            <span class="who">{session ? agentLabelOf(session) : $t("conv.agent")}</span>
            <div class="bubble">
              {#each item.parts as part, index (index)}
                {#if part.kind === "text"}
                  <p class="prose">{part.text}</p>
                {:else if part.kind === "thought"}
                  <details class="thought">
                    <summary>{$t("conv.thought")}</summary>
                    <p class="prose">{part.text}</p>
                  </details>
                {:else if part.kind === "tool"}
                  <span class="pill tool">
                    {part.title || part.toolCallId}{part.status ? ` · ${part.status}` : ""}
                  </span>
                {:else}
                  <ul class="plan">
                    {#each part.entries as entry, at (at)}
                      <li>{entry}</li>
                    {/each}
                  </ul>
                {/if}
              {/each}
              {#if item.parts.length === 0}
                <p class="prose faint">{$t("conv.noOutput")}</p>
              {/if}
            </div>
            {#if item.closed}
              <span class="stop">
                {$t("conv.turnEnded", { reason: item.stopReason ?? $t("conv.stop.unknown") })}
              </span>
            {/if}
          </div>
        {:else if item.kind === "permission"}
          {@const live = item.id === liveCard}
          <div class="card" class:resolved={item.decided !== null || !live}>
            <span class="cardHead">
              <span aria-hidden="true">●</span>
              {$t("permissions.title")}
            </span>
            <p class="prose">{item.title || (live && waitingOn ? waitingOn.summary : "")}</p>
            {#if item.decided}
              <p class="cardMeta">
                {$t("conv.decidedBy", { by: item.decided.by })}
                {#if item.decided.denied !== null}
                  · {item.decided.denied ? $t("conv.denied") : $t("conv.allowed")}
                {/if}
                {#if item.decided.rule}
                  · {item.decided.rule}
                {/if}
              </p>
            {:else if live && waitingOn}
              <div class="cardOptions">
                {#each waitingOn.options as option (option.optionId)}
                  <button onclick={() => void decide(option.optionId)}>{option.name}</button>
                {/each}
              </div>
              <p class="cardMeta">{$t("conv.alsoInTray")}</p>
            {:else}
              <!-- Decided elsewhere, expired, or denied for want of a client:
                   the card says so and offers nothing that no longer decides. -->
              <p class="cardMeta">{$t("conv.permissionStale")}</p>
            {/if}
          </div>
        {:else}
          {@const style = EVENT_STYLE[item.type] ?? { glyph: "·", tone: "faint" }}
          <div class="sysLine tone-{style.tone}">
            <span class="glyph" aria-hidden="true">{style.glyph}</span>
            <span class="kind">{item.type}</span>
            <span class="text">{item.text}</span>
          </div>
        {/if}
      {/each}
      {#if conversation.length === 0}
        <p class="faint empty">{$t("transcript.empty")}</p>
      {/if}
    {:else}
      {#each lines as line (line.id)}
      {@const style = EVENT_STYLE[line.kind] ?? { glyph: "·", tone: "faint" }}
      <div
        class="line tone-{style.tone}"
        data-line={line.id}
        class:cut={line.cut}
        class:hit={matches.has(line.id)}
        class:current={matchList[matchAt] === line.id}
      >
        {#if showTimestamps && line.ts}
          <span class="ts faint" title={absoluteTime(line.ts, $locale)}>{clockTime(line.ts)}</span>
        {/if}
        <span class="glyph" aria-hidden="true">{style.glyph}</span>
        <span class="kind">{line.kind}</span>
        <span class="text">
          {expanded[line.id] && line.full ? line.full : line.text}
        </span>
        {#if line.full && line.full !== line.text}
          <button
            class="link expand"
            onclick={() => (expanded[line.id] = !expanded[line.id])}
          >
            {expanded[line.id] ? $t("transcript.collapse") : $t("transcript.expand")}
          </button>
        {/if}
        <button
          class="link copyOne"
          aria-label={$t("transcript.copyLine")}
          onclick={() => void copyLine(line)}
        >
          <Icon name="copy" size={11} />
        </button>
      </div>
      {/each}
      {#if lines.length === 0}
        <p class="faint empty">{$t("transcript.empty")}</p>
      {/if}
    {/if}
  </div>

  <!-- The persistent composer: the conversation is where the next instruction
       is written, not a round trip through a modal. -->
  <div class="composer">
    {#if canSend}
      <textarea
        bind:this={field}
        bind:value={draft}
        onkeydown={onDraftKeydown}
        rows="2"
        spellcheck="false"
        aria-label={$t("conv.placeholder")}
        placeholder={$t("conv.placeholder")}
      ></textarea>
    {/if}

    <div class="composerRow">
      <span class="state" aria-live="polite">
        {#if refused}
          <span class="refusedText">{$t("conv.refused")} — {refused.detail}</span>
          {#if refused.remedy}
            <span class="remedy">{$t("common.remedy")}: {refused.remedy}</span>
          {/if}
        {:else if waitingOn}
          <span class="waiting">{$t("conv.waiting", { tool: waitingOn.tool })}</span>
        {:else if outcome}
          <span class="queued">{$t("conv.queued", { n: String(outcome.queuePosition) })}</span>
        {:else if resumes}
          <span class="faint">{$t("conv.resumeHint")}</span>
        {:else if !canSend}
          <span class="faint">{$t("conv.closed")}</span>
        {/if}
      </span>

      {#if canSend}
        <button
          class="primary"
          disabled={sending || draft.trim() === ""}
          title={$t("home.sendHint")}
          onclick={() => void direct()}
        >
          {sending
            ? $t("common.loading")
            : resumes
              ? $t("sessions.resume")
              : $t("home.send")}
        </button>
      {/if}
    </div>
  </div>

  {#if !atBottom && newLines > 0}
    <button class="follow" onclick={jumpDown}>↓ {newLines}</button>
  {/if}
</section>

{#if confirmCancel}
  <ConfirmDialog
    title={$t("confirm.title")}
    message={$t("sessions.cancel.warning")}
    confirmLabel={$t("sessions.cancel")}
    onConfirm={cancelSession}
    onCancel={() => (confirmCancel = false)}
  />
{/if}

<style>
  .line.current {
    outline: 1px solid var(--focus);
    outline-offset: -1px;
  }
  /* A column whose child count varies: the "gone" notice is conditional and a
     composer is about to join. A fixed row template cannot express that — with
     one conditional child absent, auto-placement drops the transcript into an
     `auto` track and the 1fr track stays empty, so the panel stops filling the
     window and starts overflowing it instead. This is the same defect the
     archived `gui-acabado-y-cierre-sdd` found in the shell column, and the same
     fix: every bar keeps its natural height, the transcript takes the rest. */
  section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    height: 100%;
    min-height: 0;
    padding: var(--sp-2) var(--sp-4) var(--sp-4);
    position: relative;
  }
  section > :not(.transcript) {
    flex: 0 0 auto;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }
  .who {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .ids {
    display: grid;
  }
  .ids .faint {
    font-size: var(--fs-caption);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    flex-wrap: wrap;
  }
  .tools button {
    font-size: var(--fs-caption);
    /* Deliberately tighter than the skin's --sp-2 (design D1). */
    gap: 4px;
  }
  .tools input {
    font-size: var(--fs-caption);
    width: 16ch;
  }
  .count {
    font-size: var(--fs-caption);
  }
  .transcript {
    flex: 1 1 0;
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
    padding: var(--sp-2) 0;
    font-size: var(--fs-dense);
    min-height: 0;
  }
  /* The operator log is monospaced because it is data; the conversation is
     prose and is read in the UI face. */
  .transcript.log {
    font-family: var(--font-mono);
  }
  .turn {
    display: grid;
    gap: 2px;
    padding: var(--sp-1) var(--sp-3);
  }
  .turn .who {
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  .bubble {
    display: grid;
    gap: var(--sp-1);
    justify-items: start;
    border-radius: var(--radius-panel);
    padding: var(--sp-2) var(--sp-3);
    background: var(--surface-2);
    max-width: 72ch;
  }
  .turn.human {
    justify-items: end;
  }
  .turn.human .bubble {
    background: var(--tint-info);
    color: var(--text);
  }
  /* A queued instruction has NOT been attended: it reads as an outline, and it
     says so in words beside the shape. */
  .bubble.pending {
    background: transparent;
    border: 1px dashed var(--border);
  }
  .prose {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .thought {
    width: 100%;
    color: var(--text-muted);
    font-size: var(--fs-caption);
  }
  .thought summary {
    cursor: pointer;
    color: var(--text-faint);
  }
  .pill.tool {
    font-family: var(--font-mono);
  }
  .plan {
    margin: 0;
    padding-left: var(--sp-4);
    color: var(--text-muted);
  }
  .stop {
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  .card {
    margin: var(--sp-1) var(--sp-3);
    display: grid;
    gap: var(--sp-1);
    border: 1px solid var(--warn);
    border-radius: var(--radius-panel);
    padding: var(--sp-2) var(--sp-3);
    /* Hard rule: a permission never animates its layout. */
    transition: none !important;
  }
  .card.resolved {
    border-color: var(--hair);
  }
  .cardHead {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    font-size: var(--fs-caption);
    color: var(--warn);
  }
  .card.resolved .cardHead {
    color: var(--text-faint);
  }
  .cardMeta {
    margin: 0;
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .cardOptions {
    display: flex;
    gap: var(--sp-2);
    flex-wrap: wrap;
    /* Hard rule: the options never move under the cursor. */
    transition: none !important;
  }
  .cardOptions button {
    font-size: var(--fs-dense);
  }
  .sysLine {
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    padding: 1px var(--sp-3);
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    color: var(--text-muted);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line {
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    padding: 1px var(--sp-3);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line:hover {
    background: var(--surface-2);
  }
  .line.hit {
    background: var(--tint-info);
  }
  .line.cut {
    color: var(--danger);
    font-style: italic;
  }
  .ts {
    flex: none;
  }
  .glyph {
    flex: none;
  }
  .kind {
    flex: none;
    color: var(--text-muted);
  }
  .text {
    flex: 1;
  }
  .expand,
  .copyOne {
    flex: none;
    font-size: var(--fs-caption);
    opacity: 0.7;
  }
  .tone-accent .glyph {
    color: var(--accent);
  }
  .tone-ok .glyph {
    color: var(--ok);
  }
  .tone-warn .glyph {
    color: var(--warn);
  }
  .tone-danger .glyph {
    color: var(--danger);
  }
  .tone-info .glyph {
    color: var(--info);
  }
  .tone-faint .glyph,
  .faint {
    color: var(--text-faint);
  }
  .danger {
    margin: 0;
    color: var(--danger);
  }
  .composer {
    display: grid;
    gap: var(--sp-1);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    padding: var(--sp-2);
  }
  .composer:focus-within {
    border-color: var(--accent);
  }
  .composer textarea {
    border: 0;
    background: transparent;
    resize: none;
    padding: var(--sp-1) var(--sp-2);
    font-family: var(--font-ui);
    font-size: var(--fs-body);
  }
  .composer textarea:focus-visible {
    outline: none;
  }
  .composerRow {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-height: 26px;
  }
  .state {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: var(--sp-2);
    flex-wrap: wrap;
    font-size: var(--fs-caption);
  }
  .queued {
    color: var(--info);
  }
  .waiting {
    color: var(--warn);
  }
  .refusedText {
    color: var(--danger);
  }
  .remedy {
    color: var(--info);
  }
  .composerRow .primary {
    font-size: var(--fs-dense);
  }
  .empty {
    margin: var(--sp-3);
  }
  .follow {
    position: absolute;
    right: var(--sp-6);
    bottom: var(--sp-6);
    border-color: var(--accent);
    color: var(--accent);
    background: var(--surface);
  }
</style>
