<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  // The mobile companion: the Agent Boss's remote post (companero-movil D1, D6).
  //
  // Not the desktop shell made small. The sidebar, the tabs and the three zones
  // exist because there is a mouse and 1,280 pixels; what is reused is
  // everything below them — the stores, the catalogue, the decision modules —
  // which is almost everything that matters.
  import { conn, request } from "./lib/daemon";
  import { t } from "./lib/i18n";
  import Icon from "./lib/components/Icon.svelte";
  import { DESTINATIONS, type MobileView } from "./lib/mobile-nav";
  import {
    allSessions,
    pending,
    refreshPending,
    refreshSessions,
    isLive,
  } from "./lib/stores";

  let view: MobileView = $state("sessions");

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
    ]);
  }

  $effect(() => {
    void reconcile();
    const onVisible = () => {
      if (document.visibilityState === "visible") void reconcile();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  });

  const live = $derived($allSessions.filter((s) => isLive(s.state)));
  const connected = $derived($conn.state === "connected");

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

<div class="app">
  {#if !connected}
    <!-- Without a channel there is nothing to show, and that is deliberate.
         This screen says what is missing and how it is fixed; it is not an
         error, and it offers no other way in. -->
    <main class="nothing">
      <h1>{$t("mobile.nochannel.title")}</h1>
      <p>{$t("mobile.nochannel.body")}</p>
      <p class="hint">{$t("mobile.nochannel.how")}</p>
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
        <p class="hint">{$t("mobile.review.decide")}</p>
        <!-- No editor and no free-text field over content: the spec forbids
             authoring here, and adopting is a governed decision that asks for
             confirmation rather than a place to type. -->
        <p class="hint">{$t("mobile.review.soon")}</p>
      {:else}
        <h1>{$t("mobile.nav.settings")}</h1>
        <p class="hint">{$t("mobile.settings.noaccounts")}</p>
      {/if}
    </main>
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
  .nothing {
    display: grid;
    align-content: center;
    gap: var(--sp-3);
    text-align: center;
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
  textarea {
    margin-top: var(--sp-2);
    width: 100%;
    padding: var(--sp-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    resize: vertical;
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
       pressed. */
    padding-bottom: env(safe-area-inset-bottom);
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
