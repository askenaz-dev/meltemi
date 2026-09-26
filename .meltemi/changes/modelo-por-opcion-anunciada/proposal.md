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

- **La opción anunciada manda, y se confirma**: al abrir sesión, si la entrada
  del agente no declara la palanca por bandera, el valor pedido —por bandera de
  la CLI, por el compositor o por el perfil— se fija con
  `session/set_config_option` **verbatim** sobre la opción que el agente anunció
  en la categoría de la palanca (`model`, `thought_level`), antes del primer
  prompt. Un valor que el agente no anunció se rehúsa **listando lo
  anunciado**, nunca se adivina (§5); y si el agente responde dejando otro valor,
  cambia el modo o no contesta, **no sale ningún prompt**: el contenido del
  repositorio va solo al modelo que se eligió (design D3).
- **`levers_of` deja de leer nombres de binario**: el registro de flota gana
  un campo `levers` por entrada (qué toma por bandera de lanzamiento, que solo
  los adaptadores propios pueden declarar) y la regla pasa a ser «bandera
  declarada, o anuncio confirmado, o rehúso». Codex y Claude Code conservan lo
  que hoy tienen, ahora como dato (design D1, D2).
- **El rehúso llega a quien lo pidió, como dato**: un evento `lever_refused` y
  el error 2007 con la palanca, lo pedido, la razón y lo anunciado, en todo verbo
  —hoy `dispatch` lo dice «el agente se negó» e `implement` lo pierde—, y las
  superficies lo dicen en español e inglés (design D4).
- **Las palancas del perfil en todos los verbos** que arrancan un agente
  (`propose`, los de autoría, `dispatch`, `implement`), resueltas antes de crear
  nada (design D5).
- **El histórico dice lo que rigió**: nunca un modelo rehusado como el que
  corrió (design D6).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: MODIFIED «Modelo y esfuerzo como transporte opaco por
  sesión» (dos vías y rehúso antes de crear nada; lo rehusado no figura como lo
  que rigió) y + «La palanca anunciada se confirma antes del primer prompt».
- `fleet-catalog`: + «El registro declara las palancas de lanzamiento»;
  MODIFIED «El perfil declara su modelo y su esfuerzo» (rige en todo verbo que
  arranca un agente).
- `gui-shell`: MODIFIED «El modelo y el esfuerzo se eligen y se ven» (la
  conversación dice la palanca rehusada).
- `tui-shell`: MODIFIED «El terminal declara el modelo efectivo» (la
  transcripción dice la palanca rehusada).
- `cli-contract`: + «El CLI dice la palanca que rigió o por qué se rehusó».

## Impact

- Archivos: `core/meltemid/src/{levels.rs, acp.rs, session_config.rs,
  free_session.rs, propose.rs, sdd_flow.rs, server.rs, session_finalize.rs,
  session_index.rs, fleet.rs}`, `core/meltemid/data/fleet-registry.toml`,
  `core/mock-agent/src/main.rs`, `proto/` (eventos y campos aditivos),
  `tui/src/{run.rs, shell/conn.rs}`, `desktop/ui/src/lib/` (conversación y
  textos), `docs/agentes.md`, `docs/sesion-libre.md`, `docs/paridad-nucleo.md`.
- Cero dependencias nuevas.
- Riesgo real: un viaje de ida y vuelta más antes del primer turno cuando se
  pide una palanca por anuncio; acotado y cancelable.
- Lo que solo la medición confirma: que OpenCode 1.18 honra un
  `set_config_option` a un modelo `ollama/…` y arranca con él.

## Fuera de alcance

- Traducir nombres de modelo entre proveedores, o un catálogo de modelos
  propio: el valor es del agente, opaco para Meltemi.
- **Recordar lo que cada agente anunció y un selector que lo ofrezca** (el chip
  de la GUI, la flota en las tres superficies): su clave, sus cotas y su
  contenido publicado son un diseño propio → `selector-de-modelo-que-sabe`.
- Cambiar de modelo a mitad de turno: `set-option` ya lo hace entre turnos.
- Palancas al reanudar con `session/direct`.
- Tokens y costo: `usage-desde-acp`.
