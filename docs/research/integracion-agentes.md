# Research interno: superficie de integración de agentes del mercado

> **Documento de investigación interna** para implementar la capa de agentes (meltemi.md §7.5). Verificado contra documentación oficial el 2026-07-09/10; este mercado cambia por semanas — revalidar antes de implementar cada adaptador. No es material de marketing: aquí sí se nombran productos, porque sin nombres el documento no sirve para programar.

## Matriz de integración (julio 2026)

| Agente | ACP | Headless estructurado | Archivos de contexto | Nivel Meltemi |
|---|---|---|---|---|
| **Gemini CLI** (Google) | ✅ Nativo: `gemini --acp` (implementación de referencia del protocolo) | `gemini -p --output-format stream-json`; resume por UUID | `GEMINI.md` jerárquico; `context.fileName` remapeable a AGENTS.md | 1 |
| **GitHub Copilot CLI** | ✅ Nativo (preview): `copilot --acp --stdio` | `copilot -p --output-format json` (JSONL); `--no-ask-user` | Combina AGENTS.md + copilot-instructions + CLAUDE.md + GEMINI.md | 1 |
| **Cursor CLI** (`agent`) | ✅ Nativo: `agent acp` | `agent -p --output-format stream-json`; `-w` crea worktrees propios | `.cursor/rules` + AGENTS.md anidados | 1 |
| **Kiro CLI** (AWS) | ✅ Nativo: `kiro-cli acp` (extensiones `_kiro.dev/`) | `kiro-cli chat --no-interactive` — **requiere API key de pago** | `.kiro/steering/`, `.kiro/specs/` (formato de facto de spec-artifacts), AGENTS.md | 1 |
| **Kilo Code** | ✅ Nativo: `kilo acp` | `kilo run --auto --json` + servidor REST `kilo serve` (OpenAPI 3.1 + SSE) | AGENTS.md primera clase; lee `.claude/` y `.agents/` | 1 · **MIT: único embebible; agente de referencia** |
| **OpenCode** | ✅ Nativo: `opencode acp` | `opencode run --format json` + `opencode serve` (HTTP+SSE) | AGENTS.md (global+proyecto); CLAUDE.md como fallback | 1 |
| **Claude Code** (Anthropic) | Adapter del org ACP: `claude-agent-acp` (sobre Claude Agent SDK) | `claude -p --output-format stream-json`; `--permission-mode` | CLAUDE.md jerárquico; AGENTS.md vía import `@AGENTS.md` | 2 (ver ToS) |
| **Codex CLI** (OpenAI) | Adapter del org ACP: `codex-acp` (sobre `codex app-server`) | `codex exec --json`; `--output-schema`; sandbox `read-only\|workspace-write` | AGENTS.md jerárquico (estándar que ellos impulsan) | 2 |
| **Antigravity** (`agy`, Google) | ❌ Sin ACP (FR abierto, issue #31); adapters comunitarios frágiles | `agy -p` — **sin salida JSON** | GEMINI.md > AGENTS.md > `.agents/rules/` | 4 (artefactos) |

Registro público de agentes ACP (catálogo consumible): `https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`.

## Notas legales por proveedor (base del principio de juego limpio)

- **Anthropic**: prohíbe usar OAuth de suscripciones Free/Pro/Max en cualquier producto de terceros, incluido el Agent SDK (bloqueo técnico ene-2026, términos feb-2026, corte de facturación 4-abr-2026). **Camino seguro**: pilotar el binario oficial `claude` donde el usuario ya hizo login; jamás hablar su API directamente. El adapter ACP usa el SDK → zona gris: preferir headless del binario oficial hasta aclaración.
- **OpenAI**: postura opuesta — tolera y anima públicamente los harnesses de terceros con suscripción ChatGPT (programa "Codex for Open Source"). `codex app-server` es la interfaz JSON-RPC oficial que usa su propia extensión de VS Code.
- **GitHub**: el más amigable — la licencia de Copilot CLI permite explícitamente redistribuir el binario sin modificar dentro de otra app; Copilot SDK GA en 6 lenguajes (incluido Go) con BYOK.
- **AWS (Kiro)**: headless documentado y sancionado, pero requiere API key de tier de pago, por asiento.
- **Google (Antigravity)**: scripting del CLI oficial permitido; multiplexar cuentas de consumidor a escala es zona gris; camino enterprise el seguro.

**Regla derivada (constitución §2)**: ejecutar siempre el binario oficial con la auth que el propio agente gestiona; nunca suplantar tráfico ni tocar credenciales. El estatus por agente (sancionado / tolerado / gris) se muestra en el catálogo de flota sin maquillaje.

## Sandboxing por plataforma (para la capa de permisos heredados)

| Agente | macOS | Linux | Windows |
|---|---|---|---|
| Claude Code | Seatbelt | bubblewrap (WSL2) | **Sin sandbox nativo** (solo WSL2) |
| Codex CLI | Seatbelt | bubblewrap | **Sandbox nativo Windows** (elevated/unelevated, may-2026) |
| Gemini CLI | Seatbelt | Docker/Podman/bwrap | Parcial (tokens restringidos) o Docker |
| OpenCode | — | — | — (sin sandbox en ninguna plataforma; solo reglas de permisos) |

Implicación: "heredar el sandbox del agente" da niveles de protección muy desiguales → el catálogo de flota debe mostrar el nivel real por agente+plataforma, y los worktrees aislados son la frontera primaria en fase 1.

## Riesgo de mercado (censo de competidores, julio 2026)

Nicho llenándose por trimestres: attacca.ai ("Attacca Forge", spec-driven), Clave (codika — orquesta Claude Code/Codex/Gemini en escritorio), RondoFlow (orquestación visual de Claude Code), Timonel.ai (agent command center en español), Baton, Tutti, Vesper Code, AuricIDE (command center de agentes sobre Tauri), meridian-cli. Ninguno combina SDD unificado + neutralidad ACP + paridad TUI/GUI + Apache 2.0. La ventana de "primeros" se mide en meses.
