<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { get } from "svelte/store";
  import { conn, request } from "../daemon";
  import { locale, setLocale, t } from "../i18n";
  import { activeProject, pushNotice } from "../stores";
  import {
    setOpenWithTemplate,
    setStoredLocale,
    setTheme,
    uiState,
    type ThemeChoice,
  } from "../ui-state";
  import Icon from "../components/Icon.svelte";

  let { onEditFile }: { onEditFile: (file: string) => void } = $props();

  const THEMES: ThemeChoice[] = ["system", "light", "dark"];

  let template = $state("");
  let config: { file: string; content: string }[] = $state([]);
  let loadingConfig = $state(true);

  $effect(() => {
    template = $uiState.openWithTemplate ?? "";
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
  .privacy {
    margin: 0;
    padding-left: 1.2em;
    display: grid;
    gap: var(--sp-1);
    color: var(--text-muted);
    font-size: var(--fs-dense);
  }
</style>
