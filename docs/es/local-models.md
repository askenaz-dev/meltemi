<!-- SPDX-License-Identifier: Apache-2.0 -->
# Modelos locales y autohospedados

[English](../local-models.md)

Meltemi usa el modelo mediante un agente configurado para ese proveedor. Esta guía usa
**OpenCode**, un agente ACP nativo que se inicia como `opencode acp`; consulta
[agentes](agentes.md#opencode--opencode). Instala y configura por separado el servidor de modelos
o el proveedor remoto.

**Verificado: 2026-09-25, con OpenCode 1.18.32, en Windows 11.** Las recetas se comprobaron
**en su forma**: el agente aceptó la configuración y enumeró el modelo. No se comprobaron
**en vivo** con un servidor de modelos funcionando. Revisa la documentación de la versión de
OpenCode instalada antes de usar una receta: enumerar un modelo no demuestra que pueda completar
una petición ni pedir permisos.

Empieza con una petición pequeña e inspecciona el resultado antes de usar la configuración para
cambios reales.

## Cómo llega una receta a Meltemi

Ninguna de estas opciones exige modificar Meltemi:

1. **Archivo del proyecto.** OpenCode lee `opencode.json` en la raíz, por lo que
   `meltemi session "…" --agent opencode` dentro del proyecto lo usa.
2. **Perfil de ejecución con nombre.** Mantiene el archivo fuera del repositorio y permite elegir
   la receta por sesión desde cualquier superficie. `OPENCODE_CONFIG` es la variable de OpenCode
   para indicar su archivo de configuración.

```toml
# .meltemi/config.toml
[[fleet.profile]]
name = "opencode-local"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/local-models.json" }
```

El valor es una ruta absoluta literal o una referencia que ocupa **todo el valor**, como
`"${OPENCODE_LOCAL_MODELS}"`. Se resuelve con el entorno que tenía el daemon al arrancar.
Una referencia dentro de un valor más largo, como `"${HOME}/local-models.json"`, **no** se expande:
escribe la ruta completa o la referencia completa.

## Elegir el modelo

El campo `model` del archivo determina el modelo predeterminado de OpenCode. Para elegirlo por
sesión, usa `--model`, el control de modelo del compositor de escritorio o `model` en el perfil.
OpenCode lo expone mediante una opción de sesión: Meltemi la configura y confirma antes del primer
prompt, o termina sin enviarlo e informa qué opciones anunció el agente. Consulta
[modelo y esfuerzo por sesión](agentes.md).

Usa el identificador con prefijo del proveedor que muestra el propio OpenCode:
`opencode models ollama` enumera `ollama/qwen2.5-coder:7b` en esta receta.

## Receta 1: un modelo en tu equipo (Ollama)

Instala el servidor siguiendo sus instrucciones; por ejemplo, en Windows,
`winget install Ollama.Ollama`. Descarga un modelo como `ollama pull qwen2.5-coder:7b`.
El archivo de OpenCode es:

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

El perfil que lo selecciona:

```toml
[[fleet.profile]]
name = "opencode-ollama"
agent = "opencode"
env = { OPENCODE_CONFIG = "C:/Users/you/.config/opencode/ollama.json" }
```

Comprueba que `opencode models ollama` muestra `ollama/qwen2.5-coder:7b`.

## Receta 2: un endpoint para varios proveedores (LiteLLM)

Un proxy LiteLLM configurado con sus propias instrucciones expone varios proveedores mediante un
endpoint compatible con OpenAI. OpenCode usa los nombres de modelo que sirva ese proxy:

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

`{env:LITELLM_API_KEY}` es la sustitución del propio OpenCode: la clave está en el entorno con que
arrancó el daemon, nunca en el archivo. Si cambias ese entorno después, detén el daemon con
`meltemi stop` e inícialo de nuevo. Comprueba que `opencode models litellm` muestra
`litellm/gpt-5-mini`.

## Receta 3: modelos alojados mediante un agregador (OpenRouter)

OpenCode conoce el catálogo de OpenRouter cuando el archivo declara el proveedor; sin esa
configuración, `opencode models openrouter` puede responder «Provider not found»:

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

Con el proveedor declarado, `opencode models openrouter` enumera los modelos que reconoce el
agente instalado. Elige un identificador que tu proveedor sirva actualmente.
`OPENROUTER_API_KEY` queda en el entorno del daemon igual que la clave anterior; Meltemi no lee
las credenciales del agente.

## Qué se mide

El protocolo usado para pilotar OpenCode no entrega contadores de tokens de entrada y salida.
La vista de consumo y `meltemi usage` muestran el contexto y coste acumulado que el agente
comunica como cifras del agente, no como mediciones de Meltemi. Para un modelo local, el coste
será lo que informe OpenCode, si informa alguno.

## Límites actuales

- **No se ejecutan agentes de nivel 3.** Aider y los demás agentes de ese nivel se rechazan con
  `level_not_pilotable`. El daemon no traduce nombres de proveedores.
- **La invocación directa de modelos desde Meltemi no está disponible**: un agente externo
  proporciona la conexión al modelo.
