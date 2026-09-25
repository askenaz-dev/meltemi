# modelo-por-opcion-anunciada

> Vía completa: enmienda la regla de «qué palancas admite un agente» que
> `modelo-y-esfuerzo-por-sesion` dejó decidida por nombre de binario, y toca
> el arranque de sesión en todos los verbos. Nace del comparativo con Orca y
> de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas LM-3 y LM-6).

## Why

`meltemi session "…" --agent opencode --model anthropic/claude-haiku-4-5`
respondió con el error 2006: «`opencode.cmd` has no documented place for a
per-session model, so it was refused rather than dropped in silence». Rehusar
en vez de callar es la postura correcta. El diagnóstico es falso: en el mismo
handshake OpenCode anunció `configOptions` con una opción de categoría
`model` y 317 valores —está medido en `docs/conformidad-manual.md` («las dos
formas, pobladas»)— y el daemon lo registró en el log como
`config_options_announced`.

La causa está en `core/meltemid/src/levels.rs:1037-1060`: `levers_of` decide
por el **nombre del binario** («codex» → modelo y esfuerzo; «claude» →
modelo; cualquier otro → nada). Fue honesto en su fecha —D9 de
`modelo-y-esfuerzo-por-sesion` concluyó que ningún proveedor pineado anunciaba
opciones— y esa fecha pasó: el único nivel 1 instalado anuncia, y la vía en
vivo (`session/set_config_option`, verbo `set-option`) existe pero solo sirve
a una sesión ya abierta, que el CLI no tiene porque su sesión termina con el
turno. Dos consecuencias más, verificadas: el `model` de un perfil
(`FleetProfile.model`, `core/meltemid/src/config.rs`) solo se aplica en la
sesión libre (`free_session.rs:119`), y se pierde **en silencio** en
`propose`, `dispatch` e `implement`; y el chip «Model» del compositor de la
GUI no sabe qué agente admite qué.

Es la palanca que decide cuánto cuesta cada sesión y la puerta de entrada al
modelo local: elegir `ollama/…` en OpenCode es exactamente esto.

## What Changes

- **La opción anunciada manda**: al abrir sesión, si el agente anuncia una
  opción de categoría `model` (o `effort`), el valor pedido —por bandera, por
  perfil o por chip— se envía con `session/set_config_option` **verbatim**
  antes del primer prompt. Un valor que no está en la lista anunciada se
  rehúsa **listando lo anunciado**, nunca se adivina ni se aproxima
  (constitución §5: Meltemi no interpreta modelos). El log registra solo el
  valor confirmado por el agente.
- **`levers_of` deja de leer nombres de binario**: el registro de flota gana
  un campo aditivo `levers` por entrada (qué admite por bandera de
  lanzamiento, verificado contra el CLI pineado) y la regla pasa a ser
  «bandera declarada, o anuncio en vivo, o rehúso». Un agente `custom` sin
  dato: anuncio o rehúso. La entrada de Codex y la de Claude conservan lo que
  hoy tienen, ahora como dato y no como `match` sobre un string.
- **`with_session_levers` en todos los verbos** que arrancan un agente
  (`sdd/propose`, `sdd/explore`, `worktree/dispatch`, `sdd/implement`), con
  la precedencia ya decidida: lo explícito de la sesión pisa el perfil; el
  perfil que no declara no impone.
- **El chip de la GUI y la paleta de la TUI saben qué ofrecer**: con el
  agente elegido, el chip «Model» muestra la lista anunciada la última vez
  por ese agente (guardada por el daemon en `fleet/list`, aditiva) o un campo
  libre con la advertencia «se confirmará al arrancar». Sin lista y sin
  bandera declarada, el chip dice que ese agente no toma modelo.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: MODIFIED «Modelo y esfuerzo como transporte opaco por
  sesión» (tres vías por palanca —bandera declarada, opción anunciada,
  rehúso—; validación solo contra lo anunciado; lo confirmado queda en el
  registro; rehúso antes de crear cuando ya se sabe y al abrir, sin prompt,
  cuando solo el anuncio lo dice).
- `fleet-catalog`: + «El registro declara las palancas de lanzamiento», «La
  flota recuerda lo último que cada agente anunció» y «Las palancas de cada
  agente se ven en toda superficie» (CLI y TUI); MODIFIED «El perfil declara su
  modelo y su esfuerzo» (rige en todo verbo que arranca un agente: sesión
  libre, `propose`, autoría, `dispatch`, `implement`).
- `gui-shell`: MODIFIED «El modelo y el esfuerzo se eligen y se ven» (el
  selector sabe qué ofrece el agente elegido).

## Impact

- Archivos: `core/meltemid/src/{levels.rs, session_config.rs, free_session.rs,
  propose.rs, sdd_flow.rs, implement.rs, worktrees.rs, fleet.rs}`,
  `core/meltemid/data/fleet-registry.toml`, `proto/schemas/v1/{fleet,
  session-config}.schema.json` (aditivos), `desktop/ui/src/lib/views/Home.svelte`
  y `SessionDetail.svelte`, `tui/src/shell/palette.rs`, `docs/agentes.md`,
  `docs/paridad-nucleo.md`.
- Cero dependencias nuevas.
- Riesgo real: el orden `session/new` → anuncio → `set_config_option` → prompt
  añade un viaje de ida y vuelta antes del primer turno; el design mide el
  costo con el agente simulado y fija el tiempo máximo de espera del anuncio.
- Lo que solo la medición confirma: que OpenCode 1.18 honra un
  `set_config_option` a un modelo `ollama/…` y arranca con él.

## Fuera de alcance

- Traducir nombres de modelo entre proveedores, o un catálogo de modelos
  propio: el valor es del agente, opaco para Meltemi.
- Cambiar de modelo a mitad de turno: `set-option` ya lo hace entre turnos.
- Tokens y costo: `usage-desde-acp`.
