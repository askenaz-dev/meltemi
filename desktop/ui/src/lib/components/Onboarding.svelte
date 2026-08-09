<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { locale, setLocale, t } from "../i18n";

  let { onClose }: { onClose: () => void } = $props();

  let closeButton: HTMLButtonElement | undefined = $state();

  $effect(() => {
    closeButton?.focus();
  });

  function dismiss() {
    // Persisted in the user's data directory: never repeats by itself,
    // re-invocable from help (`?`). No account, no network, no telemetry.
    void invoke("onboarding_mark_seen");
    onClose();
  }

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      dismiss();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="scrim" role="presentation" onkeydown={onKeydown} onclick={onClose}>
  <div
    class="card"
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
  >
    <h2 id="onboarding-title">{$t("onboarding.title")}</h2>
    <p class="intro">{$t("onboarding.intro")}</p>
    <ul>
      <li>{$t("onboarding.views")}</li>
      <li>{$t("onboarding.palette")}</li>
      <li>{$t("onboarding.permissions")}</li>
      <li>{$t("help.keys")}</li>
      <li>{$t("onboarding.help")}</li>
    </ul>
    <div class="language" role="group" aria-label={$t("common.language")}>
      <span>{$t("common.language")}:</span>
      <button
        aria-pressed={$locale === "es"}
        class:selected={$locale === "es"}
        onclick={() => setLocale("es")}>ES</button
      >
      <button
        aria-pressed={$locale === "en"}
        class:selected={$locale === "en"}
        onclick={() => setLocale("en")}>EN</button
      >
    </div>
    <div class="actions">
      <button bind:this={closeButton} class="primary" onclick={dismiss}>
        {$t("onboarding.skip")}
      </button>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.4);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .card {
    width: min(560px, 92vw);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--sp-6);
    display: grid;
    gap: var(--sp-3);
  }
  h2 {
    margin: 0;
    font-size: 1.125rem;
  }
  .intro {
    margin: 0;
    color: var(--text-muted);
  }
  ul {
    margin: 0;
    padding-left: 1.2em;
    display: grid;
    gap: var(--sp-2);
  }
  .language {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  button {
    font: inherit;
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .selected {
    border-color: var(--accent);
    color: var(--accent);
  }
  .primary {
    border-color: var(--accent);
  }
</style>
