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
    <!-- The mark of brand/meltemi-mark-gradient.svg, its outline drawn once as
         the welcome opens: the one movement the brand has, with no splash and
         no glow. Its gradient carries an id of its own so it cannot collide
         with the bar's, and the forced-colors rule for `.mark` covers it
         (senales-de-vida D7). -->
    <svg class="mark" viewBox="0 0 256 256" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient
          id="melMarkWelcome"
          x1="38"
          y1="222"
          x2="219"
          y2="54"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stop-color="var(--mel-aegean)" />
          <stop offset="1" stop-color="var(--mel-wind)" />
        </linearGradient>
      </defs>
      <path
        class="stroke"
        pathLength="1"
        d="M43 178C43 125 57 100 78 100C99 100 109 127 109 176C109 105 132 67 158 67C185 67 199 97 196 132C194 154 185 169 172 177C189 176 205 166 217 152C224 144 230 133 234 122"
        fill="none"
        stroke="url(#melMarkWelcome)"
        stroke-width="25"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <path class="hull" d="M45 199C87 211 145 215 202 189C195 204 177 216 151 220C106 226 67 215 45 199Z" fill="url(#melMarkWelcome)" />
    </svg>
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
  .mark {
    width: 40px;
    height: 40px;
    display: block;
  }
  /* H brand: the outline drawn once, 600 ms, and already drawn under reduced
     motion — the resting state IS the drawn mark, the animation only arrives
     at it. */
  .mark .stroke {
    stroke-dasharray: 1;
    stroke-dashoffset: 0;
    animation: mark-draw var(--dur-mark) var(--ease-out) both;
  }
  @media (prefers-reduced-motion: reduce) {
    .mark .stroke {
      animation: none;
    }
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
