<!-- SPDX-License-Identifier: Apache-2.0 -->
# Meltemi frente a Orca — qué falta para ser world-class

_2026-09-24 · `main` @ 4bc8e9e · Fuente: onorca.dev (landing, README de stablyai/orca, changelog), el árbol del repo verificado por 23 agentes (7 lectores por dimensión, 14 verificadores —evidencia en código y lente constitucional—, un crítico de completitud y una síntesis), y una **prueba integrada real** en esta máquina Windows 11 con Claude Code 2.1.280, Codex 0.77.0 → 0.156.1 y OpenCode 1.14.33 → 1.18.32 sobre un daemon `meltemid` aislado construido hoy en release._

---

## 0. Prueba integrada de hoy — lo medido, no lo supuesto

Entorno: daemon y adaptadores compilados desde `main` (`cargo build --workspace --release`), `MELTEMI_DATA_DIR`/`MELTEMI_CONFIG_DIR` aislados en el scratchpad, endpoint propio `\\.\pipe\meltemid-smoke-orca`, repo fixture temporal (`README.md` + `src/lib.rs` con `add`). Nada tocó tu daemon ni tus datos.

| Paso | Resultado | Evidencia |
|---|---|---|
| `meltemi fleet` | OpenCode L1 `ready`; Claude Code L2 `ready` (CLI + adaptador `bundled`); Codex L2 `ready`. Gemini/Copilot/Cursor/Kiro/Kilo/Aider/Antigravity no instalados | `out/fleet-release.out` |
| `meltemi tunnel` en Windows | Rehúsa honesto: named pipe no reenviable; el remedio nombra `meltemi bridge` | `out/tunnel.txt` |
| **OpenCode 1.14.33**, sesión libre de solo lectura | `status: completed`, 0 permisos denegados, **transcript vacío**: ni un `agent_message_chunk`, `usage_update` con 0 tokens. Causa: sus tres credenciales estaban rotas (Anthropic → 404, Copilot → 401, tier gratuito exige ≥ 1.18). **Meltemi reportó éxito con salida nula.** | logs `1e258e5b`, `bc5f7666` |
| **OpenCode 1.18.32** (actualizado), misma sesión | 20 s, 2 `tool_call` read, respuesta correcta en una frase, `usage_update` = 12 267 tokens. `meltemi usage` sigue diciendo `protocol_carries_no_usage` | log `dd976fef` |
| `meltemi session … --model anthropic/claude-haiku-4-5 --agent opencode` | **Rehusado 2006** «no documented place for a per-session model» aunque OpenCode anunció `config_options` con la lista de modelos en el mismo handshake | `core/meltemid/src/levels.rs:1037-1060` decide por nombre de binario |
| **Claude Code 2.1.280**, sesión libre de solo lectura por CLI | `completed` pero **12 permisos denegados** (kind `read`): el CLI no tiene aprobador («interactive approval arrives in a later release», `tui/src/run.rs:221-232`). El agente intentó `Read`, `Bash cat`, `PowerShell Get-Content`, `Grep`, `Glob` y un subagente; todo cancelado; respondió que no pudo leer | log `9eb77566` |
| Claude Code, sesión de escritura por CLI | 3 denegados (kind `edit`), `HELLO.txt` **no** se creó: el proxy deny-by-default funciona | log `a723cf9a` |
| **Claude Code vía `meltemi-devclient propose`** (aprobador automático) | **175 s, 7 permisos pedidos y aprobados, 0 denegados, `proposal.md` de 27 líneas** de calidad real (Why / What Changes / Impact, semántica de overflow, alcance), `src/` intacto. El hito de fase 0 con agente real, hoy | `out/devclient-propose3.out`, `proposal.md` |
| **Codex 0.77.0** | Handshake OK; el proveedor rechaza el turno: `gpt-5.6-sol` exige CLI más nuevo; `--model gpt-5.1-codex-max` «not supported with a ChatGPT account» | error 2002 con `remedy` |
| **Codex 0.156.1** (actualizado) | **El adaptador rehúsa el handshake**: el user agent ahora es `meltemi-codex-acp/0.156.1 (Windows…)` (Codex ecoa el nombre del cliente) y `version::of` busca el token `codex_cli_rs/` (`core/meltemi-adapters/src/codex/version.rs:26-29`, `wire.rs:52`). (Una versión anterior de este informe añadía que el techo `< 1.0.0` también la rehusaría; es falso, `0.156.1 < 1.0.0`, corregido por `codex-al-dia`.) **Codex no funciona con ninguna versión disponible hoy.** | error `provider_version_unreadable` |
| Modelos locales | Sin ollama, LiteLLM, LM Studio ni claves (`OPENROUTER_API_KEY`, etc.) en esta máquina. La **conexión por configuración sí queda demostrada**: `opencode.json` con proveedores `ollama` (`http://localhost:11434/v1`) y `litellm` (`http://localhost:4000/v1`) es aceptado y `opencode models ollama` lista `ollama/qwen2.5-coder:7b`. Falta el modelo vivo | `fixture-local-models/opencode.json` |
| GUI (Tauri, construida hoy con `tauri build --no-bundle`) | Arranca contra el daemon aislado, lista las 10 sesiones del fixture en la barra lateral con avatar por agente y título. **Mientras el `propose` corría (40 s de 175), la barra mostraba la sesión bajo «Stopped» y la barra de estado «0 working»**: la GUI no ve sesiones activas que otro cliente arrancó | `meltemi-gui-real-2026-09-24.png` |
| Captura oficial del sitio | `scripts/capture-desktop.ps1` corre limpio sobre mock-agent y produce la vista Sessions | `meltemi-gui-sessions-2026-09-24.png` |

**Cambios que hice en tu máquina** (reversibles): `npm i -g @openai/codex@latest` (0.77.0 → 0.156.1) y `npm i -g opencode-ai@latest` (1.14.33 → 1.18.32). Para volver: `npm i -g @openai/codex@0.77.0` · `npm i -g opencode-ai@1.14.33`. No instalé ollama: es un servicio con descarga de varios GB y lo dejo a tu decisión (abajo está la receta exacta).

**Trampas del entorno encontradas** (van a memoria): desde git-bash y desde un `pwsh` hijo el cliente no alcanza el named pipe (os error 3) — solo el PowerShell de esta sesión, que corre en integridad alta, conecta; `meltemi … | Out-File` se cuelga porque el daemon hereda el pipe (usar `Start-Process` con redirección a archivo); `meltemi-devclient --help` sin `MELTEMI_ENDPOINT` arrancó un daemon en tu endpoint por defecto (lo detuve).

---

## 1. Veredicto en diez líneas

1. **Meltemi va por delante** en lo que Orca no tiene ni puede copiar sin cambiar de negocio: gobierno de toda sesión (proxy deny-by-default, registro apend-only, checkpoint declarado — hoy comprobado con Claude Code: 15 peticiones gobernadas, 0 escrituras furtivas), método spec-first hospedado en `.meltemi/` (57 changes archivadas, 36 specs vivas), procedencia de release (SHA256SUMS y atestación; falta la clave minisign en el árbol), cero nube y cero cuentas, y una vía BYO-modelo real por OpenCode.
2. **Orca va por delante** en todo lo que el visitante ve el primer minuto: app móvil (iOS/Android por relé), 27 agentes frente a 10 (y de los 6 L1 de Meltemi solo OpenCode se ha ejercido; Codex hoy no arranca), terminal integrada, transcript rico (tool calls con resultado, diffs inline, spinner), estados de carga y movimiento, cadencia diaria frente a 7 semanas y 198 commits sin release, y comunidad (77k estrellas frente a 0).
3. **La brecha más honesta no es de features sino de evidencia y embudo**: las dos cuñas (gobierno y BYO-modelo) están construidas pero no publicadas, no documentadas y —hasta hoy— no medidas contra los CLIs instalados. La prueba de hoy encontró tres defectos reales en una tarde: Codex roto por formato de user agent, turno vacío reportado como éxito, y `--model` rehusado a un agente que sí anuncia modelos.
4. **Ninguna de las seis necesidades exige enmendar la constitución.** Dos posturas piden enmienda de rumbo/§11 (terminal como no-objetivo; idioma de la documentación). Lo único estructuralmente fuera es el relé en la nube y el push sin túnel de Orca (§3, rumbo «ni un servicio en la nube»); el resultado de usuario se alcanza con infraestructura del propio usuario.
5. **El orden importa**: sin estado vivo toda animación miente; sin clave publicada todo anuncio quema la primera impresión; sin corrida integrada los niveles declarados son papel.

---

## 2. Lo que pediste, uno por uno

### App móvil

- **Estado**: spec `.meltemi/specs/mobile-companion/spec.md` (monitorear/aprobar/revisar/dirigir, regla de subconjunto, solo túnel SSH) y maqueta `design-system/Meltemi Mobile.dc.html`. Cero código: `desktop/tauri.conf.json` solo declara msi/dmg/deb, sin `gen/android|apple`, sin `mobile/`. Los tres prerrequisitos de daemon de fase 3 están archivados y verificados: espera humana configurable (`core/meltemid/src/config.rs:131-166`), `waiting_permission`/`gatePending` reales, `session/watch` para clientes tardíos. Lo que ya llega a un teléfono hoy y nadie documenta: la TUI por `ssh -t pc meltemi` desde Termius/Blink/Termux (exige 80×24, `tui/src/shell/render.rs:91-97`) y la campana `MELTEMI_BELL=1` (`tui/src/shell/bell.rs`), que esas apps convierten en notificación local: un push de facto.
- **Brecha**: app (MOB-1); cliente SSH embebido sin decidir y `deny.toml` prohíbe las candidatas con TLS (MOB-2); emisor del aviso de espera opt-in inexistente (MOB-3; distinto del aviso de escritorio, que sí existe: `avisos-de-escritorio` 11/11); gracia de 30 s frente a suspensión en background (MOB-4); `permission/pending|decide`, `session/watch|log|cancel` sin verbo CLI (MOB-5, `docs/paridad-nucleo.md:28-30,59-60`); identidad y selector (MOB-7, `identidad-propia` 0/8); transporte `exec:` en el cliente (MOB-8).
- **Camino conforme**: (1) push con infraestructura del usuario: `[notices] on-waiting = "<comando>"` de ámbito usuario, ejecutado por GUI/TUI conectadas (nunca `meltemid`), recetas ntfy/Gotify/Apprise autohospedadas, contenido mínimo publicado antes de existir (§9), clave leída solo de `<config>/meltemi/`, jamás del repo; (2) app Android primero (Tauri 2 móvil reutilizando `desktop/ui`), `russh` solo en el crate móvil con excepción acotada en `deny.toml` y prueba de que la suite de escritorio sigue verde en Windows (§7), canal `exec` → `meltemi bridge` (funciona contra daemon Windows), clave del usuario en Keystore, sin importar claves de otras apps (§2); (3) presencia por `identity/status` leyendo el binario oficial de la malla. Orden único: MOB-5 → MOB-8 → perfiles de máquinas → `identidad-propia` → `companero-movil`.
- **Changes**: `companero-movil` (nombrada, sin abrir), `identidad-propia` (abierta, 0/8); nuevas vía rápida `verbos-del-agent-boss`, `via-ssh-del-cliente`, `aviso-de-espera-opt-in`, `acceso-remoto-documentado`. **Esfuerzo**: días para las vías rápidas; meses para la app.

### App de escritorio

- **Estado**: tablero de carrera con calles, diff por hunk y merge por archivo (`desktop/ui/src/lib/views/Review.svelte`); editor utilitario con LSP BYO; pestañas estilo Chrome con grupos; transcript con pensamiento en vivo (`pensamiento-a-la-vista`: 4/4 implementada, pendiente de archivo); compositor con encolar/relevar/detener; paleta Ctrl+K generada desde los schemas; uso y suscripciones; checkpoints.
- **Brecha confirmada**: las tool calls son una píldora título·estado; el daemon persiste `content`/`diff`/`locations` (`core/meltemid/src/acp.rs:739-742`) y ni la GUI (`desktop/ui/src/lib/conversation.ts:133-141`) ni la TUI (`tui/src/shell/conn.rs:835-845`) lo pintan. Sin adjuntos de imagen ni arrastre (`@ruta` cubre texto vía `core/meltemid/src/repo_map.rs:107-111`). Sin splits. Markdown crudo. Sin anotar diff. Sin rewind por turno. Columna worktree/rama que la spec exige (parcial: `projectRoot` ya es la ruta del worktree). Slash commands: el anuncio llega al log y a `session/watch` pero ninguna superficie lo lee. Añadidos por el crítico: sin vista previa de imágenes/PDF aunque el CSP ya admite `img-src data:`; ventana única (`minWidth: 900`); sin historial de avisos (`desktop/ui/src/lib/notices.ts:22-40` los retira a los 6 s); pestañas que no sobreviven al reinicio (`desktop/ui/src/lib/ui-state.ts:16-40`); sin updater; sin menú nativo; sin exportar sesión.
- **Fuera por rumbo, con equivalente conforme**: Design Mode → receta de servidor MCP de navegador inyectado por `mcp-passthrough` (nunca CDP desde el daemon, nunca segundo webview); GitHub/Linear nativos → «Abrir PR con…» como plantilla `openWith` (`desktop/src/fsops.rs:148`) que lanza el `gh` del usuario con su auth (§2); terminal embebido → «Abrir terminal aquí» hoy, y enmienda de rumbo que lo declare no-objetivo.
- **Changes**: `herramientas-a-la-vista` (nueva; absorbe la tarjeta de herramienta, diffs ACP y el mapeo Edit/Write → `ToolCallContent::Diff` en los adaptadores), `comandos-del-agente`, `arrastrar-referencias`, `adjuntos-en-el-compositor`, `panel-dividido`, `markdown-que-se-lee`, `anotar-el-diff`, `volver-a-este-turno`, `la-sesion-dice-su-rama`, `previsualizar-binarios`, `bandeja-de-avisos`, `pestanas-que-vuelven`, `abrir-pr-con`, `abrir-terminal-aqui`. **Esfuerzo**: días por vía rápida; semanas `panel-dividido`, `adjuntos-en-el-compositor`, `volver-a-este-turno`.

### Conexión remota (monitorear y hacer un cambio)

- **Estado**: `meltemi tunnel` (formatea `ssh -R`, rehúsa honesto en Windows — comprobado hoy), `meltemi bridge` (canal JSON-RPC completo por stdio, funciona con named pipe; `tui/src/run.rs:1642-1689`; smoke de 2026-08-15), reconexión con backoff en GUI y TUI, cola de permisos que sobrevive, `session/direct` con `interrupt`/`detach`, punto de encuentro documentado (`docs/acceso-remoto.md:84-146`). Monitorear y dirigir existen en el daemon. «Hacer un cambio» tiene la mitad: las escrituras van por `worktree/apply-edit` (RPC), pero `tree_read`/`tree_search`/`open_with` y el LSP leen el disco del cliente (`desktop/src/fsops.rs:54-148`, `desktop/ui/src/lib/editor/lsp.ts:38-59`), y `Settings.svelte:36-40` lee el `config.toml` local.
- **Brecha**: ningún cliente propio consume `bridge` (parcial: en macOS/Linux `-R` + `MELTEMI_ENDPOINT` sí sirve a GUI y TUI; desde o hacia Windows, nada); sin perfiles, prueba ni respawn del `ssh`; `connect_or_start` arranca un daemon local sobre un endpoint remoto caído (`core/meltemi-client/src/bootstrap.rs:33-53` — hoy lo vi ocurrir con el devclient); Windows como PC servido sin guía ni medición (`docs/plataformas.md:46-52` recomienda `-L` contra el `-R` del helper); túnel sin keepalive ni `ExitOnForwardFailure` (`tui/src/tunnel.rs:77`); `no-client-grace`/`wait` configurables pero no expuestos. La ruta GUI/TUI-sobre-túnel nunca se ha medido: 0 smokes en `docs/qa/`.
- **Camino conforme**: transporte `exec:` en `core/meltemi-client` (el cliente lanza el `ssh` del usuario con `meltemi bridge` fijo y habla por su stdio; cero crates; comando solo de ámbito usuario; `connect_or_start` no arranca daemon para `exec:`) → sección «Máquinas» con perfiles sin `IdentityFile` ni passphrases (delegar a ssh-agent/Pageant), presencia a demanda, ningún `ssh` huérfano al cerrar → degradación honesta de editor/búsqueda/LSP con daemon remoto y, para editar de verdad, `lectura-por-el-daemon`: RPC de lectura acotado a raíces registradas y worktrees, con paridad TUI por §4.
- **Changes**: `via-ssh-del-cliente`, `maquinas-remotas` (tras `identidad-propia`), `lectura-por-el-daemon` (nuevas); `arranque-solo-en-endpoint-propio`, `tunel-resiliente`, `acceso-remoto-documentado` (vías rápidas). **Esfuerzo**: vías rápidas en días; transporte y perfiles en semanas.

### Prueba integrada con agentes locales

- **Estado (docs)**: Claude Code alcanzó nivel verificado 2 contra `claude 2.1.167`; AskUserQuestion y `--model` solo contra mock; Codex nivel verificado 0 con esquema anclado a 0.77.0; OpenCode y los otros cinco L1 nunca ejercidos; el informe de aceptación v0.1 con dos agentes reales no existe (`docs/hito-v01.md` es plantilla).
- **Estado (hoy, sección 0)**: Claude Code completa `propose` de punta a punta con aprobaciones; OpenCode completa sesión libre tras actualizar; **Codex no arranca con ninguna versión disponible** (0.77 por modelo de cuenta, 0.156 por user agent); el CLI `session` no tiene aprobador, así que con Claude Code solo sirve para charlar.
- **Brechas nuevas encontradas hoy**: (a) turno «completed» con transcript vacío se reporta como éxito — un turno sin ningún `agent_message_chunk` ni `tool_call` debería cerrar como `empty_turn` con remedio; (b) `usage_update` de ACP llega al log y `meltemi usage` no lo pliega; (c) `--model` se rehúsa por nombre de binario aunque el agente anuncie opciones; (d) el passthrough MCP solo reconoce http/sse (`core/meltemid/src/mcp.rs:17-18`).
- **Camino**: `codex-al-dia` (aceptar `<producto>/<semver>` con producto anunciado por el cliente o `codex_cli_rs`, re-vendorizar con `codex app-server generate-json-schema`, subir el piso a la versión verificada y declarar lo no verificado, corrida de dos piernas hasta nivel 2; manual y opt-in, nunca en CI) → `turno-vacio-es-error` (vía rápida) → `corrida-integrada`: matriz {Claude 2.1.280, Codex actual, OpenCode 1.18 + ollama} × {session, propose, dispatch, implement, permiso, cancelar, direct, pregunta} × {CLI, TUI, GUI} sobre repos fixture, con `docs/conformidad-manual.md` como molde, y publicar `docs/qa/<fecha>-hito-v01-aceptacion.md`. Mientras el CLI no tenga aprobador, documentar `meltemi-devclient propose` como la vía scriptable con aprobación, o darle al CLI un `--approve-reads`.
- **Changes**: `codex-al-dia`, `turno-vacio-es-error`, `aprobador-en-el-cli`, `corrida-integrada`, `codex-reanudar-y-mcp`, `mcp-stdio-passthrough`, `usage-desde-acp`. **Esfuerzo**: días.

### Modelos locales vía LiteLLM / ollama / OpenRouter

- **Estado**: la única vía que funciona hoy es OpenCode (L1): anuncia `configOptions.model` y admite proveedores OpenAI-compatible y OpenRouter por su propio `opencode.json`; un `[[fleet.profile]]` con `env = { OPENCODE_CONFIG = ... }` y referencias `${VAR}` lo envuelve sin tocar Meltemi (`core/meltemid/src/config.rs:50-70`, lint de secretos en `:520-531`). Aider (L3) **no** es lanzable: los cinco verbos rehúsan `level_not_pilotable` (`core/meltemid/src/free_session.rs:101-112`, `propose.rs:102-113`, `sdd_flow.rs:477-486`, `server.rs:785-794,1724-1733`); la frase «Aider funciona hoy» de `docs/plan-de-cambios.md:258-261` y de la proposal de `motor-propio-byok` es falsa. `motor-propio-byok` es solo proposal y `deny.toml:62-109` prohíbe HTTP/TLS en todo el workspace.
- **Receta que funciona hoy** (verificada en forma, no en modelo vivo por falta de ollama):

  ```json
  { "$schema": "https://opencode.ai/config.json",
    "model": "ollama/qwen2.5-coder:7b",
    "provider": {
      "ollama":  { "npm": "@ai-sdk/openai-compatible", "name": "Ollama (local)",
                   "options": { "baseURL": "http://localhost:11434/v1" },
                   "models": { "qwen2.5-coder:7b": { "name": "Qwen2.5 Coder 7B" } } },
      "litellm": { "npm": "@ai-sdk/openai-compatible", "name": "LiteLLM proxy",
                   "options": { "baseURL": "http://localhost:4000/v1", "apiKey": "{env:LITELLM_API_KEY}" },
                   "models": { "gpt-5-mini": {} } },
      "openrouter": { "models": { "moonshotai/kimi-k2": {} } } } }
  ```

  Con `ollama pull qwen2.5-coder:7b` corriendo, `meltemi session "…" --agent opencode` en ese proyecto ya usa el modelo local; OpenRouter toma `OPENROUTER_API_KEY` del entorno del daemon; LiteLLM es el endpoint único para N proveedores.
- **Brecha**: cero guía (grep `ollama|litellm|openrouter` → solo el backlog); `--model` rehusado con remedio engañoso; el `model` del perfil se pierde en silencio en `propose`/`dispatch`/`implement` (solo `free_session.rs:119` aplica `with_session_levers`); el picker de la GUI no sabe qué agente admite modelo; sin tokens ni tokens/s aunque ACP ya los transporta (hoy: 12 267 tokens en el log, «no reportado» en `usage`).
- **Camino conforme**: `guia-modelos-locales` (`docs/local-models.md` con las tres recetas verificadas y fechadas, un perfil por receta, pierna «proveedor local» en `docs/conformidad-manual.md`, enlace desde `docs/agentes.md:104` y la sección BYO del sitio; corrige la afirmación sobre Aider); `modelo-por-opcion-anunciada` (tras `session/new`, si el agente anuncia opción de categoría `model`, enviar `set_config_option` con el valor verbatim, rehusar listando lo anunciado si no coincide, y aplicar `with_session_levers` en todos los verbos); `perfil-con-entorno`; `usage-desde-acp`. `motor-propio-byok` sigue siendo la cuña estratégica (meses; no adelantar; LiteLLM como fixture manual).

### World-class y atractivo visual

- **Estado**: un solo movimiento en el producto, el anillo cónico «composer-wind» (2.4 s) en Home y SessionDetail; reduced-motion honrado en dos capas (`desktop/ui/src/app.css:378-386`); regla dura pineada de que bandeja, tarjeta de permiso, avisos y barra lateral jamás animan (`desktop/tests/scenarios_shell.rs:258-272,4814-4832`); carga = texto «cargando…» en Project/Review/Fleet/Settings/Usage; TUI con glifos fijos y sin spinner aunque `.meltemi/specs/tui-shell/spec.md:336-339` reserva el gemelo ASCII. Las capturas de hoy lo muestran: una superficie densa, limpia y quieta; correcta como plano de control, muda como escaparate.
- **Brecha**: la señal «trabajando» llega tarde o no llega — anillo de SessionDetail, cubeta ▸ Trabajando y glifo de pestaña derivan de `refreshSessions()`, que solo corre al conectar, conmutar o enviar (`desktop/ui/src/lib/stores.ts:258-262`); **hoy la GUI mostró «Stopped» y «0 working» mientras Claude Code llevaba 40 s trabajando**. Sin esqueletos; cronómetro congelado (`SessionDetail.svelte:753`); «sin salida» mientras el turno vuela sin partes (`:898-900`); sin cursor de flujo; sin tokens de duración ni transiciones; TUI sin tick fino; calles sin progreso intra-turno; ventana que puede pintar el fondo del SO antes del primer frame. En producto: clave minisign fuera del árbol; 0 estrellas y Discussions desactivado; quickstart que empieza en `cargo build` y TOML a mano aunque `--agent` y el chip de flota ya lo evitan; sitio sin producto en movimiento y sin nombrar un agente; 198 commits sin release ni CHANGELOG; docs sin sitio e idioma contradictorio con §11; sin winget/brew/scoop; sin `meltemi doctor`, FAQ ni página de privacidad; sin aviso de versión nueva.
- **Camino**: `estado-en-vivo` primero (notificación `session/state` a todo cliente por JSON-RPC, declarada en `proto/schemas/v1`; la TUI la consume en vez de sondear), porque sin estado vivo toda animación de trabajo miente; luego el vocabulario de la sección 4; en producto, cerrar procedencia, abrir canales y publicar la cuña antes de anunciar.
- **Changes**: `estado-en-vivo`, `estados-de-carga`, `pulso-de-la-tui`, `senal-de-vida-del-turno`, `vocabulario-de-movimiento`, `ventana-hasta-el-primer-frame`; producto: `procedencia-de-release` (cerrar), `canales-de-comunidad`, `quickstart-sin-cargo`, `cadencia-y-changelog`, `demo-animada-del-sitio`, `meltemi-doctor`, `docs-en-el-sitio` (con enmienda de §11), `distribucion-gestores-paquetes`, `demo-guiada`.

---

## 3. Tabla de brechas verificadas

Verificación: **C** confirmada, **P** parcial (con lo que existe). Constitución: sí / reencuadre / enmienda. Filas fusionadas cuando dos dimensiones describen el mismo hecho. Las filas marcadas ★ las confirmó además la prueba de hoy.

| Id | Dimensión | Severidad | Verificación | Constitución | Change | Esfuerzo |
|---|---|---|---|---|---|---|
| AI-1 ★ | Agentes | crítica | C: Codex no arranca con 0.77 ni con 0.156 | sí (manual, nunca en CI) | codex-al-dia (nueva) | días |
| AI-2 ★ | Agentes | alta | C: sin corrida integrada publicada | sí (fixtures, no este repo) | corrida-integrada (nueva) | días |
| AI-11 ★ | Agentes | alta | C: turno vacío = «completed» | sí | turno-vacio-es-error (nueva) | días |
| AI-12 ★ | Agentes | alta | C: el CLI `session` deniega todo permiso; solo devclient aprueba | sí (§3: la decisión sigue siendo humana o de regla) | aprobador-en-el-cli (nueva) | días |
| LM-3 ★ | Modelos | alta | P: el daemon rehúsa `--model` por nombre de binario; OpenCode sí anuncia | sí | modelo-por-opcion-anunciada (nueva) | días |
| LM-7 / AI-6 ★ | Modelos·Agentes | media | C: `usage_update` en el log, `usage` dice «no reportado» | sí (`_meta` namespaced; §2) | usage-desde-acp · extension-acp-usage | días / semanas |
| VIS-1 ★ | Visual | crítica | P: el anillo del Home usa `running` local; el resto depende de `refreshSessions` | sí (notificación JSON-RPC; §4) | estado-en-vivo (nueva) | semanas |
| R3 ★ | Remoto | alta | C: cliente arranca daemon local sobre endpoint ajeno | sí (refuerza §3) | arranque-solo-en-endpoint-propio (vía rápida) | días |
| MOB-1 | Móvil | crítica | C | sí (§10 por plugin; `mobile/` al rumbo) | companero-movil (sin abrir) | meses |
| MOB-2 | Móvil | crítica | C | sí (russh solo en crate móvil; excepción `deny.toml`) | companero-movil | semanas |
| MOB-3 / R6 | Móvil·Remoto | alta | P: `MELTEMI_BELL` ya es push por `ssh -t`; gracia configurable sin exponer; emisor del aviso ausente | sí (ámbito usuario; nunca el daemon; §9 antes) | aviso-de-espera-opt-in (nueva) | días |
| MOB-4 | Móvil | alta | C | sí (gracia finita, sin keep-alive) | companero-movil (design) | días |
| MOB-5 | Móvil | alta | C | sí (refuerza §4) | verbos-del-agent-boss (nueva) | días |
| MOB-6 / R5 | Móvil·Remoto | media | C | sí (es la TUI, no el compañero) | acceso-remoto-documentado · modo-estrecho-tui | días |
| MOB-7 / R8 | Móvil·Remoto | media | C | sí (sin sondeo silencioso; BYO-network) | identidad-propia (0/8) → maquinas-remotas | semanas |
| MOB-8 / R1 | Móvil·Remoto | crítica | P: `-R` + `MELTEMI_ENDPOINT` sirve en macOS/Linux; sin `exec:`; Windows sin camino | sí (comando solo de ámbito usuario) | via-ssh-del-cliente (nueva) | semanas |
| MOB-9 | Móvil | media | C | sí (confirmación explícita por archivo) | diferida con razón (eventos-para-tardios) | semanas |
| D1 / VIS-2 / AI-4 / D9 / VIS-8 | Escritorio·Visual·Agentes | alta | C (GUI y TUI; calles con glifo por turno, sin lo vivo) | sí (§5: estructura ACP genérica; `rawInput` plegado) | herramientas-a-la-vista (nueva) | semanas |
| D2 / AI-7 | Escritorio·Agentes | alta | P: llega al log y a `session/watch`; nadie lo lee | sí (texto al agente, sin interpretar) | comandos-del-agente (nueva) | días |
| D3 / AI-5 | Escritorio·Agentes | alta | C para imágenes/arrastre; `@ruta` cubre texto | sí (log sin bytes; gating por `prompt_capabilities.image`) | arrastrar-referencias · adjuntos-en-el-compositor | días / semanas |
| D4 | Escritorio | alta | C | sí (2 paneles, dentro de edit-surface) | panel-dividido (nueva) | semanas |
| D5 | Escritorio | media | C | **reencuadre**: imágenes vía daemon, no bridge Rust | markdown-que-se-lee (nueva) | días |
| D6 | Escritorio | media | C | sí (cierra meltemi.md §6.8) | anotar-el-diff (nueva) | días |
| D7 | Escritorio | media | P: filtro por texto en Sesiones, búsqueda por transcript, Ctrl+P en Editor | sí (§4 TUI + CLI) | paleta-que-encuentra · buscar-en-las-sesiones | días |
| D8 | Escritorio | media | C | sí (§4: TUI y CLI también) | volver-a-este-turno (nueva) | semanas |
| D10 | Escritorio | baja | C | sí como receta MCP; motor embebido prohibido | receta en docs/agentes.md | días |
| D11 | Escritorio | baja | C | sí (plantilla, `--web`, §2) | abrir-pr-con (nueva) | días |
| D12 | Escritorio | baja | C | **enmienda** para la postura; atajo sí | abrir-terminal-aqui + enmienda rumbo | días |
| D13 | Escritorio | baja | P: `projectRoot` es el worktree; falta rama y columna | sí (repara spec viva) | la-sesion-dice-su-rama (nueva) | días |
| D-V1 | Escritorio | media | C: sin updater, menú nativo ni exportar sesión | sí (aviso opt-in, solo cliente; §9 antes) | avisos-de-version · menu-nativo-aplicacion | días |
| R2 | Remoto | alta | P: chip ◌/●/▲ y backoff existen; sin perfiles, prueba ni respawn | sí (sin `IdentityFile`; sin ssh huérfano) | maquinas-remotas (nueva) | semanas |
| R4 | Remoto | alta | C | sí (§7 lo exige) | acceso-remoto-documentado + smoke | días |
| R7 | Remoto | media | C (también `Settings.svelte` lee config local) | sí (→ §4; acotado a raíces) | lectura-por-el-daemon (nueva) | semanas |
| R9 | Remoto | baja | C | sí | tunel-resiliente (vía rápida) | días |
| R-V1 | Remoto | media | C: GUI/TUI sobre túnel nunca medidos | sí | smoke en corrida-integrada | días |
| AI-3 | Agentes | alta | P: `[[fleet.custom]]` ya admite cualquier ACP nativo; faltan datos de registro | sí (`grey` por defecto; verificar vendor por vendor) | flota-ampliada / registro-agentes-en-superficie | días |
| AI-8 | Agentes | media | C | sí (override por cable, no archivos del CLI) | codex-reanudar-y-mcp | días |
| AI-9 | Agentes | media | C | sí para L1 + L3; pty = enmienda | flota-ampliada · runner-nivel-3 | meses |
| AI-V1 | Agentes | alta | C: 5/6 L1 nunca ejercidos; MCP stdio omitido (`mcp.rs:17-18`) | sí | corrida-integrada · mcp-stdio-passthrough | días |
| LM-1 | Modelos | crítica | C: «Aider funciona hoy» es falso | **reencuadre**: modelo verbatim por dato del registro | corrección de texto (vía rápida) · runner-nivel-3 | días / semanas |
| LM-2 / LM-5 / LM-8 / P5 | Modelos·Producto | alta | C: cero guía | sí (claves solo por `${VAR}`) | guia-modelos-locales (nueva) | días |
| LM-4 | Modelos | alta | C | sí (excepción `deny.toml` acotada + test) | motor-propio-byok | meses |
| LM-6 | Modelos | media | C | sí (§4 ×3; lint de secretos) | perfil-con-entorno (nueva) | días |
| VIS-3 | Visual | alta | C | sí (keyframes en `app.css`) | estados-de-carga (vía rápida) | días |
| VIS-4 | Visual | media | P: ticker de 2 s ya redibuja; sin tick fino ni frames | sí (usar `--ascii` existente; probar en conhost) | pulso-de-la-tui (vía rápida) | días |
| VIS-5 / VIS-6 | Visual | media | C | sí (cursor fijo, o enmendar «uno por vista») | senal-de-vida-del-turno (vía rápida) | días |
| VIS-7 | Visual | media | C | sí (no relajar las guardias) | vocabulario-de-movimiento (vía rápida) | días |
| VIS-9 | Visual | baja | P: flash de tema mitigado; ventana sin `visible:false`; onboarding estático | sí (`allow-show` o `show()` desde `lib.rs`; tope 500 ms) | ventana-hasta-el-primer-frame | días |
| P1 | Producto | crítica | P: SHA256SUMS y atestación ejecutables; sin clave minisign ni firma de plataforma | sí (firma manual fuera de CI) | procedencia-de-release (6/8) · firma-de-plataforma | días |
| P2 | Producto | crítica | P: docs de gobernanza existen; canales no | sí (badges no en `site/`) | canales-de-comunidad (nueva) | días |
| P3 | Producto | alta | P: `--agent` y chip de flota ya evitan el TOML; docs desactualizados; sin demo | sí (mock-agent es binario propio) | quickstart-sin-cargo · demo-guiada | días / semanas |
| P4 | Producto | alta | C | sí (sin JS; reduced-motion; «simulado» declarado) | demo-animada-del-sitio (nueva) | días |
| P6 | Producto | media | C (198 commits sin release) | sí | cadencia-y-changelog (nueva) | días |
| P7 | Producto | media | P: `site/es/` existe; CONTRIBUTING tiene política | **enmienda** §11; sin buscador JS | docs-en-el-sitio + enmienda-idioma | semanas |
| P8 | Producto | media | C | sí (tras `release: published`) | distribucion-gestores-paquetes | semanas |
| P9 | Producto | media | P: `e2e_hito` y dos procedimientos manuales existen; falta el informe | sí | corrida-integrada (mantenedor) | días |
| P-V1 | Producto | media | C: sin `meltemi doctor`, FAQ, página de privacidad, completions ni SBOM | sí | meltemi-doctor · faq-y-privacidad | días |
| C-A | Escritorio | media | C: 0 `<img>`; CSP ya admite `data:` | sí (edit-surface) | previsualizar-binarios | días |
| C-B | Escritorio | baja-media | C: ventana única | sí | segunda-ventana | semanas |
| C-C | Escritorio | media | C: `notices.ts:22-40` sin archivo | sí (§4 si toca el daemon) | bandeja-de-avisos | días |
| C-D | Escritorio | baja-media | C: `ui-state.ts` sin pestañas | sí | pestanas-que-vuelven | días |
| C-F | Agentes | baja | C: ningún verbo de Meltemi expuesto a un agente | sí | receta CLI-como-herramienta + MOB-5 | días |

---

## 4. Vocabulario de movimiento propuesto

Compatible con `docs/ux/design-system.md` (Motion: 120–160 ms, ease-out, solo opacidad/transform, un indicador ambiental por vista) y con las guardias pineadas. Tokens en `desktop/ui/src/app.css`: `--dur-1: 120ms` (llegada), `--dur-2: 160ms` (cambio de estado), `--dur-ambient: 2.4s` (existe), `--dur-pulse: 1.2s` (esqueleto), `--ease-out: cubic-bezier(0.2, 0, 0, 1)`. Jamás altura, posición ni color de fondo animados.

| Clase | Qué | Dónde | Duración · easing | Reduced-motion | Regla que respeta |
|---|---|---|---|---|---|
| A · Ambiental | anillo cónico (existe) | compositor de Home y SessionDetail; tablero | 2.4 s lineal | se retira (`display:none`), no se congela | «uno por vista»; encendido solo por estado vivo (VIS-1) |
| B · Llegada | `.arrive` opacidad 0→1 | `<main>` al conmutar vista; ítems nuevos del transcript; resultados de paleta; contenido del cajón | 100–120 ms · ease-out | kill-switch global (`app.css:378-386`) | nunca en bandeja, tarjeta de permiso, avisos ni barra lateral (llegada en seco) |
| C · Cambio de estado | crossfade de glifo/píldora (▸→●, ○→◔→✓/✕) | filas de herramienta; cabecera de sesión; calles; pestañas (un solo parpadeo) | 160 ms · ease-out | cambio en seco | contadores de la barra lateral cambian en seco (guardias `scenarios_shell.rs:4814-4832`) |
| D · Esqueleto | filas de 32 px, barras 62/38/48/55/70/30 % (del mockup móvil), `aria-busy` | Project, Review, Fleet, Settings, Usage, paleta; solo si la espera supera 150 ms | pulso de opacidad 0.55↔1 · 1.2 s ease-in-out | barras estáticas + «cargando…» | nunca gradiente en barrido (reserva de marca); keyframes en `app.css` |
| E · Flujo | «◔ trabajando… m:ss» en turno abierto sin partes; cursor ▍ fijo tras el último chunk; sin typewriter | transcript | texto; cronómetro tabular 1 s | no aplica | cursor fijo para no ser segundo ambiental |
| F · Herramientas | fila de 32 px: glifo de estado, título mono, `0:07`, línea ⎿ resultado (n líneas, +a −r), diff plegable cerrado, lotes «Leyó 4 archivos» | transcript GUI; TUI como `◔ 0:07 Read src/x.rs` + `⎿` | llegada B + estado C | glifo en seco | ACP puro (`kind`, `content`, `locations`); `rawInput` solo plegado bajo demanda |
| G · TUI | frames braille ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ con gemelo `-\|/`; cronómetro `mm:ss` en header y drill-in | glifo ACTIVE; tercer arm de `select!` en `tui/src/shell/mod.rs:82-117` armado solo con `is_working()` | 80–100 ms/frame; tick de 1 s | `--ascii`/`MELTEMI_ASCII` → ▶ + «trabajando…» | spec tui-shell:336-339 (gemelo ASCII); cero CPU en reposo; probar en conhost (§7) |
| H · Marca | trazo del mark una sola vez | onboarding | 600 ms · ease-out | estático | único sitio con animación de marca; sin splash ni glow |
| I · Ventana | `visible:false` + `show()` tras el primer frame | `desktop/tauri.conf.json`, `desktop/src/lib.rs` | tope 500 ms aunque el frontend no señale | no aplica | capability `core:window:allow-show` justificada, o `show()` desde Rust |

---

## 5. Hoja de ruta en tres tandas

### Tanda 1 — ≤ 2 semanas: confianza ejecutable, prueba integrada y quick wins

| Change | Por qué | Dependencia |
|---|---|---|
| codex-al-dia | Codex hoy no arranca con ninguna versión; el parser de user agent, el piso y la compuerta de aprobación que el cable nuevo exige gobernar, más la re-vendorización | — |
| turno-vacio-es-error · aprobador-en-el-cli | los dos defectos que la prueba de hoy vio primero; sin aprobador el CLI no sirve con Claude Code | — |
| procedencia-de-release (cerrar 2/8) | `minisign.pub` en el árbol, lint de clave, v0.1.2 firmada: sin esto todo anuncio quema la primera impresión | mantenedor |
| corrida-integrada | la matriz agente × verbo × superficie; publica el informe de aceptación v0.1 | codex-al-dia |
| guia-modelos-locales | única vía funcional (OpenCode) sin escribir; corrige lo de Aider; nombra LiteLLM; receta de arriba | instalar ollama para la pierna local |
| modelo-por-opcion-anunciada · usage-desde-acp | hacen funcionar el picker y `--model` con OpenCode; muestran los tokens que ya llegan | — |
| verbos-del-agent-boss | `meltemi permissions/decide/watch`: aprobar desde el teléfono por SSH y script del aviso | — |
| arranque-solo-en-endpoint-propio · tunel-resiliente | un día cada una; hoy un endpoint caído arranca un daemon equivocado | — |
| acceso-remoto-documentado | Windows como PC servido, «desde el teléfono, hoy», `MELTEMI_BELL`, ajustes de gracia; corrige `docs/plataformas.md` | — |
| comandos-del-agente · arrastrar-referencias | ≤ 2 días cada una | — |
| estados-de-carga · pulso-de-la-tui · senal-de-vida-del-turno · ventana-hasta-el-primer-frame | quick wins visuales sin tocar el daemon | — |
| canales-de-comunidad · cadencia-y-changelog · quickstart-sin-cargo | Discussions, plantillas, badges, CHANGELOG desde el archivo, quickstart sin `cargo` ni TOML | anunciar solo tras procedencia |

### Tanda 2 — 1 a 2 meses: estado vivo, transcript rico, remoto usable

| Change | Por qué | Dependencia |
|---|---|---|
| estado-en-vivo | sin `session/state` toda animación de trabajo miente (hoy: «0 working» con un agente trabajando); TUI deja de sondear | — |
| herramientas-a-la-vista | tarjeta de herramienta, diffs ACP en el transcript, subagentes, calles vivas | estado-en-vivo; adaptadores emiten `ToolCallContent::Diff` |
| via-ssh-del-cliente → maquinas-remotas → lectura-por-el-daemon | GUI/TUI marcan a `ssh pc meltemi bridge`, perfiles, chip, y editar de verdad en remoto | identidad-propia (terminar 0/8) |
| aviso-de-espera-opt-in | push con ntfy/Gotify del usuario; contenido publicado antes (§9) | archivar avisos-de-escritorio |
| flota-ampliada · codex-reanudar-y-mcp · mcp-stdio-passthrough | de 10 a ~18 agentes L1 con estatus `grey` verificado; Codex reanuda y recibe MCP | codex-al-dia |
| extension-acp-usage | tokens y rate-limits por `_meta` namespaced; propuesta upstream | usage-desde-acp |
| markdown-que-se-lee · anotar-el-diff · paleta-que-encuentra · la-sesion-dice-su-rama · previsualizar-binarios · bandeja-de-avisos · pestanas-que-vuelven · avisos-de-version | pulido de escritorio, todas vías rápidas | — |
| vocabulario-de-movimiento | tokens y utilidades de la sección 4 en `app.css` y design-system | estados-de-carga |
| demo-animada-del-sitio · meltemi-doctor · docs-en-el-sitio (+ enmienda §11) · distribucion-gestores-paquetes · perfil-con-entorno | embudo: producto vivo en la portada, diagnóstico, docs navegables, `brew`/`winget`/`scoop` | procedencia cerrada; enmienda ratificada |

### Tanda 3 — apuestas grandes

| Change | Por qué | Dependencia |
|---|---|---|
| companero-movil | Android primero, `russh` acotado, canal `exec` → `bridge`, reconexión al frente, gracia larga por config con techo | identidad-propia, via-ssh-del-cliente, estado-en-vivo, aviso-de-espera-opt-in, verbos-del-agent-boss |
| motor-propio-byok | la cuña BYO-modelo sin depender de OpenCode; excepción `deny.toml` acotada a `core/meltemi-engine` con test de que `meltemid` sigue sin HTTP; LiteLLM como fixture manual | guia-modelos-locales, extension-acp-usage |
| panel-dividido · volver-a-este-turno · demo-guiada · segunda-ventana | dos paneles dentro de edit-surface; checkpoint por turno con confirmación; `meltemi demo` con mock-agent etiquetado «simulado» | vocabulario-de-movimiento |
| runner-nivel-3 (opcional) | solo si Aider u otro CLI headless lo justifica | corrección de texto ya hecha |
| enmiendas: terminal como no-objetivo, §11 idioma, sucesion-y-custodia | posturas escritas antes de que la comunidad las pregunte | firma del mantenedor |

---

## 6. Lo que Meltemi NO debe copiar de Orca

- **Relé en la nube, pairing por código y push sin túnel**: §3, rumbo product («ni un servicio en la nube»), spec `remote-access`. Equivalente: bastión o malla del usuario + aviso opt-in a ntfy/Gotify autohospedado.
- **Cuentas, login y usage por API del proveedor**: §2 prohíbe leer credenciales o llamar a la API con el token del CLI; §5 y rumbo prohíben cuentas. El uso solo entra por la salida oficial del agente (`_meta` ACP).
- **Hot-swap de cuentas y caché de passphrases**: tocar `~/.claude`, `~/.codex` o retener passphrases roza material de claves (§2). Se elige por perfil al abrir sesión y se delega a ssh-agent/Pageant.
- **«Any CLI» en terminal pty**: una pty esquiva el proxy deny-by-default y el registro apend-only que el rumbo declara piso de toda sesión; exigiría enmienda y un quinto nivel «sin gobierno». La amplitud se cubre por ACP nativo (L1) y headless con salida JSON (L3).
- **Chromium embebido por worktree (Design Mode)**: meltemi.md §7.1/§7.4 e `instaladores-linux-sin-webview` rechazan empaquetar un motor de navegador. Vía: MCP de navegador del agente.
- **GitHub/Linear/CI nativos en el daemon**: rumbo «ni CI/CD», sin red en el daemon (§3). Vía: `gh` del usuario por plantilla y MCP declarado.
- **Permisos Yolo por defecto**: §3 deny-by-default; lo irreversible exige aprobación explícita incluso en modo autónomo.
- **Búsqueda de sesiones entre máquinas por agregador**: un daemon por túnel; nada de índice central.
- **Heartbeats, telemetría o métricas remotas**: §9.
- **Releases automáticas, nightlies o firma en CI**: `procedencia-de-release` y SLSA v1.2; los builds de `build.yml` son artefactos sin firmar de 7 días.
- **JavaScript o widgets externos en el sitio**: spec `product-site`; el demo animado va en SVG/WebM autoalojado con `prefers-reduced-motion`.
- **Splits infinitos y terminal WebGL**: §10 y edit-surface; como máximo dos paneles.

---

## 7. Lo que solo una medición real puede confirmar (y hoy no se pudo)

- Un modelo local vivo bajo Meltemi: falta `ollama` en esta máquina (`winget install Ollama.Ollama` + `ollama pull qwen2.5-coder:7b`; luego la receta de la sección 2 y `meltemi session "…" --agent opencode`).
- Codex con el adaptador corregido: nivel alcanzable, `thread/resume`, MCP.
- GUI y TUI sobre un socket reenviado por SSH y `sshd → meltemi bridge` en Windows (sin OpenSSH Server aquí).
- `set_config_option` a un modelo `ollama/…` en OpenCode 1.18; forma de `updatedInput` para AskUserQuestion en Claude 2.1.280.
- Gemini/Copilot/Cursor/Kiro/Kilo: `bin`/`acp-args` del registro nunca ejecutados.
- `russh` en el toolchain Windows; render braille en conhost; costo de CPU del tick de la TUI.
- Los tres jobs de `build.yml` en runner real y el DMG de macOS por esa ruta.

## Anexo — evidencia de la prueba integrada

- Capturas y logs de la corrida (sesiones JSONL, salidas por paso, dos capturas de la GUI) quedaron en el scratchpad de la sesión de trabajo y no se versionan; la corrida reproducible es la change `corrida-integrada`.
- Las 39 propuestas que cubren las brechas viven en `.meltemi/changes/` y su orden en `docs/plan-de-cambios.md` («Orden de abordaje tras el comparativo con Orca»).
