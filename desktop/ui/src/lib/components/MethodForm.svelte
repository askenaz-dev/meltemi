<!-- SPDX-License-Identifier: Apache-2.0 -->
<script lang="ts">
  import { t } from "../i18n";
  import { METHOD_FORMS, type MethodField } from "../generated/method-forms";

  let {
    method,
    values,
    raw = $bindable(),
    mode = $bindable(),
  }: {
    method: string;
    values: Record<string, unknown>;
    raw: string;
    mode: "form" | "raw";
  } = $props();

  const form = $derived(METHOD_FORMS[method]);
  const fields = $derived<MethodField[]>(form?.fields ?? []);

  function update(field: MethodField, input: string | boolean) {
    if (field.kind === "boolean") {
      values[field.name] = Boolean(input);
    } else if (field.kind === "number") {
      const parsed = Number(input);
      values[field.name] = Number.isFinite(parsed) ? parsed : undefined;
    } else if (field.kind === "array") {
      const text = String(input).trim();
      values[field.name] = text
        ? text.split(",").map((part) => part.trim()).filter(Boolean)
        : [];
    } else {
      values[field.name] = String(input);
    }
    raw = JSON.stringify(values, null, 2);
  }

  function toText(value: unknown): string {
    if (value === undefined || value === null) return "";
    if (Array.isArray(value)) return value.join(", ");
    if (typeof value === "object") return JSON.stringify(value);
    return String(value);
  }
</script>

{#if !form}
  <p class="muted">{$t("palette.form.unavailable")}</p>
{:else if mode === "form"}
  <div class="fields">
    {#each fields as field (field.name)}
      <label class="field">
        <span class="name">
          {field.name}
          {#if field.required}
            <span class="req" title={$t("palette.form.required")}>*</span>
          {/if}
        </span>
        {#if field.kind === "boolean"}
          <input
            type="checkbox"
            checked={Boolean(values[field.name])}
            onchange={(event) => update(field, event.currentTarget.checked)}
          />
        {:else if field.options}
          <select
            value={toText(values[field.name])}
            onchange={(event) => update(field, event.currentTarget.value)}
          >
            <option value=""></option>
            {#each field.options as option (option)}
              <option value={String(option)}>{option}</option>
            {/each}
          </select>
        {:else}
          <input
            type={field.kind === "number" ? "number" : "text"}
            value={toText(values[field.name])}
            placeholder={field.kind === "array" ? "a, b, c" : ""}
            oninput={(event) => update(field, event.currentTarget.value)}
          />
        {/if}
      </label>
    {/each}
  </div>
  <p class="muted source">
    {$t("palette.form.source", { schema: form.schema, def: form.def })}
  </p>
{/if}

<style>
  .fields {
    display: grid;
    gap: var(--sp-2);
  }
  .field {
    display: grid;
    grid-template-columns: 130px 1fr;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-dense);
  }
  .name {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: var(--fs-caption);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .req {
    color: var(--warn);
  }
  input[type="checkbox"] {
    width: auto;
    justify-self: start;
  }
  .muted {
    margin: 0;
    color: var(--text-faint);
    font-size: var(--fs-caption);
  }
  .source {
    font-family: var(--font-mono);
  }
</style>
