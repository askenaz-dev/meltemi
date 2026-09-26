<!-- SPDX-License-Identifier: Apache-2.0 -->
# Local and self-hosted models

Meltemi keeps no catalog of models and runs no model of its own: its own engine
is a separate change that does not exist yet. Today, a model of your own
reaches Meltemi through an agent of your fleet that talks to local and
self-hosted providers by its own configuration — **OpenCode** (level 1,
launched as `opencode acp`; see [the agents guide](agentes.md#opencode--opencode)).
This page has three recipes for it.

**Verified: 2026-09-25, with OpenCode 1.18.32, on Windows 11.** Every recipe
below is verified **in form**: OpenCode accepted the file and listed the model
with `opencode models <provider>`. None is verified **live** yet — no model
server was running on the machine that checked them. The live check is the
local-provider leg of [the manual conformance run](conformidad-manual.md). A
new OpenCode version means checking again; the date above says when this was
last true, and nothing on this page is quoted from memory.

Recipe 1 was also driven through Meltemi the same day, with its profile and no
model server running. OpenCode announced `ollama/qwen2.5-coder:7b` as its
current model, among 80 on its list. `--model ollama/qwen2.5-coder:7b` was set
and confirmed before the first prompt; `--model ollama/no-such-model` was
refused before any prompt, with the announced list. The turn itself failed after
about 70 seconds with OpenCode's own «Cannot connect to API», and the session
ended in error — which is what a missing model server looks like.

## How a recipe reaches Meltemi

Either way needs no change to Meltemi:

1. **A file in the project.** OpenCode reads `opencode.json` from the project
   root, so `meltemi session "…" --agent opencode` in that project uses it.
2. **A named launch profile**, which keeps the file out of the repository and
   makes the recipe selectable by name, per session, from every surface.
   `OPENCODE_CONFIG` is OpenCode's own variable for a configuration file.

```toml
# .meltemi/config.toml
[[fleet.profile]]
name = "opencode-local"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/local-models.json" }
```

The value is a literal absolute path — a path is not a secret — or, when the
path differs from machine to machine, a **whole-value** reference such as
`"${OPENCODE_LOCAL_MODELS}"`, resolved from the environment the daemon started
in. A reference inside a longer value, like `"${HOME}/local-models.json"`, is
**not** expanded: write the whole path, or the whole reference.

## Choosing the model

The `model` in the file is the one OpenCode uses by default. To choose one per
session, use `--model` on the command line, the model chip in the desktop
composer, or `model` in the profile. OpenCode takes its model through an option
it announces, so Meltemi sets that option before the first prompt and confirms
it — or ends the session without a prompt, listing what OpenCode announced (see
[model and effort per session](agentes.md#model-and-effort-per-session)). Use
the provider-prefixed id OpenCode itself prints: `opencode models ollama` lists
`ollama/qwen2.5-coder:7b`.

## Recipe 1 — a model on your own machine (ollama)

Install the model server by its own instructions — on Windows,
`winget install Ollama.Ollama` — and pull a model, for instance
`ollama pull qwen2.5-coder:7b`. Then the OpenCode file:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "ollama/qwen2.5-coder:7b",
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Ollama (local)",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": { "qwen2.5-coder:7b": { "name": "Qwen2.5 Coder 7B" } }
    }
  }
}
```

And the profile that selects it:

```toml
[[fleet.profile]]
name = "opencode-ollama"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/ollama.json" }
```

Check: `opencode models ollama` prints `ollama/qwen2.5-coder:7b`.

## Recipe 2 — one endpoint for many providers (LiteLLM)

A LiteLLM proxy, run by its own documentation, serves many providers behind one
OpenAI-compatible endpoint. OpenCode talks to it like to any such endpoint; the
model names are whatever the proxy serves:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "litellm/gpt-5-mini",
  "provider": {
    "litellm": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LiteLLM proxy",
      "options": {
        "baseURL": "http://localhost:4000/v1",
        "apiKey": "{env:LITELLM_API_KEY}"
      },
      "models": { "gpt-5-mini": { "name": "gpt-5-mini via LiteLLM" } }
    }
  }
}
```

```toml
[[fleet.profile]]
name = "opencode-litellm"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/litellm.json" }
```

`{env:LITELLM_API_KEY}` is OpenCode's own substitution: the key lives in the
environment the daemon started in, never in a file. Setting it after the daemon
started means `meltemi stop` and starting again. Check: `opencode models
litellm` prints `litellm/gpt-5-mini`.

## Recipe 3 — hosted models through an aggregator (OpenRouter)

OpenCode knows OpenRouter's catalog, but does not offer it until the file
declares the provider — without it, `opencode models openrouter` answers
«Provider not found»:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "openrouter/moonshotai/kimi-k2",
  "provider": {
    "openrouter": {
      "options": { "apiKey": "{env:OPENROUTER_API_KEY}" },
      "models": { "moonshotai/kimi-k2": {} }
    }
  }
}
```

```toml
[[fleet.profile]]
name = "opencode-openrouter"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/openrouter.json" }
```

With the provider declared, `opencode models openrouter` lists the catalog —
385 models on the day of the check, `openrouter/moonshotai/kimi-k2` among them.
The key, `OPENROUTER_API_KEY`, lives in the daemon's environment, like the one
above; Meltemi never reads it (constitution §2).

## What is measured

Tokens are not carried by the protocol Meltemi pilots OpenCode with. What the
agent does report over it — the context it holds and its cumulative cost — the
usage view and `meltemi usage` show as the agent's own figures, never as
Meltemi's measurement. For a model on your own machine, the cost is whatever
OpenCode reports, if anything.

## What does not work today

- **Level-3 agents are not launched.** Aider and any other headless agent are
  refused by every verb (`level_not_pilotable`). If a level-3 runner ever
  exists, the provider prefix its CLI expects is yours to write: the daemon
  translates no provider names.
- **Meltemi's own engine** is a separate change and does not exist yet.
