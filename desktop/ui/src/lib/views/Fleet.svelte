<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { request } from "../daemon";
  import { binaryName } from "../agents";
  import { fleet, pushNotice, refreshFleet, type FleetAgent } from "../stores";
  import { groupFleet } from "../fleet-groups";
  import Avatar from "../components/Avatar.svelte";
  import Drawer from "../components/Drawer.svelte";
  import EmptyState from "../components/EmptyState.svelte";
  import Icon from "../components/Icon.svelte";

  let selectedId: string | null = $state(null);
  let refreshing = $state(false);

  const selected = $derived($fleet.find((agent) => agent.id === selectedId) ?? null);
  const detected = $derived($fleet.filter((agent) => agent.detected).length);
  /** Each catalog agent followed by its subscriptions (design D1). */
  const rows = $derived(groupFleet($fleet));

  $effect(() => {
    void refreshFleet().catch(() => {});
  });

  async function refresh() {
    refreshing = true;
    try {
      await refreshFleet();
    } catch (raw) {
      const e = raw as { message?: string };
      pushNotice(`${$t("common.error")}: ${e?.message ?? String(raw)}`, "danger");
    } finally {
      refreshing = false;
    }
  }

  async function copyCommand(command: string) {
    try {
      await navigator.clipboard.writeText(command);
      pushNotice($t("fleet.commandCopied"), "info");
    } catch {
      pushNotice($t("banner.copyFailed"), "danger");
    }
  }

  /** The link being composed on the selected entry, and the gesture the last
      link answered with — kept until another entry is selected, because the
      login is the one thing the user still has to run. */
  let linkName = $state("");
  let linking = $state(false);
  let gesture: { profile: string; powershell: string; posix: string } | null = $state(null);

  $effect(() => {
    // Selecting another entry drops the composed state.
    void selectedId;
    linkName = "";
    gesture = null;
  });

  /** Links a subscription on the selected entry (vincular-suscripciones D5):
      the daemon writes the profile and answers with the login gesture it
      never runs — shown here, copyable, the user's to execute. */
  async function linkSubscription() {
    if (!selected || !linkName.trim() || linking) return;
    linking = true;
    try {
      const result = await request<{
        profile: string;
        gesture: { powershell: string; posix: string };
      }>("subscription/link", { agent: selected.id, name: linkName.trim() });
      gesture = {
        profile: result.profile,
        powershell: result.gesture.powershell,
        posix: result.gesture.posix,
      };
      linkName = "";
      pushNotice($t("fleet.link.done", { profile: result.profile }), "info");
      await refreshFleet();
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      const detail = e?.detail ? `: ${e.detail}` : "";
      const remedy = e?.remedy ? ` — ${e.remedy}` : "";
      pushNotice(`${e?.message ?? String(raw)}${detail}${remedy}`, "danger");
    } finally {
      linking = false;
    }
  }

  /** Unlinks the selected profile row. The context directory is deliberately
      left behind (credentials are the provider's, not ours to destroy), and
      the notice names it so the human can decide. */
  async function unlinkSubscription() {
    if (!selected) return;
    try {
      const result = await request<{ profile: string; contextDir: string }>(
        "subscription/unlink",
        { name: selected.id },
      );
      pushNotice($t("fleet.unlink.done", { dir: result.contextDir }), "info");
      selectedId = null;
      await refreshFleet();
    } catch (raw) {
      const e = raw as { message?: string; detail?: string | null; remedy?: string | null };
      const detail = e?.detail ? `: ${e.detail}` : "";
      const remedy = e?.remedy ? ` — ${e.remedy}` : "";
      pushNotice(`${e?.message ?? String(raw)}${detail}${remedy}`, "danger");
    }
  }

  function levelLabel(agent: FleetAgent): string {
    const level = agent.verifiedLevel ?? agent.integrationLevel;
    return `N${level}`;
  }

  /**
   * The word, not just the glyph: `integration-levels` requires the surface to
   * show the declared/verified distinction "con etiqueta textual", and a bare
   * check mark is exactly what that forbids. The glyph stays beside the word so
   * whoever already read it keeps their shortcut.
   */
  function levelState(agent: FleetAgent): string {
    return agent.verifiedLevel !== undefined
      ? $t("fleet.level.verified")
      : $t("fleet.level.declared");
  }
</script>

<div class="wrap">
  <div class="main">
    {#if $fleet.length === 0}
      <EmptyState glyph="fleet" title={$t("fleet.empty.title")} hint={$t("fleet.empty.hint")}>
        <button disabled={refreshing} onclick={() => void refresh()}>
          <Icon name="refresh" size={14} />
          {refreshing ? $t("common.loading") : $t("fleet.refresh")}
        </button>
      </EmptyState>
    {:else}
      <table class="dense">
        <thead>
          <tr>
            <th scope="col">{$t("fleet.col.agent")}</th>
            <th scope="col">{$t("fleet.col.source")}</th>
            <th scope="col">{$t("fleet.col.level")}</th>
            <th scope="col">{$t("fleet.col.detected")}</th>
            <th scope="col">{$t("fleet.col.configured")}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.agent.id)}
            {@const agent = row.agent}
            <tr aria-selected={agent.id === selectedId} class:child={row.child}>
              <td>
                <button
                  class="agent ghost"
                  class:childAgent={row.child}
                  onclick={() => (selectedId = agent.id === selectedId ? null : agent.id)}
                >
                  <Avatar id={agent.id} name={agent.displayName} size={row.child ? 18 : 22} />
                  <span class="names">
                    <span class="name">
                      {agent.displayName}
                      {#if row.subscriptions}
                        <span class="pill sub">
                          {$t("fleet.subscriptions", { n: String(row.subscriptions) })}
                        </span>
                      {/if}
                    </span>
                    {#if row.child}
                      <!-- In words, never by indentation: a screen reader and a
                           copied table must carry the same relation. -->
                      <span class="bin">
                        {row.orphan
                          ? $t("fleet.subscription.orphan", { agent: row.belongsTo || "—" })
                          : $t("fleet.subscription.of", { agent: row.belongsTo ?? "" })}
                      </span>
                    {:else}
                      <span class="bin mono">{binaryName(agent.binaryPath) || agent.id}</span>
                    {/if}
                  </span>
                </button>
              </td>
              <td>
                <span class="pill">{$t(("fleet.source." + agent.source) as never)}</span>
              </td>
              <td>
                <span class="pill" class:ok={agent.verifiedLevel !== undefined}>
                  {levelLabel(agent)} · {levelState(agent)}{agent.verifiedLevel !== undefined
                    ? " ✓"
                    : ""}
                </span>
              </td>
              <td>
                {#if agent.detected}
                  <span class="ok"><span class="dot on" aria-hidden="true"></span>{$t("common.yes")}</span>
                {:else}
                  <span class="faint"><span class="dot" aria-hidden="true"></span>{$t("common.no")}</span>
                {/if}
              </td>
              <td>
                {#if agent.configured}
                  <span class="ok">▸ {$t("common.yes")}</span>
                {:else}
                  <span class="faint">—</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="summary">{$t("fleet.summary", { total: $fleet.length, detected })}</p>
    {/if}
  </div>

  {#if selected}
    <Drawer label={selected.displayName} onClose={() => (selectedId = null)}>
      <div class="head">
        <Avatar id={selected.id} name={selected.displayName} size={32} />
        <div>
          <p class="dname">{selected.displayName}</p>
          {#if selected.detected}
            <p class="ok small">● {$t("fleet.detected")}</p>
          {:else}
            <p class="faint small">■ {$t("fleet.notDetected")}</p>
          {/if}
        </div>
      </div>

      <dl>
        <dt>{$t("fleet.col.level")}</dt>
        <dd>
          {selected.verifiedLevel ?? selected.integrationLevel}
          <span class="faint">
            ({selected.verifiedLevel !== undefined
              ? $t("fleet.level.verified")
              : $t("fleet.level.declared")})
          </span>
        </dd>
        <dt>{$t("fleet.mcp")}</dt>
        <dd>{selected.mcpSupport ? $t("common.yes") : $t("common.no")}</dd>
        <dt>{$t("fleet.col.source")}</dt>
        <dd>{$t(("fleet.source." + selected.source) as never)}</dd>
        {#if selected.underlyingAgent}
          <dt>{$t("fleet.underlyingLabel")}</dt>
          <dd class="mono">{selected.underlyingAgent}</dd>
        {/if}
        <dt>{$t("fleet.binary")}</dt>
        <dd class="mono break">{selected.binaryPath ?? "—"}</dd>
        <dt>{$t("fleet.id")}</dt>
        <dd class="mono">{selected.id}</dd>
      </dl>

      <!-- Two-layer entries: which layer is missing, and the exact command.
           The command is data — shown, copyable, never executed. -->
      {#if selected.layers && selected.layers.length > 1}
        <div class="layers">
          <span class="layersTitle">{$t("fleet.layers")}</span>
          {#each selected.layers as layer (layer.kind + layer.bin)}
            <div class="layer">
              <span class="pill" class:ok={layer.detected && !layer.evidenceOnly}>
                {$t(("fleet.layer." + layer.kind) as never)}
              </span>
              <code class="lbin">{layer.bin}</code>
              {#if layer.detected && layer.evidenceOnly}
                <span class="warnText">! {$t("fleet.layer.shimOnly")}</span>
              {:else if layer.detected}
                <span class="ok">▸ {$t("fleet.layer.found")}</span>
              {:else}
                <span class="faint">■ {$t("fleet.layer.missing")}</span>
              {/if}
              <!-- Where the find came from, when it came with Meltemi: a binary
                   the user never installed should say so, and a missing one
                   should say it is ours to ship (adaptadores-propios-acp D8). -->
              {#if layer.source === "bundled"}
                <span class="faint">· {$t("fleet.layer.bundled")}</span>
              {:else if layer.bundled && !layer.detected}
                <span class="faint">· {$t("fleet.layer.bundledMissing")}</span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if selected.installState && selected.installState !== "ready"}
        <p class="state">
          <span class="pill warn">{$t(("fleet.state." + selected.installState) as never)}</span>
        </p>
        {#if selected.remedy}
          <p class="hint">{selected.remedy}</p>
        {/if}
        {#if selected.remedyCommand}
          <div class="command">
            <code>{selected.remedyCommand}</code>
            <button
              class="ghost"
              aria-label={$t("fleet.copyCommand")}
              onclick={() => void copyCommand(selected.remedyCommand!)}
            >
              <Icon name="copy" size={13} />
            </button>
          </div>
        {/if}
      {/if}

      {#if selected.legalNote}
        <p class="legal">
          <span class="pill" class:warn={selected.legalStatus === "grey"}>
            {$t(("fleet.legal." + (selected.legalStatus ?? "tolerated")) as never)}
          </span>
          {selected.legalNote}
        </p>
      {/if}

      <!-- Linking a subscription (vincular-suscripciones): offered exactly
           where the registry declares the auth-context variable; everywhere
           else the manual path is named instead of a dead control. -->
      {#if selected.authContextVar}
        <div class="linkBox">
          <span class="layersTitle">{$t("fleet.link.title")}</span>
          <div class="linkForm">
            <input
              type="text"
              bind:value={linkName}
              placeholder={$t("fleet.link.placeholder")}
              aria-label={$t("fleet.link.placeholder")}
            />
            <button disabled={linking || !linkName.trim()} onclick={() => void linkSubscription()}>
              {$t("fleet.link.action")}
            </button>
          </div>
          {#if gesture}
            <p class="hint">{$t("fleet.link.gestureHint", { profile: gesture.profile })}</p>
            <div class="command">
              <code>{gesture.powershell}</code>
              <button
                class="ghost"
                aria-label={$t("fleet.copyCommand")}
                onclick={() => void copyCommand(gesture!.powershell)}
              >
                <Icon name="copy" size={13} />
              </button>
            </div>
            <div class="command">
              <code>{gesture.posix}</code>
              <button
                class="ghost"
                aria-label={$t("fleet.copyCommand")}
                onclick={() => void copyCommand(gesture!.posix)}
              >
                <Icon name="copy" size={13} />
              </button>
            </div>
          {/if}
        </div>
      {:else if selected.source === "profile"}
        <div class="linkBox">
          <button onclick={() => void unlinkSubscription()}>
            {$t("fleet.unlink.action")}
          </button>
          <p class="hint">{$t("fleet.unlink.keeps")}</p>
        </div>
      {:else if selected.source === "registry"}
        <p class="hint">{$t("fleet.link.manualHint")}</p>
      {/if}

      <button disabled={refreshing} onclick={() => void refresh()}>
        <Icon name="refresh" size={14} />
        {$t("fleet.refresh")}
      </button>
    </Drawer>
  {/if}
</div>

<style>
  .wrap {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .main {
    flex: 1;
    min-width: 0;
    overflow: auto;
    padding: 0 var(--sp-2);
  }
  /* The indent only accompanies: the relation is in the row's own words. */
  .childAgent {
    padding-left: var(--sp-6);
  }
  tr.child .name {
    font-weight: 400;
  }
  .pill.sub {
    margin-left: var(--sp-1);
  }
  .agent {
    display: flex;
    padding: 0;
    background: transparent;
    border-color: transparent;
    text-align: left;
    width: 100%;
  }
  .names {
    display: grid;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bin {
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-faint);
    margin-right: 6px;
  }
  .dot.on {
    background: var(--ok);
  }
  .ok {
    color: var(--ok);
  }
  .faint {
    color: var(--text-faint);
  }
  .summary {
    margin: var(--sp-2) var(--cell-pad);
    font-size: var(--fs-caption);
    color: var(--text-faint);
  }
  .head {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
  }
  .dname {
    margin: 0;
    font-weight: 500;
  }
  .small {
    margin: 0;
    font-size: var(--fs-caption);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: var(--sp-1) var(--sp-2);
    font-size: var(--fs-dense);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
  }
  .break {
    overflow-wrap: anywhere;
  }
  .hint {
    margin: 0;
    font-size: var(--fs-caption);
    color: var(--text-muted);
  }
  .layers {
    display: grid;
    gap: var(--sp-1);
  }
  .layersTitle {
    font-size: var(--fs-caption);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-faint);
  }
  .layer {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-caption);
  }
  .lbin {
    font-family: var(--font-mono);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .warnText {
    color: var(--warn);
  }
  .state,
  .legal {
    margin: 0;
    font-size: var(--fs-caption);
    color: var(--text-muted);
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    align-items: baseline;
  }
  .command {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    background: var(--surface-2);
    border-radius: var(--radius-control);
    padding: var(--sp-1) var(--sp-2);
  }
  .command code {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    overflow-wrap: anywhere;
  }
  .linkBox {
    display: grid;
    gap: var(--sp-1);
    margin-block: var(--sp-2);
  }
  .linkForm {
    display: flex;
    gap: var(--sp-1);
  }
  .linkForm input {
    flex: 1;
    min-width: 0;
  }
</style>
