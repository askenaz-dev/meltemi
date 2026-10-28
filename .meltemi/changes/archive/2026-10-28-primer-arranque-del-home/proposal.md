# primer-arranque-del-home

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre `gui-shell`, cero
> dependencias nuevas, ningún movimiento del contrato ni del daemon, y cromo
> exclusivo de la GUI sin deber de paridad (la regla quedó escrita en
> `lanzador-conversacional`).

## Why

`lanzador-conversacional` convirtió la llegada en un compositor cuyos chips
son el asistente de configuración, en contexto y sin ceremonia. La mitad
reactiva quedó ejemplar y está verificada en `Home.svelte`: si el daemon
rehúsa resolver el agente, el rehúso no es un banner — entra al chip con los
candidatos, su estado de instalación y su remedio, y el menú se abre solo,
donde el usuario ya está mirando.

La mitad proactiva no existe. Con la flota sin un solo agente lanzable, la
cara del chip dice «agente del proyecto» en tono neutro — promete un default
que el envío va a rehusar — mientras el chip de proyecto, en la misma fila,
sí advierte con tono y texto cuando falta la carpeta: la asimetría está a la
vista. Dentro del menú vacío la pista ya es honesta («sin agentes detectados:
revisa la Flota», más la sugerencia de remedio), pero es un párrafo sin
gesto: nombra la vista de flota y no la abre. Y no hay momento de
reconocimiento: quien instala Meltemi con dos CLIs ya en el PATH — desde los
adaptadores empaquetados, el caso que queremos volver típico — no ve nunca un
«detectados: 2»; la detección que el catálogo ya hace queda muda hasta el
primer menú o el primer rehúso.

El comparativo del 2026-07-31 (Orca, de Stably AI, capturas del mantenedor)
puso el contraste delante. Su respuesta al primer arranque es un modal de
tres pasos más una checklist persistente de ocho ítems — coreografía que este
proyecto rechaza por recarga: nuestro home ya es el asistente, en su sitio.
Pero su gesto de reconocimiento («Detected on your system · 5») es
exactamente el que al compositor le falta, y cuesta la cara de un chip. Se
adopta el gesto, no la coreografía.

## What Changes

- **La cara del chip de agente dice la verdad antes de fallar**: con cero
  lanzables (ni detectados ni perfiles declarados), etiqueta «sin agentes
  detectados» y tono de advertencia — el mismo trato que el chip de proyecto
  ya da a la carpeta ausente. Se acabó prometer un default irresoluble.
- **El menú vacío gana el gesto**: además de la pista, abrir la vista de
  flota — donde viven la detección por capas y el remedio con su comando
  exacto. El ítem «agente del proyecto» no se ofrece como elegible cuando no
  hay nada que resolver.
- **Reconocimiento discreto**: en la primera llegada con lanzables, el chip
  muestra el recuento («N detectados») una sola vez; sin globos, sin tour,
  sin más persistencia que la de haberlo dicho.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «Primer arranque del compositor» — estado
  proactivo del chip de agente con flota vacía (advertencia en la cara del
  chip y gesto del menú hacia la vista de flota) y reconocimiento único del
  recuento de lanzables en la primera llegada. ADDED-only: ningún requisito
  vigente se enmienda.

## Impact

- Solo `desktop/ui`: cromo del compositor. Cero dependencias, cero contrato,
  cero daemon. La TUI no tiene deuda aquí: su onboarding de primer uso es
  requisito vigente de `tui-shell` desde `tui-nucleo-ux`.
- i18n es/en para las cadenas nuevas; los gates svelte-check e i18n-lint
  existentes las cubren.
- Tests de componente por escenario; el estado «flota vacía» es construible
  con los stores actuales — la lista completa del catálogo ya viaja al
  cliente con `detected` por entrada, así que no hay fixture nuevo que
  inventar.
- Riesgo: ninguno estructural. Lo único delicado es que el reconocimiento no
  se vuelva molestia — por eso es una vez y discreto, y el escenario lo fija
  así.

## Fuera de alcance

- **Agente por defecto global persistible desde superficies**: hoy el default
  es del proyecto y los chips eligen por sesión; un default global editable
  exigiría RPC nuevo y paridad ×3 — change propia si la evidencia la pide.
- **Wizard multi-paso, checklist persistente, milestones, tours**: la
  coreografía del comparativo, rechazada por escrito arriba.
- **Tocar la detección o el catálogo** (`fleet-catalog` queda como está): la
  lista y sus remedios ya son correctos; esto es jerarquía y gesto en el
  compositor.
- **El resto de estados vacíos de la GUI**: sesiones, changes y flota tienen
  los suyos desde `gui-clase-mundial`; esta change es el primer arranque del
  home, no una pasada general.
