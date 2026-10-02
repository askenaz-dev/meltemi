<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { get } from "svelte/store";
  import { conn, request } from "../daemon";
  import { locale, setLocale, t } from "../i18n";
  import {
    activeProject,
    identity,
    identityGesture,
    machines,
    probeMachine,
    pushNotice,
    refreshIdentity,
    refreshMachines,
    switchMachine,
    type LinkGesture,
  } from "../stores";
  import { orderedMachines, presenceGlyph } from "../identity";
  import { probeLine, withPresence } from "../machines";
  import {
    setOpenWithTemplate,
    setStoredLocale,
    setTheme,
    uiState,
    type ThemeChoice,
  } from "../ui-state";
  import Icon from "../components/Icon.svelte";

  let {
    onEditFile,
    onGuarded,
  }: {
    onEditFile: (file: string) => void;
    /**
     * Runs `go` once unsaved work has been dealt with. The SAME guard that
     * leaving the editor uses, handed down rather than reimplemented: two
     * guards for one danger is how two behaviours appear for one question
     * (maquinas-remotas design D2).
     */
    onGuarded: (go: () => void) => void;
  } = $props();

  /**
   * The waiting notice. The command itself is not editable here on purpose: it
   * is user-scope configuration, edited in the user's own file like every other
   * user-scope setting, and a field in this view would suggest the project could
   * carry it (aviso-de-espera-opt-in design D3). What this view does is say
   * whether one is configured and let a person rehearse it, which is the only
   * moment a misconfiguration is visible.
   */
  let noticeConfigured = $state(false);
  let noticeResult: "sent" | "none" | null = $state(null);

  const loadNoticeState = async () => {
    try {
      const state = await invoke<{ configured: boolean }>("notice_state");
      noticeConfigured = state.configured;
    } catch {
      noticeConfigured = false;
    }
  };

  const testNotice = async () => {
    noticeResult = null;
    try {
      noticeResult = (await invoke<boolean>("notice_test")) ? "sent" : "none";
    } catch {
      noticeResult = "none";
    }
    await loadNoticeState();
  };

  /**
   * The linking gesture, asked for and never run.
   *
   * This surface has no plugin to launch a process and no reach to any https
   * origin — its CSP is pinned by a test — so "copy it" is not a limitation
   * worked around but the whole design: the user runs it, in their terminal,
   * and signs in with their own provider (identidad-propia design D3).
   */
  let gesture: LinkGesture | null = $state(null);
  let copied = $state(false);

  const loadGesture = async () => {
    try {
      gesture = await identityGesture();
    } catch {
      // No gesture to offer is not an error of this view: the identity block
      // above already says why, with the daemon's own remedy.
      gesture = null;
    }
  };

  $effect(() => {
    // Asked for when the view opens, so the answer is current rather than
    // whatever was true when the window did.
    void refreshIdentity().then(loadGesture);
  });

  async function copyGesture() {
    if (!gesture) return;
    // The platform's own form: the plain reading is the one that breaks when
    // pasted into a shell that quotes differently.
    const text = navigator.userAgent.includes("Windows") ? gesture.powershell : gesture.posix;
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      pushNotice($t("settings.identity.copied"), "info");
    } catch {
      copied = false;
    }
  }

  /** The result of the last test, per machine. Nothing is remembered longer. */
  let probes: Record<string, string> = $state({});
  let probing: string | null = $state(null);

  $effect(() => {
    void refreshMachines();
  });

  /** The configured machines, with the mesh's presence when it knows them. */
  const shownMachines = $derived(withPresence($machines.machines, $identity));

  async function testMachine(name: string) {
    probing = name;
    try {
      const probe = await probeMachine(name);
      probes = {
        ...probes,
        [name]: probeLine(probe, {
          answered: $t("settings.machines.answered"),
          failed: $t("settings.machines.failed"),
        }),
      };
    } catch (raw) {
      const e = raw as { message?: string };
      probes = {
        ...probes,
        [name]: `${$t("settings.machines.failed")}: ${e?.message ?? String(raw)}`,
      };
    } finally {
      probing = null;
    }
  }

  function useMachine(name: string) {
    // The guard first, always: the tabs of the machine being left point at
    // another daemon's sessions.
    onGuarded(() => {
      void switchMachine(name)
        .then((switched) =>
          pushNotice($t("settings.machines.switched", { name: switched }), "info"),
        )
        .catch((raw) => {
          const e = raw as { message?: string };
          pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
        });
    });
  }

  const THEMES: ThemeChoice[] = ["system", "light", "dark"];

  let template = $state("");
  let config: { file: string; content: string }[] = $state([]);
  let loadingConfig = $state(true);

  $effect(() => {
    template = $uiState.openWithTemplate ?? "";
  });

  $effect(() => {
    void loadNoticeState();
  });

  // The project's effective configuration, read-only, with a jump to edit it
  // through the traceable editor.
  $effect(() => {
    const root = get(activeProject);
    if (!root) {
      loadingConfig = false;
      return;
    }
    const wanted = [".meltemi/config.toml", ".meltemi/permissions.toml"];
    void Promise.all(
      wanted.map(async (file) => {
        try {
          const read = await import("../editor/files").then((m) => m.readFile(root, file));
          return { file, content: read.content };
        } catch {
          return { file, content: "" };
        }
      }),
    ).then((entries) => {
      config = entries;
      loadingConfig = false;
    });
  });

  function saveTemplate() {
    setOpenWithTemplate(template);
    pushNotice($t("settings.openWith.saved"), "info");
  }

  function pickLocale(next: "es" | "en") {
    setLocale(next);
    setStoredLocale(next);
  }

  async function projectContext() {
    const root = get(activeProject);
    if (!root) return;
    try {
      await request("context/project", { projectRoot: root });
      pushNotice($t("settings.project.projected"), "info");
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    }
  }
</script>

<div class="settings">
  <section aria-labelledby="appearance">
    <h2 id="appearance">{$t("settings.appearance")}</h2>
    <div class="row">
      <span class="label">{$t("settings.theme")}</span>
      <div class="choices" role="group" aria-label={$t("settings.theme")}>
        {#each THEMES as choice (choice)}
          <button
            class:sel={($uiState.theme ?? "system") === choice}
            aria-pressed={($uiState.theme ?? "system") === choice}
            onclick={() => setTheme(choice)}
          >
            {$t(("settings.theme." + choice) as never)}
          </button>
        {/each}
      </div>
    </div>
    <div class="row">
      <span class="label">{$t("common.language")}</span>
      <div class="choices" role="group" aria-label={$t("common.language")}>
        <button aria-pressed={$locale === "es"} class:sel={$locale === "es"} onclick={() => pickLocale("es")}>
          ES
        </button>
        <button aria-pressed={$locale === "en"} class:sel={$locale === "en"} onclick={() => pickLocale("en")}>
          EN
        </button>
      </div>
    </div>
  </section>

  <section aria-labelledby="editor">
    <h2 id="editor">{$t("settings.editor")}</h2>
    <div class="row">
      <span class="label">{$t("settings.openWith")}</span>
      <div class="inline">
        <input
          bind:value={template}
          placeholder={$t("settings.openWith.example")}
          aria-label={$t("settings.openWith")}
        />
        <button onclick={saveTemplate}>{$t("common.save")}</button>
      </div>
    </div>
    <p class="hint">{$t("settings.openWith.hint")}</p>
  </section>

  <section aria-labelledby="notices">
    <h2 id="notices">{$t("settings.notices")}</h2>
    <div class="row">
      <span class="label">{$t("settings.notices.onWaiting")}</span>
      <div class="inline">
        <span class="mono">
          {noticeConfigured ? $t("settings.notices.on") : $t("settings.notices.off")}
        </span>
        <button onclick={testNotice} disabled={!noticeConfigured}>
          {$t("settings.notices.test")}
        </button>
      </div>
    </div>
    {#if noticeResult === "sent"}
      <p class="hint">{$t("settings.notices.sent")}</p>
    {:else if noticeResult === "none"}
      <p class="hint warn">{$t("settings.notices.failed")}</p>
    {/if}
    <p class="hint">{$t("settings.notices.hint")}</p>
  </section>

  <section aria-labelledby="project">
    <h2 id="project">{$t("settings.project")}</h2>
    {#if !$activeProject}
      <p class="hint">{$t("settings.project.none")}</p>
    {:else}
      <p class="hint mono break">{$activeProject}</p>
      {#if loadingConfig}
        <p class="hint">{$t("common.loading")}</p>
      {:else}
        {#each config as entry (entry.file)}
          <div class="configBlock">
            <div class="configHead">
              <code>{entry.file}</code>
              <button class="ghost" onclick={() => onEditFile(entry.file)}>
                <Icon name="editor" size={14} />
                {$t("settings.project.edit")}
              </button>
            </div>
            {#if entry.content.trim() === ""}
              <p class="hint">{$t("settings.project.missing")}</p>
            {:else}
              <pre>{entry.content}</pre>
            {/if}
          </div>
        {/each}
      {/if}
      <button onclick={() => void projectContext()}>{$t("settings.project.project")}</button>
    {/if}
  </section>

  <section aria-labelledby="machines">
    <h2 id="machines">{$t("settings.machines")}</h2>
    <p class="hint">{$t("settings.machines.hint")}</p>
    {#if $machines.machines.length === 0}
      <p class="hint">{$t("settings.machines.none")}</p>
    {:else}
      <ul class="endpoints">
        {#each shownMachines as machine (machine.name)}
          <li>
            <span class="presence" class:on={machine.active} aria-hidden="true"
              >{machine.active ? "●" : "○"}</span
            >
            <span class="name">{machine.name}</span>
            {#if machine.active}
              <span class="word">({$t("settings.machines.here")})</span>
            {/if}
            <span class="hint mono">{machine.via ?? machine.configured}</span>
            {#if machine.presence}
              <!-- The mesh's presence, which is NOT a promise about a daemon:
                   that is learned by testing or by connecting. -->
              <span class="hint"
                >{machine.presence === "online"
                  ? $t("settings.machines.online")
                  : $t("settings.machines.offline")}</span
              >
            {/if}
            <span class="grow"></span>
            <button onclick={() => void testMachine(machine.name)} disabled={probing !== null}>
              {probing === machine.name
                ? $t("settings.machines.testing")
                : $t("settings.machines.test")}
            </button>
            {#if !machine.active}
              <button onclick={() => useMachine(machine.name)}>
                {$t("settings.machines.use")}
              </button>
            {/if}
          </li>
          {#if probes[machine.name]}
            <li class="hint probe">{probes[machine.name]}</li>
          {/if}
        {/each}
      </ul>
    {/if}
    {#each $machines.diagnostics as said (said)}
      <p class="hint mono">{said}</p>
    {/each}
  </section>

  <section aria-labelledby="identity">
    <h2 id="identity">{$t("settings.identity")}</h2>
    <!-- Whose identity this is, said where it is read: nobody may take this
         section for a Meltemi account, because there are none (design D7). -->
    <p class="hint">{$t("settings.identity.whose")}</p>
    {#if $identity === null}
      <p class="hint">{$t("settings.identity.notConfigured")}</p>
    {:else if !$identity.available}
      <p>
        {$identity.reason === "client_missing"
          ? $t("settings.identity.clientMissing")
          : $identity.reason === "not_linked"
            ? $t("settings.identity.notLinked")
            : $t("settings.identity.notConfigured")}
      </p>
      {#if $identity.remedy}
        <p class="hint mono">{$identity.remedy}</p>
      {/if}
    {:else}
      {#if $identity.user}
        <p class="who">{$identity.user}</p>
      {/if}
      {#if $identity.provider}
        <div class="row">
          <span class="label">{$t("settings.identity.provider")}</span>
          <span class="mono">{$identity.provider}</span>
        </div>
      {/if}
      {#if $identity.this}
        <div class="row">
          <span class="label">{$t("settings.identity.this")}</span>
          <span class="mono">{$identity.this}</span>
        </div>
      {/if}
      {#if $identity.machines.length > 0}
        <h3>{$t("settings.identity.machines")}</h3>
        <ul class="machines">
          {#each orderedMachines($identity.machines) as machine (machine.name)}
            <li>
              <!-- Symbol AND word, never either alone and never colour alone:
                   the same rule the terminal follows with its ASCII twin. -->
              <span
                class="presence"
                class:on={machine.presence === "online"}
                aria-hidden="true">{presenceGlyph(machine.presence)}</span
              >
              <span class="name">{machine.name}</span>
              <span class="word"
                >{machine.presence === "online"
                  ? $t("settings.identity.online")
                  : $t("settings.identity.offline")}</span
              >
              {#if machine.system}<span class="hint">{machine.system}</span>{/if}
              {#if machine.lastSeen}
                <span class="hint">{$t("settings.identity.lastSeen", { at: machine.lastSeen })}</span>
              {/if}
              {#if machine.thisMachine}
                <span class="hint">({$t("settings.identity.thisMachine")})</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
    {#if gesture}
      <h3>{$t("settings.identity.gesture")}</h3>
      <pre class="gesture">{navigator.userAgent.includes("Windows")
          ? gesture.powershell
          : gesture.posix}</pre>
      <p class="hint">{$t("settings.identity.gesture.hint")}</p>
      {#if gesture.loginServer}
        <div class="row">
          <span class="label">{$t("settings.identity.loginServer")}</span>
          <span class="mono">{gesture.loginServer}</span>
        </div>
      {/if}
      <div class="actions">
        <button onclick={() => void copyGesture()}>
          <Icon name={copied ? "check" : "copy"} size={14} />
          {$t("settings.identity.copy")}
        </button>
        <button onclick={() => void refreshIdentity().then(loadGesture)}>
          <Icon name="refresh" size={14} />
          {$t("settings.identity.reread")}
        </button>
      </div>
    {/if}
  </section>

  <section aria-labelledby="privacy">
    <h2 id="privacy">{$t("settings.privacy")}</h2>
    <ul class="privacy">
      <li>{$t("settings.privacy.noAccounts")}</li>
      <li>{$t("settings.privacy.noNetwork")}</li>
      <li>{$t("settings.privacy.noTelemetry")}</li>
      <li>{$t("settings.privacy.credentials")}</li>
    </ul>
    <p class="hint mono">
      {$conn.state === "connected"
        ? $t("settings.about", { version: $conn.version })
        : $t("settings.about.offline")}
    </p>
  </section>
</div>

<style>
  .settings {
    display: grid;
    gap: var(--sp-6);
    align-content: start;
    padding: var(--panel-pad);
    overflow: auto;
    height: 100%;
    max-width: 760px;
  }
  section {
    display: grid;
    gap: var(--sp-2);
  }
  h2 {
    margin: 0;
    font-size: var(--fs-section);
    font-weight: 500;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }
  .label {
    min-width: 120px;
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
  .choices {
    display: flex;
    gap: var(--sp-1);
  }
  .choices .sel {
    border-color: var(--accent);
    color: var(--accent);
  }
  .inline {
    display: flex;
    gap: var(--sp-2);
    flex: 1;
    min-width: 260px;
  }
  .inline input {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  .hint {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
  .break {
    overflow-wrap: anywhere;
  }
  .configBlock {
    border: 1px solid var(--hair);
    border-radius: var(--radius-panel);
    overflow: hidden;
  }
  .configHead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    background: var(--surface-2);
    border-bottom: 1px solid var(--hair);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
  }
  pre {
    margin: 0;
    padding: var(--sp-3);
    overflow: auto;
    max-height: 200px;
    font-family: var(--font-mono);
    font-size: var(--fs-dense);
    background: var(--surface);
  }
  .endpoints li {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }
  .endpoints .grow {
    flex: 1;
  }
  .endpoints .probe {
    font-family: var(--font-mono);
  }
  .machines,
  .endpoints {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .machines li {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    font-size: var(--fs-caption);
  }
  .machines .name {
    font-family: var(--font-mono);
  }
  /* Colour is added to the symbol, never the thing that carries the meaning:
     the glyph differs in shape and the word is always beside it. */
  .presence {
    color: var(--text-faint);
  }
  .presence.on {
    color: var(--ok);
  }
  .who {
    margin: 0;
    font-weight: 500;
  }
  .gesture {
    margin: 0;
    padding: var(--sp-2);
    background: var(--surface-2);
    border: 1px solid var(--hair);
    border-radius: var(--radius-control);
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .actions {
    display: flex;
    gap: var(--sp-2);
  }
  h3 {
    margin: var(--sp-2) 0 0;
    font-size: var(--fs-caption);
    font-weight: 500;
    color: var(--text-muted);
  }
  .privacy {
    margin: 0;
    padding-left: 1.2em;
    display: grid;
    gap: var(--sp-1);
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
</style>
