# Tasks — modelo-por-opcion-anunciada

Toda tarea cuyo gate arranca el simulado corre antes `cargo build --workspace`:
los e2e cargan el binario `mock-agent` ya construido.

## 1. El contrato y la flota

- [ ] 1.1 Contrato: `FleetAgent.levers` (las palancas declaradas; ausente = sin
  declaración) y `FleetAgent.announced` (fecha y opciones tal cual), y el evento
  `levers_applied` con el valor que el agente confirmó; schemas y casos de
  conformidad (design D2, D3, D6) — gates: `cargo build --workspace`, suite de
  `meltemi-proto`

- [ ] 1.2 El registro declara las palancas: campo `levers` (conjunto completo
  de la entrada; un valor desconocido es diagnóstico), `CatalogEntry`,
  `fleet/list` lo devuelve, y la regla por nombre de binario (`levers_of`) se
  sustituye por la declaración de la entrada; el registro embebido declara
  Codex (`model`, `effort`) y Claude Code (`model`) (design D2) — escenarios
  «Una entrada declara sus palancas de lanzamiento», «Sin declaración la
  palanca es lo que el agente anuncia», «El nombre del binario no decide» —
  gates: suite de `meltemid`

- [ ] 1.3 La flota recuerda lo último anunciado: `fleet-announced.json` en el
  directorio de datos, escrito en cada apertura que anuncia con un agente de la
  flota, y `fleet/list` lo devuelve por entrada (design D6) — escenario «El
  anuncio se recuerda por agente» — gates: suite de `meltemid`

## 2. Las palancas llegan al agente

- [ ] 2.1 `run_session` fija las palancas por anuncio antes del primer prompt:
  opción por categoría (`model`, `thought_level`), validación con
  `chosen_value`, espera acotada, `levers_applied` con lo confirmado; sin la
  categoría o sin el valor, termina sin prompt con un error tipado que lista lo
  anunciado; unit tests de la búsqueda y de los textos (design D3, D4) —
  escenarios «La opción anunciada fija el modelo antes del primer prompt», «Un
  valor no anunciado se rehúsa listando lo anunciado», «Sin bandera ni anuncio
  la palanca se rehúsa al abrir», «El recuerdo no decide» — gates: `cargo
  build --workspace`, suite de `meltemid`

- [ ] 2.2 La sesión libre reparte las palancas por vía (D1): rehúso antes de
  crear nada para el conjunto cerrado y los niveles 3/4; el rehúso al abrir se
  vuelve el error del verbo, con lo anunciado, y la sesión termina
  `lever_refused`; el índice y `session/list` toman lo confirmado; las pruebas
  de palancas de hoy pasan a un registro de prueba (design D4, D8) — escenarios
  «El modelo pedido viaja sin interpretarse», «Lo que rigió queda en el
  registro», «Una palanca que el agente no admite se rehúsa» — gates: suite de
  `meltemid`

- [ ] 2.3 Las palancas del perfil en todo verbo que arranca un agente
  (`propose`, los verbos de autoría, `dispatch`, `implement`), con e2e de
  `propose` y `dispatch` (design D5) — escenarios «El modelo del perfil rige en
  todo verbo que arranca un agente», «La sesión pisa el default del perfil»,
  «Un perfil sin declaración no impone nada» — gates: suite de `meltemid`

## 3. Superficies y documentación

- [ ] 3.1 `meltemi fleet` y la vista Flota de la TUI dicen por agente su
  palanca por bandera y lo último que anunció, con cuántos valores y cuándo
  (design D7) — escenarios «El CLI dice las palancas de cada agente», «La TUI
  dice las palancas de cada agente» — gates: suite de `meltemi` (tui)

- [ ] 3.2 El selector de modelo del compositor de la GUI según el agente
  elegido: lo anunciado (con búsqueda y aviso para lo escrito fuera de la
  lista), por bandera, «no toma modelo», o «se confirma al arrancar»; textos en
  español e inglés (design D7) — escenarios «El selector ofrece lo que el
  agente anunció», «Un agente que no toma modelo lo dice», «Se elige con
  búsqueda y se admite entrada libre», «La ficha no inventa lo que no sabe»,
  «Cambiar en marcha se advierte» — gates: suite de `meltemi-desktop`,
  `svelte-check` y `node --test` de `desktop/ui`

- [ ] 3.3 Documentación: `docs/agentes.md` (las palancas por agente y sus
  vías), `docs/sesion-libre.md` (`--model`/`--effort`), la ayuda de `session` y
  `docs/referencia-cli.md` regenerada, `docs/paridad-nucleo.md` (los campos
  nuevos de `fleet/list` y el evento) — gates: tests de docs

## 4. Cierre

- [ ] 4.1 `meltemi validate modelo-por-opcion-anunciada` limpio, `meltemi
  verify modelo-por-opcion-anunciada` completo, gate del workspace (`cargo test
  --workspace --no-fail-fast`, clippy, fmt), el daemon que arranquen
  `validate`/`verify`/`land` detenido, estado de la change en
  `docs/plan-de-cambios.md`, `meltemi land modelo-por-opcion-anunciada` y
  `cargo clean` del taller
