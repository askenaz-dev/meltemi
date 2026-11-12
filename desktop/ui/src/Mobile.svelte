<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  // The mobile companion: the Agent Boss's remote post (companero-movil D1, D6;
  // companero-que-conecta D4, D7).
  //
  // Not the desktop shell made small. The sidebar, the tabs and the three zones
  // exist because there is a mouse and 1,280 pixels; what is reused is
  // everything below them — the stores, the catalogue, the decision modules —
  // which is almost everything that matters. What is the companion's own is
  // the pairing: the key to authorise, the destination to dial, and the
  // actor's life, which is this app's time in front.
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, untrack } from "svelte";
  import { get } from "svelte/store";
  import { conn, request, startConnListener } from "./lib/daemon";
  import { t } from "./lib/i18n";
  import Icon from "./lib/components/Icon.svelte";
  import { DESTINATIONS, type MobileView } from "./lib/mobile-nav";
  import { actions, pairing, worthSending } from "./lib/mobile-pair";
  import {
    activeProject,
    allSessions,
    pending,
    refreshPending,
    refreshSessions,
    startIncomingRouter,
    isLive,
  } from "./lib/stores";

  let view: MobileView = $state("sessions");

  /** This app's line for `authorized_keys`, or why there is none. */
  let keyLine: string | null = $state(null);
  let keyError: string | null = $state(null);
  /** The destination the host remembers, as typed; `null` means unpaired. */
  let remembered: string | null = $state(null);
  /** What the destination field holds. */
  let typed = $state("");
  /** How many times the actor said unreachable since the last connect. */
  let attempts = $state(0);
  /**
   * The user cancelled: coming back to the front does not redial until they
   * connect again on purpose. The destination stays remembered on disk.
   */
  let cancelled = $state(false);

  const connected = $derived($conn.state === "connected");
  /** The pairing screen's state, or null once connected. */
  const paired = $derived(pairing(remembered, $conn, attempts));

  /**
   * Reconciling after the app comes back, and never trusting what was on screen.
   *
   * Suspended is not connected (design D4): nothing holds the socket in the
   * background, so what was shown before is a memory of a different moment. The
   * state is asked for again before anything is presented as current.
   */
  async function reconcile() {
    await Promise.all([
      refreshSessions().catch(() => {}),
      refreshPending().catch(() => {}),
      readLanes().catch(() => {}),
    ]);
  }

  /**
   * Dials `destination`. The host parses it first and refuses, with the shape
   * it wanted, anything that is not `user@host[:port]`; what it accepts it
   * remembers, and from then on coming back to the front redials it.
   */
  async function connect(destination: string) {
    said = null;
    cancelled = false;
    try {
      await invoke("companion_connect", { destination });
      remembered = destination;
      typed = destination;
      attempts = 0;
    } catch (raw) {
      said = `${$t("common.error")}: ${String(raw)}`;
    }
  }

  /** Ends the actor; the destination stays on disk for a deliberate retry. */
  async function cancel() {
    cancelled = true;
    remembered = null;
    await invoke("companion_disconnect").catch(() => {});
  }

  /** Forgets the destination for good, and ends the actor with it. */
  async function forgetDestination() {
    said = null;
    try {
      await invoke("companion_forget_destination");
      remembered = null;
      typed = "";
      attempts = 0;
      said = $t("mobile.pair.forgotten");
    } catch (raw) {
      said = `${$t("common.error")}: ${String(raw)}`;
    }
  }

  /**
   * Forgets a host's key and dials again. The only way past a changed host
   * key, and a person's choice — the actor never takes it by itself.
   */
  async function forgetHostAndConnect(destination: string) {
    said = null;
    try {
      await invoke("companion_forget_host", { destination });
      await connect(destination);
    } catch (raw) {
      said = `${$t("common.error")}: ${String(raw)}`;
    }
  }

  /** The public half, to the clipboard; the line itself stays selectable. */
  async function copyKey() {
    if (!keyLine) return;
    try {
      await navigator.clipboard.writeText(keyLine);
      said = $t("mobile.pair.copied");
    } catch {
      said = $t("mobile.pair.copyFailed");
    }
  }

  // The actor's reports, counted here: the backoff is its own business, and
  // the honest thing to show is how many times it tried and what it last said.
  $effect(() => {
    const state = $conn;
    untrack(() => {
      if (state.state === "unreachable") attempts += 1;
      else if (state.state === "connected") attempts = 0;
    });
  });

  // Connected is when there is something to reconcile against.
  $effect(() => {
    if (connected) void reconcile();
  });

  onMount(() => {
    const cleanups = [
      startConnListener(),
      startIncomingRouter((key, vars) => get(t)(key, vars)),
    ];

    void (async () => {
      try {
        keyLine = await invoke<string>("companion_key");
      } catch (raw) {
        keyError = String(raw);
      }
      const known = await invoke<string | null>("companion_destination").catch(() => null);
      if (known) {
        typed = known;
        await connect(known);
      }
    })();

    // Suspended is not connected (D4): leaving the front ends the actor, and
    // coming back redials the remembered destination — unless the person
    // cancelled, in which case connecting again is theirs to do.
    const onVisible = () => {
      if (document.visibilityState === "visible") {
        if (remembered && !cancelled) void connect(remembered);
      } else {
        void invoke("companion_disconnect").catch(() => {});
      }
    };
    document.addEventListener("visibilitychange", onVisible);

    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      for (const cleanup of cleanups) void cleanup.then((unlisten) => unlisten());
    };
  });

  const live = $derived($allSessions.filter((s) => isLive(s.state)));

  /** The session the composer is aimed at, when one is open. */
  let directing: string | null = $state(null);
  let instruction = $state("");
  let sending = $state(false);
  let said: string | null = $state(null);

  /**
   * Approving or denying one request, explicitly, by request.
   *
   * The same method the desktop and the terminal call, with the same effect —
   * which is the subset rule holding: this surface is not the only consumer of
   * anything (design D5).
   */
  async function decide(requestId: string, optionId: string) {
    said = null;
    try {
      await request("permission/decide", { requestId, optionId });
      said = $t("mobile.permissions.decided");
    } catch (raw) {
      const e = raw as { message?: string };
      said = `${$t("common.error")}: ${e?.message ?? String(raw)}`;
    }
    await refreshPending().catch(() => {});
  }

  /** One worktree of the active project, as `worktree/list` reports it. */
  interface Lane {
    change: string;
    task?: string;
    branch?: string;
    path: string;
  }

  let lanes: Lane[] = $state([]);
  let adopting: { lane: Lane; file: string } | null = $state(null);

  /** Reads the lanes of the active project, on demand and never on a timer. */
  async function readLanes() {
    const root = $activeProject;
    if (!root) {
      lanes = [];
      return;
    }
    try {
      const answer = await request<{ worktrees?: Lane[] }>("worktree/list", {
        projectRoot: root,
      });
      lanes = answer.worktrees ?? [];
    } catch {
      lanes = [];
    }
  }

  /**
   * Adopting one file from a lane, and only after the user confirms.
   *
   * Two gates, on purpose. This surface asks first — the dialog below — and the
   * daemon asks again: `worktree/merge-file` takes `confirm`, and without it it
   * refuses. Adopting is a governed decision and traced like any merge; what it
   * is NOT is editing, and there is nowhere here to type content.
   */
  async function adopt(lane: Lane, file: string) {
    const root = $activeProject;
    if (!root) return;
    said = null;
    try {
      await request("worktree/merge-file", {
        projectRoot: root,
        target: root,
        source: lane.path,
        file,
        confirm: true,
      });
      said = $t("mobile.review.adopted");
    } catch (raw) {
      const e = raw as { message?: string };
      said = `${$t("common.error")}: ${e?.message ?? String(raw)}`;
    } finally {
      adopting = null;
    }
  }

  /**
   * Sending an instruction to a session that already exists.
   *
   * Directing, not authoring: this is text for the agent, and no file is
   * written from here. The spec's line is between deciding and redacting, and
   * an instruction is on the deciding side.
   */
  async function direct(sessionId: string) {
    const text = instruction.trim();
    if (!text || sending) return;
    sending = true;
    said = null;
    try {
      await request("session/direct", { sessionId, instruction: text });
      instruction = "";
      directing = null;
      said = $t("mobile.direct.sent");
    } catch (raw) {
      const e = raw as { message?: string };
      said = `${$t("common.error")}: ${e?.message ?? String(raw)}`;
    } finally {
      sending = false;
    }
  }
</script>

<!-- This app's key, which is the only half that leaves. Rendered once in the
     source and shown wherever it is needed, so the rule "no field pastes a
     private key" stays a rule about one block. -->
{#snippet keyBlock()}
  <div class="field">
    <span class="label">{$t("mobile.pair.key")}</span>
    {#if keyLine}
      <code class="key">{keyLine}</code>
      <div class="actions">
        <button onclick={() => void copyKey()}>
          <Icon name="copy" size={16} />
          {$t("mobile.pair.copy")}
        </button>
      </div>
    {:else if keyError}
      <p class="hint">{$t("mobile.pair.keyMissing", { error: keyError })}</p>
    {/if}
  </div>
{/snippet}

<!-- The destination, which is the companion's second and last text field: a
     place to go, not content for a file. The host parses it; this only keeps
     an empty one from being sent. -->
{#snippet destinationField()}
  <div class="field">
    <label class="label" for="destination">{$t("mobile.pair.destination")}</label>
    <input
      id="destination"
      type="text"
      bind:value={typed}
      placeholder={$t("mobile.pair.placeholder")}
      autocapitalize="off"
      autocorrect="off"
      spellcheck="false"
      enterkeyhint="go"
      onkeydown={(event) => {
        if (event.key === "Enter" && worthSending(typed)) void connect(typed);
      }}
    />
    <div class="actions">
      <button class="primary" disabled={!worthSending(typed)} onclick={() => void connect(typed)}>
        {$t("mobile.pair.connect")}
      </button>
      {#if remembered}
        <button onclick={() => void forgetDestination()}>
          {$t("mobile.pair.forgetDestination")}
        </button>
      {/if}
    </div>
  </div>
{/snippet}

<div class="app">
  {#if paired && view !== "settings"}
    <!-- Without a channel there is nothing to show, and that is deliberate.
         This screen says what is missing and how it is fixed — and offers the
         way, right here: the key, the destination, connect. It is not an
         error, and it offers no other way in. -->
    <main class="pairing">
      <h1>{$t("mobile.nochannel.title")}</h1>
      <p>{$t("mobile.nochannel.body")}</p>
      {#if paired.kind === "unpaired"}
        <p class="hint">{$t("mobile.nochannel.how")}</p>
        {@render keyBlock()}
        {@render destinationField()}
        <p class="hint">{$t("mobile.pair.sameNetwork")}</p>
      {:else if paired.kind === "connecting"}
        <!-- A glyph of connection, not of error: nothing failed. The count
             and the last word are the actor's; no timer is invented here. -->
        <p class="status" role="status">
          <Icon name="refresh" size={16} />
          {$t("mobile.pair.connecting", { destination: paired.destination, attempt: paired.attempt })}
        </p>
        {#if paired.detail}
          <p class="hint">{paired.detail}</p>
        {/if}
        <div class="actions">
          <button onclick={() => void cancel()}>{$t("mobile.pair.cancel")}</button>
        </div>
      {:else}
        <!-- Refused for good. The actor has stopped; what happens next is a
             person's choice, and the destructive one is not under the thumb. -->
        <p class="status" role="alert">
          <Icon name="warning" size={16} />
          {paired.cause === "hostKeyChanged"
            ? $t("mobile.refused.hostKeyChanged", { destination: paired.destination })
            : $t("mobile.refused.rejected", { destination: paired.destination })}
        </p>
        <p class="hint">{paired.detail}</p>
        {#if actions(paired).includes("copyKey")}
          {@render keyBlock()}
        {/if}
        <div class="actions wrap">
          {#each actions(paired) as action (action)}
            {#if action === "forgetHost"}
              <button onclick={() => void forgetHostAndConnect(paired.destination)}>
                {$t("mobile.refused.forgetHost")}
              </button>
            {:else if action === "stay"}
              <button class="primary" onclick={() => void cancel()}>
                {$t("mobile.refused.stay")}
              </button>
            {:else if action === "retry"}
              <button class="primary" onclick={() => void connect(paired.destination)}>
                {$t("mobile.refused.retry")}
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </main>
  {:else}
    <main>
      {#if view === "sessions"}
        <h1>{$t("mobile.nav.sessions")}</h1>
        {#if live.length === 0}
          <p class="hint">{$t("mobile.sessions.none")}</p>
        {:else}
          <ul class="rows">
            {#each live as session (session.sessionId)}
              <li>
                <span class="name">{session.title ?? session.sessionId}</span>
                <span class="hint">{session.state}</span>
                {#if directing === session.sessionId}
                  <!-- Text for the agent, not content for a file: directing is
                       on the deciding side of the spec's line. -->
                  <textarea
                    bind:value={instruction}
                    rows="3"
                    placeholder={$t("mobile.direct.placeholder")}
                  ></textarea>
                  <div class="actions">
                    <button onclick={() => (directing = null)}>
                      {$t("common.cancel")}
                    </button>
                    <button
                      class="primary"
                      disabled={sending || instruction.trim() === ""}
                      onclick={() => void direct(session.sessionId)}
                    >
                      {$t("mobile.direct.send")}
                    </button>
                  </div>
                {:else}
                  <div class="actions">
                    <button onclick={() => { directing = session.sessionId; instruction = ""; }}>
                      {$t("mobile.direct.open")}
                    </button>
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === "permissions"}
        <h1>{$t("mobile.nav.permissions")}</h1>
        {#if $pending.length === 0}
          <p class="hint">{$t("mobile.permissions.none")}</p>
        {:else}
          <ul class="rows">
            {#each $pending as waiting (waiting.requestId)}
              <li>
                <span class="name">{waiting.tool}</span>
                <span class="hint">{waiting.summary}</span>
                <!-- One decision per request, never in bulk, and each option is
                     the agent's own word for it. The layout does not move while
                     a timer runs, and the destructive option is not the one the
                     thumb lands on. -->
                <div class="actions wrap">
                  {#each waiting.options as option (option.optionId)}
                    <button
                      class:primary={option.kind === "allow_once" || option.kind === "allow_always"}
                      onclick={() => void decide(waiting.requestId, option.optionId)}
                    >
                      {option.name}
                    </button>
                  {/each}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if view === "review"}
        <h1>{$t("mobile.nav.review")}</h1>
        <!-- No editor and no free-text field over content: the spec forbids
             authoring here, and adopting is a governed decision that asks for
             confirmation rather than a place to type. -->
        <p class="hint">{$t("mobile.review.decide")}</p>
        {#if lanes.length === 0}
          <p class="hint">{$t("mobile.review.none")}</p>
        {:else}
          <ul class="rows">
            {#each lanes as lane (lane.path)}
              <li>
                <span class="name">{lane.change}{lane.task ? ` · ${lane.task}` : ""}</span>
                <span class="hint">{lane.branch ?? lane.path}</span>
              </li>
            {/each}
          </ul>
          <p class="hint">{$t("mobile.review.soon")}</p>
        {/if}
      {:else}
        <!-- Reachable with or without a channel: the key and the destination
             are what somebody opens Settings for when nothing connects. -->
        <h1>{$t("mobile.nav.settings")}</h1>
        <p class="hint">{$t("mobile.settings.noaccounts")}</p>
        {@render keyBlock()}
        {@render destinationField()}
        <p class="hint">{$t("mobile.pair.sameNetwork")}</p>
      {/if}
    </main>
  {/if}

  {#if adopting}
    <!-- The surface's own gate. The daemon has another: `worktree/merge-file`
         refuses without `confirm`. Two gates for one irreversible thing. -->
    <div class="confirm" role="dialog" aria-modal="true" aria-label={$t("mobile.review.adopt")}>
      <p>{$t("mobile.review.adopt.ask", { file: adopting.file })}</p>
      <div class="actions">
        <button onclick={() => (adopting = null)}>{$t("common.cancel")}</button>
        <button class="primary" onclick={() => void adopt(adopting!.lane, adopting!.file)}>
          {$t("mobile.review.adopt")}
        </button>
      </div>
    </div>
  {/if}

  {#if said}
    <p class="said" role="status">{said}</p>
  {/if}

  <!-- Four destinations and no fifth: the list comes from the module a test
       holds, not from markup somebody can extend on an afternoon. -->
  <nav aria-label={$t("mobile.nav.label")}>
    {#each DESTINATIONS as destination (destination.id)}
      <button
        class:current={view === destination.id}
        aria-current={view === destination.id ? "page" : undefined}
        onclick={() => (view = destination.id)}
      >
        <Icon name={destination.icon} size={20} />
        <span>{$t(destination.label as never)}</span>
      </button>
    {/each}
  </nav>
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: 1fr auto;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
  }
  main {
    overflow: auto;
    padding: var(--sp-4);
    /* The notch at the top is not ours to paint under. */
    padding-top: calc(var(--sp-4) + env(safe-area-inset-top));
  }
  .pairing {
    display: grid;
    align-content: start;
    gap: var(--sp-3);
  }
  .pairing > p {
    margin: 0;
  }
  h1 {
    margin: 0 0 var(--sp-3);
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--sp-2);
  }
  .rows li {
    display: grid;
    gap: 2px;
    padding: var(--sp-2);
    background: var(--surface);
    border: 1px solid var(--hair);
    border-radius: var(--radius-control);
    /* Rows are targets too, and a target under the minimum is a target that
       gets missed. */
    min-height: 44px;
  }
  .name {
    font-weight: 500;
  }
  .hint {
    color: var(--text-muted);
    font-size: var(--fs-caption);
  }
  .label {
    display: block;
    color: var(--text-muted);
    font-size: var(--fs-caption);
  }
  .field {
    display: grid;
    gap: var(--sp-2);
    margin-top: var(--sp-2);
  }
  /* Selectable as a whole, because copying it by hand is a path that must
     work when the clipboard does not. Broken anywhere, because a key line is
     longer than any phone is wide. */
  .key {
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    word-break: break-all;
    user-select: all;
    padding: var(--sp-2);
    background: var(--surface);
    border: 1px solid var(--hair);
    border-radius: var(--radius-control);
  }
  /* A glyph of connection, not of error: no red, no alarm. */
  .status {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin: 0;
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
    margin-top: var(--sp-2);
  }
  .actions.wrap {
    flex-wrap: wrap;
  }
  .actions button {
    /* Every control is a target, and a target under the minimum is one that
       gets missed on a phone. */
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    min-height: var(--touch-min);
    padding: 0 var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
  }
  .actions button.primary {
    border-color: var(--accent);
    color: var(--accent);
  }
  .actions button:disabled {
    opacity: 0.5;
  }
  textarea,
  input {
    margin-top: var(--sp-2);
    width: 100%;
    padding: var(--sp-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }
  textarea {
    resize: vertical;
  }
  input {
    margin-top: 0;
    min-height: var(--touch-min);
    font-family: var(--font-mono);
  }
  .confirm {
    position: fixed;
    inset: auto 0 0 0;
    padding: var(--sp-4);
    padding-bottom: calc(var(--sp-4) + env(safe-area-inset-bottom));
    background: var(--surface);
    border-top: 1px solid var(--border);
  }
  .said {
    margin: 0;
    padding: var(--sp-2) var(--sp-4);
    background: var(--surface-2);
    border-top: 1px solid var(--hair);
    font-size: var(--fs-caption);
  }
  nav {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    border-top: 1px solid var(--hair);
    background: var(--panel);
    /* The home indicator sits below this; a navigation under it cannot be
       pressed. Measured on a device and not assumed: with the inset alone the
       gesture bar crossed the labels, so there is a floor under it. `max()`
       rather than a fixed value, because a device with a real inset needs the
       real one. */
    padding-bottom: max(env(safe-area-inset-bottom), 16px);
  }
  nav button {
    display: grid;
    justify-items: center;
    gap: 2px;
    /* The design system's own token, which exists for exactly this. */
    min-height: var(--touch-min);
    padding: var(--sp-2) 0;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: var(--fs-caption);
  }
  nav button.current {
    color: var(--accent);
  }
</style>
