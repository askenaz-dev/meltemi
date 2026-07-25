<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../i18n";
  import Icon from "./Icon.svelte";

  let {
    label,
    onClose,
    children,
  }: { label: string; onClose: () => void; children: Snippet } = $props();

  // Esc closes the detail panel first; the list keeps its place (spec).
  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      event.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside
  aria-label={label}
  onkeydown={onKeydown}
  tabindex="-1"
>
  <header>
    <span class="title">{label}</span>
    <button class="ghost" aria-label={$t("common.close")} onclick={onClose}>
      <Icon name="close" size={14} />
    </button>
  </header>
  <div class="body">{@render children()}</div>
</aside>

<style>
  aside {
    width: 268px;
    flex: none;
    border-left: 1px solid var(--hair);
    background: var(--panel);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--hair);
  }
  .title {
    font-size: var(--fs-dense);
    font-weight: 500;
  }
  .body {
    padding: var(--panel-pad);
    overflow: auto;
    display: grid;
    gap: var(--sp-3);
    align-content: start;
    min-height: 0;
  }
</style>
