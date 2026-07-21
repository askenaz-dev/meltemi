<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  // Placeholder shell over the live bridge: the real chrome and views land
  // with gui-tauri-paridad block 2. Connection state is already honest.
  import { onMount } from "svelte";
  import { conn, startConnListener } from "./lib/daemon";

  onMount(() => {
    const pending = startConnListener();
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  });
</script>

<main>
  <h1>Meltemi</h1>
  {#if $conn.state === "connecting"}
    <p>◌ conectando…</p>
  {:else if $conn.state === "connected"}
    <p>
      <span class="ok">▸ conectado</span> — daemon v{$conn.version} ·
      {$conn.sessions} sesión(es)
    </p>
  {:else}
    <p class="danger">▲ daemon inalcanzable</p>
    <p class="detail">{$conn.detail}</p>
    <p class="detail">endpoint: <code>{$conn.endpoint}</code></p>
  {/if}
</main>

<style>
  main {
    height: 100%;
    display: grid;
    place-content: center;
    text-align: center;
    gap: var(--sp-2);
  }

  h1 {
    margin: 0;
    font-size: 1.25rem;
  }

  p {
    margin: 0;
    color: var(--text-muted);
  }

  .ok {
    color: var(--ok);
  }

  .danger {
    color: var(--danger);
  }

  .detail {
    font-family: var(--font-mono);
    font-size: 0.8125rem;
  }
</style>
