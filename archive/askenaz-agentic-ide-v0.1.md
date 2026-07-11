# Askenaz AIDE — Documento Fundacional de Visión y Especificación de Producto

> *"La especificación antes que el código. Para todo el mundo mundial, en beneficio de la humanidad."*
> Versión 0.1 (borrador fundacional) — 9 de julio de 2026
> Este documento practica lo que predica: define **qué** se construirá antes de escribir una sola línea de código.

---

## TL;DR

- **Askenaz AIDE será un IDE agéntico de código totalmente abierto (Apache 2.0) cuyo núcleo es el Spec-Driven Development (SDD): ninguna línea de código sin una especificación revisada primero, con paridad real entre la terminal (CLI/TUI) y el escritorio (GUI).** Su diferenciador frente a Kiro y Orca es unificar el mejor SDD del mercado (Kiro + spec-kit + OpenSpec) sobre una arquitectura abierta, model-agnostic y sin lock-in de proveedor.
- **Recomendación técnica firme (no una lista de opciones): construir una aplicación INDEPENDIENTE, NO un fork de VS Code.** El motor de agente y de specs vive en un **núcleo en Go** ejecutado como daemon headless; encima corren dos clientes finos: un **TUI en Go + Bubble Tea** y un **GUI de escritorio en Tauri (NO Electron)**, comunicándose con el núcleo por JSON-RPC. Se adoptan **MCP** (para herramientas/datos) y **ACP** (para desacoplar cliente y agente) como protocolos abiertos de primera clase.
- **Licencia: Apache 2.0** (permisiva + concesión de patentes) para maximizar adopción institucional, con posible copyleft (AGPL) reservado solo para futuros componentes de servidor/nube. La estrategia de crecimiento imita a los proyectos abiertos ganadores (opencode, Aider, Zed): distribución de binario único, BYOK (bring-your-own-key) y comunidad en GitHub/Discord.

---

## 1. Visión y Misión

### El porqué

El desarrollo asistido por IA vive una tensión: los agentes generan código rápido pero, sin una fase de requisitos y diseño explícita, producen software frágil, no documentado y desalineado con la intención real —el patrón que la industria llama *"vibe coding"*. Kiro (AWS) lo describe como el síntoma de saltarse la fase de requisitos; GitHub, con spec-kit, lo resume así: *"We're moving from 'code is the source of truth' to 'intent is the source of truth.'"*

La respuesta de la industria —Spec-Driven Development— está fragmentada entre herramientas potentes pero cerradas o con lock-in (Kiro depende de Amazon Bedrock y modelos Claude/Nova), herramientas ligeras pero solo de metodología (spec-kit, OpenSpec, que dependen de un agente externo), y entornos de orquestación agéntica excelentes pero sin disciplina de specs (Orca).

**La misión de Askenaz AIDE es democratizar el desarrollo agéntico con disciplina de ingeniería para toda la humanidad**, mediante:

1. **Apertura radical**: código 100% abierto en GitHub, sin lock-in de proveedor de IA ni de nube.
2. **Disciplina spec-first como valor por defecto**, no como opción avanzada.
3. **Paridad terminal + escritorio**: el mismo poder desde un servidor SSH headless o desde una GUI pulida.
4. **Neutralidad de modelo**: Claude, GPT, Gemini, o modelos locales vía Ollama, con la misma experiencia.

### Nombre y posicionamiento

*Askenaz AIDE* (Agentic IDE for Agentic Development). "AIDE" refuerza tanto "IDE agéntico" como la idea de *ayuda* (aide, en francés/inglés) al desarrollador. Posicionamiento: **"El IDE agéntico abierto donde la especificación es la unidad de trabajo."**

---

## 2. Análisis del Estado del Arte

### 2.1 Kiro (AWS) — el pionero del spec-as-artifact

**Qué es.** Kiro es un IDE agéntico lanzado por AWS en public preview en **julio de 2025** y llevado a disponibilidad general con free tier en **marzo de 2026**. Está construido sobre **Code OSS** (la misma base abierta MIT de VS Code) y funciona sobre **Amazon Bedrock**, enrutando entre modelos **Claude Sonnet y Amazon Nova**. Desde mediados de 2026 corre en tres superficies: IDE de escritorio, CLI y web (Kiro Web). AWS ha confirmado que Kiro es el sucesor de Amazon Q Developer (registros nuevos de Q bloqueados desde el 15 de mayo de 2026).

**Flujo spec-driven.** Al iniciar una funcionalidad, Kiro genera tres artefactos en `.kiro/specs/<feature>/`:
- `requirements.md` — historias de usuario y criterios de aceptación en **notación EARS** (Easy Approach to Requirements Syntax). EARS fue desarrollada por Alistair Mavin y colegas en Rolls-Royce (presentada en la conferencia RE'09, IEEE, 2009) y estructura los requisitos con un pequeño conjunto de patrones: ubicuo ("The system shall…"), state-driven ("**While** <estado>, the system shall…"), event-driven ("**When** <trigger>, the system shall…"), unwanted behavior ("**If** <condición>, then the system shall…") y optional ("**Where** <feature>, the system shall…").
- `design.md` — arquitectura del sistema, componentes, modelos de datos, flujo de datos.
- `tasks.md` — lista de tareas secuenciadas por dependencias.

**Steering y hooks.** Los *steering files* (`.kiro/steering/*.md`, con foundation files `product.md`, `tech.md`, `structure.md`) dan contexto persistente; admiten modos de inclusión (always/conditional por `fileMatch`/manual) vía front-matter YAML. Los *agent hooks* son automatizaciones disparadas por eventos de archivo (guardar/crear/borrar) para generar tests, actualizar docs o escanear credenciales. Hay dos modos de autonomía: **Autopilot** (edita el workspace de forma autónoma, con reversión posible) y **Supervised** (aprobación por *hunks* antes de aplicar). Soporta MCP y "Kiro Powers" (marketplace de servidores MCP orientados a AWS: CDK, CloudFormation, pricing).

**Fortalezas.** El modelo spec-as-artifact es genuinamente novedoso: los artefactos estructurados sobreviven a la sesión de chat y dan a futuros agentes mucho más contexto que un historial de commits. Trazabilidad, disciplina de TDD (desde mayo de 2026 el agente escribe tests que fallan a partir de la spec, luego implementa), y gobernanza para uso empresarial.

**Debilidades y crítica documentada.**
- **Over-engineering**: reportes de 20 archivos y 1.500 líneas para algo que necesitaba 200; genera tests obsesivamente. Martin Fowler describió el flujo como *"usar un mazo para cascar una nuez"* en un bug pequeño (4 historias de usuario y 16 criterios de aceptación para un bugfix trivial).
- **Precios controvertidos**: tras el lanzamiento, AWS pasó a un sistema de créditos que provocó fuerte crítica (*The Register*: "a wallet-wrecking tragedy"). Los tiers actuales según kiro.dev/pricing: Free (50 créditos), Pro $20 (1.000), Pro+ $40 (2.000), Power $200 (10.000), con **excedente a $0,04 por crédito**. Existe además una "spec tax": el spec-mode consume créditos a mayor tasa que el vibe-mode. Los créditos no se acumulan y las suscripciones son individuales, no compartidas.
- **Lock-in a AWS/Bedrock**: fuera del ecosistema AWS, buena parte del valor de integración se vuelve overhead; el modelo está limitado a Claude/Nova vía Bedrock.
- **Fiabilidad**: tareas que se atascan, fallan y pierden contexto al reintentar (consumiendo presupuesto). El repositorio público reflejaba miles de issues abiertos, señal de fricción en producción. Un incidente citado (5 de marzo de 2026) atribuyó a código desplegado sin revisión por un agente Kiro una caída de 6 horas —recordatorio de que la autonomía necesita controles de permisos estrictos.

### 2.2 Orca (stablyai) — el ADE de agentes paralelos

**Qué es.** Orca se autodefine como **ADE (Agent Development Environment)**, no IDE: un entorno de escritorio (macOS/Windows/Linux) con app móvil (iOS/Android) para correr **múltiples agentes CLI en paralelo**, cada uno en su propio **git worktree** aislado. Es **open source bajo licencia MIT**, con release reciente (v1.4.x en junio de 2026) y varios miles de estrellas en GitHub.

**Qué lo hace único.**
- **Worktree-native**: cada tarea/feature obtiene su propio worktree, terminal de agente y pestaña de navegador; se pueden correr 3 agentes sobre el mismo bug y elegir el ganador ("race three agents").
- **BYO-agent y BYO-subscription**: no requiere login propio; soporta más de 30 agentes CLI (Claude Code, Codex, Gemini, Cursor CLI, OpenCode, Amp, Cline, Kiro, Qwen, etc.) usando tus propias suscripciones.
- **Design Mode**: navegador embebido; haces clic en un elemento de la UI y se inyecta como contexto en el prompt del agente.
- **SSH, notificaciones, hot-swap de cuentas Codex, source control con revisión de diffs línea a línea, y una Orca CLI** para que los agentes controlen el propio IDE.

**Fortalezas.** Es el primer entorno que trata "múltiples agentes en paralelo" como flujo por defecto, no como feature avanzada. Excelente UX de revisión de diffs y orquestación multi-agente.

**Debilidades (para nuestro propósito).** No es spec-driven: no impone ni genera una fase de requisitos/diseño; es un *orquestador* de agentes de terceros más que un motor propio. Depende de agentes CLI externos, y su UX es exclusivamente de escritorio/móvil (sin una TUI de primera clase). Es la inspiración clave para nuestra capa de **orquestación multi-agente y revisión de diffs**, pero no para el flujo de specs.

### 2.3 GitHub spec-kit — metodología SDD portátil

**Qué es.** Toolkit open source (MIT) que operacionaliza SDD para agentes de codificación. Se distribuye como CLI (`specify init`), funciona con 30+ agentes (Copilot, Claude Code, Gemini CLI, Codex, Cursor, Zed, Kiro…) e instala comandos slash. Flujo en cuatro fases con checkpoints humanos: **Constitution → /specify → /plan → /tasks → /implement**.

**Conceptos clave.** El `constitution.md` establece principios "no negociables" del proyecto (p. ej. "toda app debe ser CLI-first", políticas de testing) que se inyectan en cada fase. Cada fase produce un artefacto Markdown que alimenta a la siguiente. Añade análisis de consistencia entre artefactos y checklists de calidad ("unit tests for English").

**Fortalezas.** Máxima portabilidad y personalización (todos los artefactos en el workspace); agent-agnostic; human-in-the-loop con aprobación explícita por fase; extensiones, presets y bundles por rol.

**Limitaciones.** Pesado: genera muchos archivos Markdown repetitivos y tediosos de revisar (crítica recurrente de Martin Fowler). Rígidas *phase gates*. Setup en Python. Fuerte para greenfield (0→1), débil para actualizaciones que cruzan varias specs. Y el propio agente a veces no sigue todas las instrucciones pese a la abundancia de plantillas.

### 2.4 OpenSpec (Fission AI) — SDD ligero y brownfield-first

**Qué es.** Framework SDD open source, deliberadamente **ligero y brownfield-first** (pensado para bases de código maduras, no solo greenfield). Funciona con 20-30+ asistentes vía slash commands (`/opsx:propose`, `/opsx:apply`, `/opsx:archive`). Requiere Node.js 20.19+.

**Estructura de artefactos.** Separa dos carpetas: `openspec/specs/` (la **verdad viva** de cómo funciona hoy el sistema) y `openspec/changes/` (propuestas). Cada cambio es una carpeta con `proposal.md`, `specs/` (deltas), `design.md` y `tasks.md`. La innovación central son los **delta specs**: en lugar de reescribir toda la spec, se marcan secciones como `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`. Al **archivar** un cambio, solo el delta se funde ("merge") en la spec fuente de verdad; propuesta, diseño y tareas eran andamiaje.

**Fortalezas.** Iteración fluida sin *phase gates* rígidas; specs viven junto al código (contexto que sobrevive a la sesión); ideal para 1→n (modificar comportamiento existente) donde spec-kit y Kiro son más débiles. Soporta schemas personalizados (p. ej. `research-first`, `spec-driven-with-adr` para conservar ADRs).

**Limitaciones.** Requiere disciplina humana ("solo funciona si realmente lees y piensas las specs"); no es un IDE ni un agente, solo la capa de specs; funcionalidades de equipo/multi-repo aún emergentes.

### 2.5 Panorama más amplio (agentes CLI e IDEs) — qué robar de cada uno

| Herramienta | Qué hace excelente | Lección para Askenaz |
|---|---|---|
| **opencode / Crush** | Binario único en Go; TUI pulida con **Bubble Tea** (arquitectura Elm); model-agnostic (75+ proveedores); switch de modelo mid-sesión; LSP; sesiones SQLite | El stack Go + Bubble Tea + binario único es el modelo de referencia para nuestra TUI |
| **Claude Code** | Bucle agéntico profundo, plan mode, hooks, subagents, project memory; definió la forma del sector | Plan/act modes, subagents, memoria de proyecto |
| **Codex CLI (OpenAI)** | Reescrito en **Rust**, Apache-2.0; **sandboxing por defecto** (shell en contenedor, `--full-auto` opcional) | Sandbox y aprobación por defecto como modelo de seguridad |
| **Aider** | Git-native: **un commit atómico por cambio de IA**; repo map; voz | Disciplina git de primera clase (commit por tarea) |
| **Cline / Roo Code** | Model-agnostic; SDK Apache-2.0 extraído como infraestructura; modos plan/act | SDK/plugin reutilizable como infraestructura |
| **Zed** | Editor nativo en Rust (GPUI), altísimo rendimiento; **creador de ACP** | Rendimiento nativo; adopción de ACP |
| **Warp / Amp** | Terminal agéntica; "deep mode" de investigación autónoma | Modo de investigación extendida |
| **Goose** | Agente MCP-purista, ahora bajo la Linux Foundation | MCP como ciudadano de primera clase |

**Patrones convergentes de 2025-2026** que Askenaz debe incorporar: **MCP** para herramientas externas, **skills/plugins** para comportamiento, **checkpoints/rollback**, **paralelismo con git worktrees**, **sandboxing con aprobación**, **BYOK** y **distribución de binario único**.

---

## 3. Propuesta de Valor y Diferenciación

Askenaz AIDE es el primer entorno que combina, en un solo producto abierto:

1. **SDD como núcleo unificado** que toma lo mejor de los tres enfoques: la tríada legible de Kiro (requirements/design/tasks con EARS), la `constitution` y los slash commands de spec-kit, y los **delta specs + separación verdad/propuestas** de OpenSpec (brownfield-first). Nadie más los une.
2. **Paridad total terminal ↔ escritorio**, gracias a un motor núcleo compartido (a diferencia de Kiro, cuyo CLI y GUI son superficies separadas de un producto propietario).
3. **Neutralidad de modelo y de nube real** (a diferencia de Kiro/Bedrock): Anthropic, OpenAI, Gemini, OpenRouter y modelos locales vía Ollama, con switch mid-sesión.
4. **Orquestación multi-agente estilo Orca** (worktrees paralelos, revisión de diffs línea a línea) pero **gobernada por specs**.
5. **Apertura y sostenibilidad**: Apache 2.0, BYOK, sin métrica de créditos opaca ni "spec tax".
6. **Protocolos abiertos de primera clase**: **MCP** (herramientas/datos) y **ACP** (cliente↔agente), evitando reinventar integraciones y permitiendo que Askenaz sea, a la vez, cliente ACP y host MCP.

---

## 4. Principios de Diseño

1. **Spec-first (no code without spec)**: el flujo por defecto exige una spec aprobada antes de implementar. Debe existir una vía ligera ("fast-forward"/vibe-mode) para bugs triviales, aprendiendo del error de Kiro de aplicar el mazo a todo.
2. **Paridad terminal + escritorio**: toda capacidad del núcleo está disponible por igual en TUI y GUI; nada es exclusivo de una superficie.
3. **Open source, abierto de verdad**: sin lock-in de proveedor, sin marketplace cerrado. Extensiones y MCP servers desde registros abiertos.
4. **Model-agnostic**: la lógica del agente no asume un proveedor. BYOK primero.
5. **Extensible**: sistema de plugins/skills, MCP y ACP; el núcleo es una librería/SDK, no un monolito.
6. **Seguridad por defecto**: sandbox de ejecución y aprobación explícita de comandos peligrosos; principio de menor privilegio para acceso a repos y nube.
7. **Trazabilidad**: cada cambio de código es rastreable hasta la spec y el requisito que lo originó.
8. **Legibilidad de artefactos**: excelente UX de revisión de specs (el punto débil unánime de spec-kit y Kiro).

---

## 5. El Flujo Spec-Driven de Askenaz

### 5.1 Estructura de artefactos: el directorio `.askenaz/`

```
.askenaz/
├── constitution.md            # Principios no negociables (de spec-kit)
├── steering/                   # Contexto persistente (de Kiro)
│   ├── product.md             #  el "porqué" del producto
│   ├── tech.md                #  stack y restricciones técnicas
│   ├── structure.md           #  organización y convenciones
│   └── *.md                   #  con front-matter: always | fileMatch | manual
├── specs/                      # VERDAD VIVA (de OpenSpec): cómo funciona hoy
│   └── <capability>/
│       └── spec.md            #  requisitos vigentes en EARS
├── changes/                    # PROPUESTAS (de OpenSpec + tríada de Kiro)
│   └── <change-name>/
│       ├── proposal.md        #  por qué y qué cambia
│       ├── requirements.md    #  historias + criterios de aceptación (EARS)
│       ├── design.md          #  arquitectura, modelos de datos, interfaces
│       ├── specs/             #  DELTAS: ## ADDED / ## MODIFIED / ## REMOVED
│       └── tasks.md           #  tareas secuenciadas por dependencias
├── changes/archive/           # cambios completados (histórico + ADRs opcionales)
└── hooks/                      # automatizaciones por evento (de Kiro)
```

### 5.2 El ciclo de vida (comandos slash unificados)

```
/constitution   → establece o edita los principios del proyecto
/explore        → socio de pensamiento sin compromiso (de OpenSpec): lee el código,
                  sopesa opciones, propone un plan antes de escribir nada
/propose <idea> → crea changes/<name>/ con proposal, requirements (EARS),
                  design, deltas y tasks. Human-in-the-loop.
/review         → UX de revisión de specs de primera clase (diff de deltas,
                  detección de contradicciones y huecos, checklist de calidad)
/plan           → refina design.md y secuencia tasks.md
/implement      → ejecuta tareas en modo plan/act, con checkpoints
/verify         → valida la implementación contra la spec fuente de verdad
/archive        → funde los deltas aprobados en specs/ y preserva el histórico
```

**Notación EARS obligatoria** en requisitos y deltas, para eliminar ambigüedad. **Delta-based** para brownfield: los cambios describen lo que cambia, no reescriben toda la spec. **Constitution + steering** inyectados como contexto en cada fase. **Modo dual**: `spec-full` (disciplina completa) y `fast-forward` (genera todos los artefactos de una vez para cambios pequeños), evitando la fricción que hundió la reputación de Kiro en tareas triviales.

---

## 6. Funcionalidades Clave

1. **Editor de specs** con vista Markdown enriquecida (estilo Notion/Obsidian, como el que Orca añadió para revisar specs), diff de deltas ADDED/MODIFIED/REMOVED, validación EARS en vivo y detección de contradicciones/huecos.
2. **Ejecución de agente con modos Plan/Act** (inspirado en Cline/Claude Code) y **Supervised/Autopilot** (de Kiro): Supervised aprueba por *hunks*; Autopilot ejecuta con guardarraíles (comandos de confianza permitidos, resto denegado), acotado al workspace.
3. **Hooks** por evento (guardar/crear/borrar/manual) para tests, docs, escaneo de secretos y commits convencionales.
4. **Steering files** con modos de inclusión (always/fileMatch/manual) y scope workspace o global.
5. **Checkpoints y rollback**: snapshots antes de cada tarea; reversión granular.
6. **Orquestación multi-agente** con **git worktrees** aislados (de Orca): correr varios agentes/modelos en paralelo sobre la misma tarea y elegir/mezclar el mejor resultado.
7. **Integración MCP de primera clase**: cliente MCP para conectar herramientas/datos (tools, resources, prompts) sobre JSON-RPC 2.0 (stdio o Streamable HTTP); registro abierto de servidores.
8. **Gestión de contexto**: repo map, indexado de codebase, referencias `#file`/`#folder`, auto-compactación de contexto.
9. **Integración Git**: commit atómico por tarea (disciplina de Aider), revisión de diffs línea a línea con comentarios que vuelven al agente (de Orca).
10. **UX de revisión/diff** como ciudadano de primera clase, tanto en TUI como en GUI.
11. **Sandbox de ejecución** con aprobación por defecto de comandos peligrosos (modelo de Codex CLI).

---

## 7. Arquitectura Técnica Recomendada

### 7.1 Decisión 1 — Independiente vs. fork de VS Code: **INDEPENDIENTE**

**Recomendación: construir una aplicación independiente, NO un fork de Code OSS.** Razones basadas en la investigación:

- **Carga de mantenimiento del fork**: seguir el ritmo de upstream de VS Code es un coste permanente que absorbe recursos que deberían ir a la lógica de specs/agente.
- **Trampa del marketplace de Microsoft**: los términos del Visual Studio Marketplace **prohíben** que forks o productos derivados de Code-OSS accedan a él ("alternative products including those built on a fork of the Code-OSS Repository, are not permitted to access the Visual Studio Marketplace"). Microsoft ya lo ha aplicado técnicamente: en abril de 2025 su extensión C/C++ (v1.24.5) dejó de funcionar en VSCodium y Cursor. Un fork nos ata a **Open VSX** (Eclipse Foundation), que tiene muchas menos extensiones, ha sufrido malware (extensiones maliciosas con millones de instalaciones) e introdujo tiers de pago en 2026 para organizaciones con alto tráfico. Kiro y Cursor viven precisamente con esta limitación.
- **Rendimiento y coherencia**: partir de Electron/Code OSS hereda el peso de Chromium. Zed demostró que un editor nativo (Rust/GPUI) es dramáticamente más rápido, y que forkear no es requisito para el éxito. opencode/Crush demostraron que un binario Go de ~5MB puede desafiar el supuesto de que "se necesita un IDE para programar con IA".
- **Excepción evaluada y descartada**: solo forkearíamos si el objetivo primario fuera reutilizar el ecosistema de 90.000+ extensiones de VS Code. Nuestro objetivo primario es SDD + paridad terminal/escritorio + neutralidad, que un fork no facilita y sí encarece.

Askenaz será independiente, integrando el protocolo LSP directamente para inteligencia de código (como hace opencode) sin depender del marketplace de Microsoft.

### 7.2 Decisión 2 — Arquitectura núcleo + clientes finos

**Patrón recomendado: motor núcleo headless (daemon) + clientes finos vía JSON-RPC**, siguiendo el modelo probado de daemons con CLI/GUI (Bitcoin Core `bitcoind`/`bitcoin-qt`, Transmission, Erigon `rpcdaemon`, source{d} `srcd`/`srcd-server`):

```
┌───────────────────────────────────────────────────────────┐
│  CLIENTES FINOS (thin clients)                             │
│  ┌───────────────────┐        ┌────────────────────────┐  │
│  │  TUI (Go +        │        │  GUI escritorio         │  │
│  │  Bubble Tea)      │        │  (Tauri: Rust + webview)│  │
│  └─────────┬─────────┘        └───────────┬────────────┘  │
│            │      JSON-RPC 2.0 (stdio /    │               │
│            │      unix socket / HTTP)      │               │
├────────────┴──────────────────────────────┴───────────────┤
│  askenazd — NÚCLEO HEADLESS (Go)                           │
│  • Motor de specs (constitution, deltas, EARS, ciclo)     │
│  • Motor de agente (plan/act, checkpoints, orquestación)  │
│  • Capa de abstracción de proveedores LLM                 │
│  • Cliente MCP + host ACP  • Git/worktrees  • Sandbox     │
└───────────────────────────────────────────────────────────┘
```

El daemon `askenazd` concentra toda la lógica. La CLI lo arranca on-demand si no está corriendo (como `srcd`). Beneficios: **paridad garantizada** (ambos clientes consumen la misma API), soporte **headless/SSH** trivial (el daemon corre en un servidor y la TUI/GUI se conectan remotamente, como el `orca serve`), y capacidad de exponer el mismo núcleo a integraciones futuras.

### 7.3 Decisión 3 — Terminal: **Go + Bubble Tea** (Charm)

**Recomendación: Go con Bubble Tea** para la TUI y el propio núcleo. Justificación:
- Es exactamente el stack de los agentes CLI open source más exitosos: **opencode/Crush** (SST/Charm) usan Go + Bubble Tea, con **binario único** (`curl | bash`, sin npm ni venv) —una ventaja de experiencia enorme en herramientas de desarrollador.
- Bubble Tea sigue la **arquitectura Elm** (Model → Update → View), que da gestión de estado predecible, crítica para una TUI que maneja input, streaming del LLM y resultados de herramientas simultáneamente.
- Go ofrece concurrencia excelente (goroutines) para orquestar múltiples agentes y streams, cross-compilación sencilla y despliegue de binario único.

**Alternativa considerada (Rust + Ratatui):** Ratatui rinde ~30-40% menos memoria y ~15% menos CPU que Bubble Tea en dashboards de alta frecuencia, y es lo que usa Codex CLI. Pero su modelo *immediate-mode* obliga a construir a mano el bucle de eventos y la gestión de estado; la velocidad de desarrollo de Go+Bubble Tea gana para nuestro caso (una TUI de agente, no un dashboard de 1.000 puntos/seg). Al elegir Go para el núcleo, la TUI en Go comparte tipos y lógica sin FFI.

### 7.4 Decisión 4 — Escritorio: **Tauri**, no Electron

**Recomendación firme: Tauri.** La evidencia es contundente:

| Métrica | Electron | Tauri |
|---|---|---|
| Tamaño instalador | 80-150 MB (bundle de Chromium + Node) | <10 MB (a menudo ~2,5-5 MB) |
| Memoria en idle | 150-300 MB | 30-50 MB |
| Arranque | 1-2 s | <0,5 s |
| Backend | Node.js | Rust (memory-safe) |
| Seguridad | acceso total a Node por defecto | capacidades denegadas por defecto |
| Móvil | no | sí (Tauri 2.x: iOS/Android) |

Casos reales: Hoppscotch migró de Electron a Tauri reduciendo el bundle de 165 MB a 8 MB y la memoria un 70%. AppFlowy eligió Tauri sobre Electron precisamente para diferenciarse en rendimiento. La regla práctica de 2026 en la comunidad: *"start a new app in Tauri v2 unless you have a specific reason not to."*

**Contrapartida honesta y mitigación:** Tauri usa el **WebView nativo** de cada SO (WebView2/Chromium en Windows, WebKitGTK en Linux, WebKit en macOS), lo que produce **inconsistencias de render** (Safari/WebKit suele ir un paso por detrás; falta de prefijos `-webkit`). Mitigación: usar un framework web con buen soporte cross-webview, incluir polyfills, y **testear en los tres SO**. Como nuestro backend pesado vive en el daemon Go (no en el proceso del GUI), el requisito de escribir mucho Rust en Tauri es mínimo: Tauri actúa sobre todo como shell nativo ligero que habla JSON-RPC con `askenazd`. Electron solo sería preferible si necesitáramos render idéntico pixel-perfect en todas las plataformas o dependiéramos fuertemente del ecosistema Node —no es nuestro caso.

### 7.5 Decisión 5 — Abstracción de proveedores + MCP + ACP

- **Capa de abstracción LLM**: interfaz única con adaptadores para Anthropic (Claude), OpenAI (GPT/Codex), Google (Gemini), OpenRouter y **modelos locales vía Ollama**, con switch de modelo mid-sesión (como Crush) y BYOK.
- **MCP (Model Context Protocol)**: lanzado por Anthropic a finales de 2024 y donado a la **Agentic AI Foundation** (Linux Foundation) en diciembre de 2025. Es el estándar abierto tipo "USB-C para IA" que conecta agentes con herramientas/datos vía **JSON-RPC 2.0** (stdio o Streamable HTTP), con tres primitivas de servidor (**tools, resources, prompts**) y handshake de capacidades. Askenaz será **host/cliente MCP** de primera clase. Nota de seguridad: MCP tiene riesgos documentados (prompt injection, tools envenenados); implementaremos consentimiento, revisión de descripciones de tools y control de acceso.
- **ACP (Agent Client Protocol)**: creado por **Zed Industries** y lanzado en **agosto de 2025** bajo **licencia Apache 2.0**, usa **JSON-RPC 2.0 sobre stdin/stdout** (el editor arranca el agente como subproceso on-demand). Su propósito, en palabras del CEO de Zed, Nathan Sobo: *"Just as the Language Server Protocol unbundled language intelligence from monolithic IDEs, our goal with the Agent Client Protocol is to enable you to switch between multiple agents without switching your editor."* ACP y MCP son **complementarios, no competidores**: *"MCP connects an agent to tools and data; ACP connects an editor to an agent"* — un agente es servidor ACP hacia el editor y cliente MCP hacia sus herramientas al mismo tiempo. Ya lo soportan Zed, JetBrains, Gemini CLI, Goose, y vía adaptadores Claude Code, Codex y OpenCode (repo oficial: `github.com/zed-industries/agent-client-protocol`; sitio `agentclientprotocol.com`). **Adoptar ACP permite que Askenaz sea tanto un cliente que puede pilotar agentes externos como un agente pilotable desde otros editores** — un multiplicador de interoperabilidad que ni Kiro ni Orca ofrecen de forma estándar.
- **Sistema de plugins/extensiones**: núcleo como SDK (modelo Cline), skills declarativas (modelo Pi: descripción de una línea, carga perezosa del schema completo), y servidores MCP como mecanismo primario de extensión.

---

## 8. Estrategia Open Source

### 8.1 Licencia: **Apache 2.0**

**Recomendación: Apache 2.0** para el núcleo, la CLI/TUI y el GUI. Razones:
- Permisiva (máxima adopción, incluida la institucional: los equipos legales de grandes empresas aprueban MIT/Apache casi por defecto y muchos **vetan AGPL**).
- Añade **concesión y protección de patentes** que MIT no tiene —relevante para un proyecto que recibirá contribuciones externas y podría ser usado por grandes empresas. Es la elección de Google, CNCF, Codex CLI, Gemini CLI y ACP.
- Alternativas evaluadas: **MIT** (usada por opencode y Orca) es válida pero carece de cláusula de patentes; **AGPL** (usada por Warp para el cliente) protege contra que hiperescaladores exploten el SaaS sin contribuir, pero **congela la adopción empresarial**. Estrategia recomendada, imitando a **Zed** (GPL editor / AGPL servidor / Apache framework): mantener el corazón en Apache 2.0 y **reservar AGPL solo para futuros componentes de servidor/nube** si se ofrece un servicio gestionado.

### 8.2 Estructura de repositorio (monorepo)

```
askenaz-aide/                    (Apache-2.0)
├── core/          # askenazd: motor Go (specs, agente, providers, MCP/ACP)
├── cli/           # cliente TUI (Go + Bubble Tea)
├── desktop/       # cliente GUI (Tauri)
├── sdk/           # SDK público para integradores/plugins
├── proto/         # esquemas JSON-RPC compartidos
├── docs/          # documentación (practicando SDD sobre sí mismo)
├── .askenaz/      # el propio proyecto se desarrolla con Askenaz (dogfooding)
├── CONTRIBUTING.md, GOVERNANCE.md, CODE_OF_CONDUCT.md, SECURITY.md
```

### 8.3 Comunidad y gobernanza

- **Dogfooding**: Askenaz se desarrolla a sí mismo con SDD (el `.askenaz/` del propio repo es la mejor demo).
- **Distribución** estilo herramienta ganadora: binario único (`curl | bash`, Homebrew, AUR, releases de GitHub), sin dependencias pesadas.
- **BYOK** desde el día uno: gratis como herramienta; el usuario paga solo los tokens de su proveedor.
- **Gobernanza abierta**: contribuciones vía propuestas de cambio SDD (como OpenSpec exige a sus propios contribuidores); CLA para preservar la opción de dual-licensing futuro; roadmap público (modelo Zed).
- **Canales**: GitHub Discussions + Discord; "fireside hacks"/desarrollo en vivo (modelo Zed).

---

## 9. Roadmap por Fases

### Fase 0 — Fundación (mes 0-1)
- Este documento fundacional + constitution del proyecto. Definir esquemas JSON-RPC (`proto/`). Prototipo del daemon `askenazd` con un proveedor (Anthropic) y BYOK.
- **Hito**: `askenazd` responde a un `/propose` mínimo por JSON-RPC.

### Fase 1 — MVP (mes 2-4)
- Motor de specs completo (`.askenaz/`, EARS, delta specs, ciclo constitution→propose→implement→archive).
- TUI en Go + Bubble Tea con paridad de flujo SDD; integración Git (commit por tarea); modos Plan/Act y Supervised.
- 3 proveedores (Anthropic, OpenAI, Gemini) + Ollama. Cliente MCP básico.
- **Hito v0.1**: un desarrollador puede llevar una feature de idea a código, íntegramente en la terminal, con specs revisables. Métrica objetivo: primeras 1.000 estrellas y 20 contribuidores externos.

### Fase 2 — v1.0 (mes 5-9)
- GUI de escritorio en Tauri con paridad total (editor de specs enriquecido, revisión de diffs línea a línea).
- Orquestación multi-agente con git worktrees (estilo Orca); checkpoints/rollback; hooks; sandbox de ejecución.
- Host MCP completo + **soporte ACP** (cliente y agente). Sistema de plugins/skills.
- Modo dual spec-full / fast-forward.
- **Hito v1.0**: paridad terminal+escritorio verificada; instalable en macOS/Windows/Linux vía binario único; funciona headless por SSH.

### Fase 3 — Futuro (mes 10+)
- Compañero móvil (Tauri 2.x iOS/Android) para monitorizar agentes (estilo Orca mobile).
- Funciones de equipo: specs compartidas multi-repo (estilo OpenSpec Stores), steering por MDM/políticas.
- Schemas SDD personalizados (research-first, ADR). Marketplace abierto de skills/servidores MCP.
- Property-based testing sobre specs (idea validada por Kiro) y verificación automática spec↔código.

---

## 10. Riesgos y Mitigaciones

| Riesgo | Impacto | Mitigación |
|---|---|---|
| **Over-engineering del SDD** (el error de Kiro: mazo para nueces) | Fricción, abandono | Modo dual `fast-forward` para cambios triviales; specs proporcionales al tamaño de la tarea |
| **UX de revisión de specs pobre** (crítica unánime a spec-kit/Kiro) | Los devs prefieren revisar código a Markdown | Invertir en editor de specs enriquecido y diffs de deltas desde el MVP |
| **Inconsistencias de WebView en Tauri** | Bugs de UI por SO | Testeo en 3 SO, polyfills, framework cross-webview; lógica pesada fuera del GUI |
| **Seguridad de agentes autónomos** (incidente Kiro: caída de 6h por código sin revisar) | Daño en producción | Sandbox + aprobación por defecto; comandos de confianza explícitos; menor privilegio; Autopilot acotado al workspace |
| **Riesgos de MCP** (prompt injection, tools envenenados) | Exfiltración de datos | Consentimiento, revisión de descripciones de tools, control de acceso, registros auditados |
| **Ritmo del mercado** (herramientas cambian cada release) | Obsolescencia | Arquitectura desacoplada por protocolos abiertos (MCP/ACP); neutralidad de modelo |
| **Sostenibilidad open source** (freeloading de hiperescaladores) | Falta de fondos | Apache 2.0 para el núcleo + AGPL reservado a servidor/nube; modelo de servicios (estilo Zed) |
| **Fatiga del mantenedor / bus factor** | Estancamiento | Gobernanza abierta, CLA, roadmap público, dogfooding que atrae contribuidores |

---

## 11. Métricas de Éxito

**Adopción (comunidad):** estrellas en GitHub (referencia: opencode ~180k, Orca ~7k), número de contribuidores externos, descargas de releases/binarios, instalaciones por Homebrew/AUR.

**Producto (calidad SDD):** % de features entregadas con spec aprobada previa; ratio de trazabilidad código→requisito; tiempo de idea→código; tasa de rollback/retrabajo post-implementación.

**Paridad y rendimiento:** cobertura de features con paridad terminal↔escritorio (objetivo 100% del núcleo); tamaño de instalador GUI (objetivo <15 MB, gracias a Tauri); memoria en idle (objetivo <60 MB); arranque (<1 s).

**Salud del ecosistema:** número de servidores MCP y skills de la comunidad; agentes/editores interoperables vía ACP; issues abiertos/cerrados ratio (evitar el patrón de fricción alta que reflejó Kiro).

**Neutralidad:** número de proveedores LLM soportados y % de usuarios usando modelos locales (indicador de que la promesa model-agnostic y de privacidad se cumple).

---

*Fin del documento fundacional. Como corresponde a un proyecto spec-first, el siguiente paso NO es escribir código, sino ratificar la `constitution.md` y abrir la primera `change proposal` en `.askenaz/changes/` — construyendo Askenaz con Askenaz.*