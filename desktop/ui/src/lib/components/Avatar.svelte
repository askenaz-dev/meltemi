<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { initialsFor, toneFor } from "../agents";

  let {
    id,
    name,
    size = 22,
  }: { id: string; name?: string; size?: number } = $props();

  const tone = $derived(toneFor(id));
  const initials = $derived(initialsFor(name ?? id, id));
</script>

<span
  class="avatar"
  style="width:{size}px; height:{size}px; background:{tone.bg}; color:{tone.fg}; font-size:{Math.round(
    size * 0.45,
  )}px"
  aria-hidden="true">{initials}</span
>

<style>
  .avatar {
    flex: none;
    display: inline-grid;
    place-items: center;
    border-radius: var(--radius-control);
    font-weight: 500;
    letter-spacing: 0.2px;
  }
</style>
