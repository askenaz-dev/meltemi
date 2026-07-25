<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";

  let {
    title,
    message,
    confirmLabel,
    onConfirm,
    onCancel,
    extraLabel,
    onExtra,
  }: {
    title: string;
    message: string;
    confirmLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
    /** A third, non-destructive path (e.g. "save") for the dirty guard. */
    extraLabel?: string;
    onExtra?: () => void;
  } = $props();

  let cancelButton: HTMLButtonElement | undefined = $state();
  let dialog: HTMLDivElement | undefined = $state();

  // Non-destructive default: focus lands on Cancel (gui-shell a11y + the
  // TUI's modal contract); Esc always cancels safely.
  $effect(() => {
    cancelButton?.focus();
  });

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
      return;
    }
    if (event.key === "Tab" && dialog) {
      const focusables = dialog.querySelectorAll<HTMLElement>("button");
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="scrim" role="presentation" onkeydown={onKeydown}>
  <div
    class="dialog"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-message"
    bind:this={dialog}
  >
    <h2 id="confirm-title">{title}</h2>
    <p id="confirm-message">{message}</p>
    <div class="actions">
      <button bind:this={cancelButton} onclick={onCancel}>
        {$t("common.cancel")}
      </button>
      {#if extraLabel && onExtra}
        <button class="primary" onclick={onExtra}>{extraLabel}</button>
      {/if}
      <button class="destructive" onclick={onConfirm}>{confirmLabel}</button>
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
    z-index: 40;
  }
  .dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-overlay);
    padding: var(--sp-6);
    max-width: 44ch;
    display: grid;
    gap: var(--sp-3);
  }
  h2 {
    margin: 0;
    font-size: 1rem;
  }
  p {
    margin: 0;
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
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
  .destructive {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
