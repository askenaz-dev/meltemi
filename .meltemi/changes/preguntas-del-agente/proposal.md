# preguntas-del-agente

> Vía completa (proposal → design → specs → tasks). Dos frentes en una
> historia: la pregunta viva se acopla al compositor (GUI) y el adaptador de
> Claude deja de rehusar `AskUserQuestion` (relitiga dos decisiones escritas
> del adaptador — por eso el design es obligatorio). Depende de
> `redirigir-turno` para el escape de texto libre en agentes ACP nativos.

## Why

Cuando un agente pregunta —Claude, Codex y Copilot lo hacen a mitad de
trabajo: «¿cuál de estas dos rutas?», con opciones y una recomendada— el
usuario de Meltemi merece lo que esos productos dan: la pregunta **en la
misma caja donde escribe**, como un listado que se contesta con un clic o una
tecla, con una salida de texto libre al final.

Lo notable es cuánto existe ya. `session/request_permission` de ACP
transporta N opciones arbitrarias con nombre; el proto las conserva; la
bandeja las lista y la conversación las pinta como tarjeta con un botón por
opción, decidiendo por `permission/decide` — la misma cola, jamás una
segunda. Los avisos de escritorio ya llaman la atención cuando una sesión
espera. Lo que falta es **presentación** (la tarjeta queda arriba en el
transcript mientras el compositor solo dice «waiting») y **un emisor real de
preguntas ricas**.

Porque el emisor principal hoy rehúsa: el adaptador de Claude declara
`AskUserQuestion` interactive-only y lo deniega con razón escrita — «pregunta
en la interfaz del propio CLI, que una sesión headless no tiene». Esa
premisa caducó: Meltemi **es** la interfaz, y ya pinta opciones como botones.
Los propios tests del adaptador prueban que la pregunta llega por el canal de
permisos con su `tool_use_id`, y el canal de respuesta transporta
`updatedInput` — el cable para devolver la elección existe de punta a punta.
Levantar el rehúso toca además el principio «una llamada permitida corre
exactamente como se pidió»: la excepción tiene fundamento — en
`AskUserQuestion` el input **es el formulario**; el humano no reescribe lo
que el agente iba a hacer, completa lo que el agente vino a preguntar — pero
merece quedar escrita con ese fundamento, no colarse.

El texto libre tiene un límite de protocolo que se dice de frente: ACP solo
devuelve `Selected(optionId)` o `Cancelled`. Por el canal de Claude el texto
libre viaja en `updatedInput`; por ACP nativo no viaja — y ahí el escape
honesto es el verbo de `redirigir-turno`: la pregunta se resuelve cancelada y
el texto entra como relevo del turno. Misma UX, dos cables, cada uno con la
verdad de su protocolo.

## What Changes

- **La pregunta viva se acopla al compositor** (GUI): con la sesión en
  `waiting_permission`, la zona del compositor presenta la pregunta y sus
  opciones como listado navegable por teclado (números, flechas, Enter), con
  el nombre de cada opción tal como el agente lo mandó — si marcó una
  recomendada en su rótulo, se ve. Aparece **de golpe, sin transición**: la
  regla dura vigente (una permission jamás anima su layout) aplica entera.
  La tarjeta del transcript se queda — el log es la verdad; el compositor es
  el control vivo de la misma cola.
- **La última opción es siempre «Otra respuesta…»**: abre la caja de texto
  libre en el mismo sitio. En sesiones del adaptador de Claude viaja como
  `updatedInput`; en agentes ACP nativos ejecuta interrumpir-y-relevar
  (`redirigir-turno`) con la pregunta resuelta como cancelada — y si esa
  change no está desplegada, la salida se rotula como lo que es (encolar
  para el próximo turno).
- **El adaptador de Claude releva `AskUserQuestion`**: el rehúso
  interactive-only se levanta para esta herramienta; el input se parsea
  (preguntas, opciones con rótulo y descripción, multiSelect) y cada
  pregunta sale como `session/request_permission` con sus opciones; la
  elección vuelve por `updatedInput` con la forma que el CLI espera. La
  excepción al principio de input-intacto queda escrita en el módulo con su
  fundamento. La verificación contra el CLI real es manual y se documenta
  (CI jamás corre agentes reales).
- **El mock-agent aprende a preguntar**: un escenario con opciones y
  recomendada, para que e2e y demos ejerciten el flujo completo sin red.
- **La TUI no pierde**: su bandeja ya lista opciones con nombre y decide por
  el mismo verbo; el escape de texto libre le llega por `session/direct`
  como a la GUI. No hay capacidad nueva de daemon: no nace paridad nueva que
  servir, y la que existe se verifica.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: + requisito «Las preguntas del agente se relevan como
  peticiones con opciones» — el mapeo de `AskUserQuestion`, la respuesta por
  `updatedInput`, la excepción escrita al input-intacto, y el rehúso que se
  conserva para cualquier otra herramienta interactive-only.
- `gui-shell`: + requisito «La pregunta se contesta donde se escribe» — el
  acople al compositor, el teclado, la aparición sin animación de layout, el
  escape de texto libre con su verdad por protocolo.

## Impact

- Archivos: `core/meltemi-adapters/src/claude/permission.rs` (+ shim/gate
  donde el design lo pida), `core/mock-agent`, `desktop/ui`
  (`SessionDetail`), i18n es/en. `proto/` no se mueve: las opciones ya
  viajan.
- Cero dependencias nuevas.
- **Riesgo mayor: la forma exacta del `updatedInput`** que el CLI de Claude
  espera para `AskUserQuestion` no está especificada por nosotros y puede
  cambiar con su versión; el requisito de conformidad por versión de
  `own-adapters` ya cubre el desfase rehusado, y la validación manual se
  anota con la versión probada.
- Riesgo de UX: una pregunta con muchas opciones en un compositor angosto;
  el design fija el tope visual y el desbordamiento (scroll propio, jamás
  del panel).
- Codex hoy no emite preguntas por su canal (solo aprobaciones exec/patch);
  si su protocolo gana un equivalente, entra como seguimiento del adaptador,
  no aquí.

## Fuera de alcance

- **Texto libre sobre ACP puro** (sin relevo): exigiría extender el
  protocolo; §6 manda demostrar primero que ningún estándar lo cubre, y la
  extensión, si algún día se justifica, es change propia con esa prueba.
- **Multi-selección en una sola petición ACP**: el desenlace de ACP es una
  opción; el design decide si multiSelect se descompone en preguntas
  secuenciales o se rehúsa con mensaje honesto — pero la ambición queda
  acotada a lo que el cable dice.
- **Preguntas proactivas de Meltemi al usuario** (fuera de una sesión):
  otra naturaleza, otro dueño (el método), otra change.
- **Cambiar la bandeja de permisos**: sigue siendo la vista de cola completa;
  el compositor es un acceso más a la misma decisión, nunca una segunda
  cola.
