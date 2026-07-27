## Why

El mantenedor probó el lanzador actual y lo llamó «ordinario», y el mockup
que siguió terminó de fijar la dirección: tres directivas que esta change
convierte en una sola propuesta, superseding los borradores
`sesion-conversacional` y `gestion-proyectos-en-superficie`, cuyo contenido
absorbe. La auditoría de la GUI describe el presente sin maquillaje: el
modal NewSession dispara uno de los cuatro RPC existentes y se cierra con
un aviso **sin navegar a la sesión creada**; el drill-in de sesión es un
transcript vivo de solo lectura (`session/log` + `session/watch`) **sin
campo de entrada**, de modo que cada instrucción de seguimiento es un viaje
de ida y vuelta por el modal; y no existe vía alguna para añadir un
directorio como proyecto que no sea arrancar la app dentro de él o correr
sesiones ahí. La dirección final es la de los escritorios que el usuario ya
conoce (Claude, Codex): un **home conversacional** — compositor al centro,
contexto como chips dentro de él — que al enviar navega hacia adentro de la
conversación; **modo libre como default** — una sesión nueva es, por
defecto, una sesión libre gobernada sobre el proyecto elegido, y el método
es opt-in en el mismo compositor —; y **los proyectos en la navegación
lateral**, siempre visibles, con sus sesiones anidadas y un «Abrir
carpeta…» que hoy no existe en ninguna parte.

La segunda directiva obliga a nombrar una tensión de producto y resolverla
de frente, no a escondidas. Hoy no existe RPC para iniciar una sesión
libre: `session/direct` exige una sesión existente (`SessionDirectParams`
requiere `sessionId`), `propose` y `sdd/explore` son verbos del método, y
`worktree/dispatch` exige change y tarea. Y la promesa pública dice lo
contrario del default pedido: el rumbo de producto vende «bajo una
disciplina donde ninguna línea de código se escribe sin una especificación
revisada» (`rumbo/product.md`) y la tesis de meltemi.md promete un entorno
«donde ninguna línea de código se escribe sin una especificación revisada
primero». La resolución honesta no es esconder la contradicción sino
enmendarla: la sesión libre queda **gobernada siempre** — proxy de permisos
con deny-by-default, aislamiento y checkpoints, log de sesión apend-only:
eso es constitución (§3) y no se negocia — pero **no spec-gated**. El
método deja de ser cerrojo y pasa a ser la propuesta de valor de Meltemi
por sesión: Proponer y Explorar a un gesto en el mismo compositor, nunca un
peaje previo. La constitución de este repositorio no se toca — el
desarrollo de Meltemi sigue siendo spec-first (§1) —; lo que se enmienda es
la redacción de la promesa de producto, con texto propuesto en el design y
**ratificación del mantenedor como gate**, igual que toda enmienda
fundacional anterior.

La arquitectura es deliberadamente austera: la conversación es composición
del lado cliente sobre verbos que el daemon ya habla — `session/direct`
encola o reanuda, y `eventos-para-tardios` (2026-07-26) convirtió el stream
en un bien por sesión que cualquier conexión puede pedir; esa change es el
habilitador. Al daemon entran únicamente los verbos que la dirección
genuinamente no tiene: el arranque de sesión libre, `project/register` y
`project/forget`, más parámetros aditivos de agente donde ya hay
resolución (`resolve_fleet_agent` existe). Todo lo daemon-side llega con
paridad ×3 sin nota al pie (§4); el cromo exclusivo de la GUI — layout del
nav, diálogo nativo — no tiene deber de paridad, pero ninguna capacidad
nueva del daemon queda accesible desde una sola superficie.

## What Changes

- **Home conversacional en lugar del modal NewSession**: la vista de
  llegada es un compositor al centro con el contexto como chips dentro de
  él — proyecto (con «Abrir carpeta…» en el propio chip), agente y modo. El
  modo default es **Libre**; Proponer y Explorar son modos opt-in del mismo
  compositor (el método como oferta, no como peaje); dispatch conserva su
  entrada desde la superficie del método (vista Proyecto / tareas de una
  change). Enviar **navega hacia adentro** de la sesión creada — nunca más
  «lanzada» + modal que se cierra. Todos los puntos de entrada actuales
  (Ctrl+N, top bar, estados vacíos, «Propose» de la vista Proyecto) rutean
  al compositor con el contexto prefijado; el modal se retira.
- **Vista de conversación** sobre el detalle de sesión: compositor
  persistente que envía por `session/direct` con la sesión prefijada, y
  transcript con **render conversacional** — burbujas de turno plegadas
  sobre el log de eventos (`prompt_sent`, `agent_update`,
  `turn_completed`), **no en su lugar**: el log de operador sigue
  disponible con un conmutador, porque el transcript es la verdad y las
  burbujas son una lectura de ella. Las peticiones de permiso se renderizan
  **en línea como tarjetas**, porque son parte del diálogo — misma cola del
  proxy, mismos RPC de decisión, otra vista. Estados honestos con la
  sesión: una terminada no ofrece enviar (ofrece reanudar), una ocupada
  dice qué hace, y como `session/direct` encola, la interfaz dice
  «encolada» con su posición — jamás simula que la instrucción fue
  atendida.
- **Verbo nuevo de sesión libre** (forma final en el design: `session/start
  {projectRoot, instruction, agent?}` o `session/direct` sin `sessionId`
  que crea una): arranca una sesión gobernada sin change, sin spec y sin
  gate. No relaja ni una pieza del gobierno: resolución de agente por
  `resolve_fleet_agent`, proxy de permisos con deny-by-default, política
  vigente de aislamiento y checkpoints, log JSONL apend-only y hub de
  eventos — compone maquinaria existente; lo único nuevo es la puerta de
  entrada. El design decide dónde opera la sesión libre (raíz del proyecto
  como direct/resume hoy, o worktree por defecto) y lo deja escrito, no
  implícito.
- **Enmienda de la promesa de producto**, nombrada con sus frases exactas:
  «bajo una disciplina donde ninguna línea de código se escribe sin una
  especificación revisada» (`rumbo/product.md`, párrafo «Qué es Meltemi») y
  «donde ninguna línea de código se escribe sin una especificación revisada
  primero» (meltemi.md, tesis). El design propone la redacción que dice la
  verdad nueva — toda sesión corre gobernada; el método spec-first es la
  disciplina opt-in que Meltemi hace deseable — y el mantenedor la ratifica
  como toda enmienda fundacional; sin ratificación, la change no archiva.
- **Proyectos en la navegación**: sección «Proyectos» persistente en el
  sidebar, estilo escritorio de Codex/Claude — todo proyecto registrado
  listado con sus sesiones anidadas, siempre visible, no solo un modal
  switcher; clic en un proyecto conmuta el ámbito; acción rápida por
  proyecto para iniciar una sesión ahí (al compositor, con el chip
  prefijado); «Abrir carpeta…» en el nav y en el chip del compositor, con
  el diálogo nativo del SO vía el **plugin oficial de diálogo de Tauri** —
  dependencia nueva del cliente, justificada en el design (§10); el set de
  dependencias de meltemid no se mueve. En el daemon, los dos métodos que
  el registro nunca tuvo: `project/register` — alta explícita de un
  directorio, validado que existe y canonicalizado antes de entrar — y
  `project/forget` — baja **solo del registro, jamás toca el disco**: una
  línea de olvido en el JSONL apend-only que el plegado last-wins resuelve;
  un proyecto olvidado que vuelve a usarse o registrarse simplemente
  reaparece. La frontera de §3 explícita: el diálogo vive en el cliente; el
  daemon recibe una ruta y la valida, no abre ventanas ni sondea nada fuera
  de ella.
- **Selector de agente en todas partes**: parámetro `agent` opcional y
  aditivo en el verbo de sesión libre **y** en `propose` y `sdd/explore`,
  resuelto por el orden existente de `resolve_fleet_agent` (perfil > id de
  catálogo > configurado; un perfil o id que resuelve a binario no
  detectado rehúsa y jamás degrada en silencio). Y el error de resolución
  se vuelve estructurado: en vez del string crudo «neither `agent.command`
  nor `agent.id` is configured» (levels.rs), un error de contrato con los
  **candidatos detectados** de la flota, para que toda superficie pueda
  ofrecer elegir en vez de transcribir un lamento.
- **Paridad ×3 de todo lo daemon-side**: verbos CLI para el arranque libre
  y `meltemi project register|forget`, flag `--agent` donde el contrato lo
  gana; en la TUI, cablear interactivo el verbo `direct` que la paleta
  lista como reservado — anunciado, nunca cableado — con entrada de
  instrucción desde el drill-in y los mismos estados honestos, más el
  render del registro de proyectos y sus formularios de alta/baja (la ruta
  se tipea: es lo que la TUI tendrá de todos modos). El verbo scriptable
  `meltemi direct` queda intacto; `docs/paridad-nucleo.md` se actualiza en
  la misma change.

## Capabilities

### New Capabilities
- `free-session`: el verbo de sesión libre gobernada — sin change ni gate
  de spec, con proxy, aislamiento, checkpoints y log completos; selección
  aditiva de agente y error de resolución estructurado con candidatos.
- `conversational-session`: home conversacional y vista de conversación —
  compositor persistente sobre `session/direct`, render de turnos con
  conmutador al log de operador, tarjetas de permiso en línea, estados
  honestos de compositor; en la TUI, el `direct` interactivo del drill-in.

### Modified Capabilities
- `project-registry`: + `project/register` (alta validada y canonicalizada)
  y `project/forget` (baja solo-registro por plegado last-wins); + sección
  Proyectos persistente y acciones rápidas por proyecto en las superficies.
- `propose-flow`: + parámetro `agent` aditivo con resolución de flota.
- `sdd-authoring`: + parámetro `agent` aditivo en `sdd/explore`.
- `fleet-catalog`: + error de resolución estructurado con candidatos
  detectados en lugar del string crudo.
- `gui-shell`: home conversacional reemplaza el modal de lanzamiento; nav
  con Proyectos; navegación al enviar.
- `tui-shell`: `direct` deja de ser verbo reservado; formularios y render
  de proyectos en la paleta.
- `cli-contract`: verbos nuevos (`start` de sesión libre, `project
  register|forget`) y flag `--agent`; nombres finales en el design.

## Impact

- Contrato: métodos y campos aditivos en `proto/` (params/result del verbo
  de sesión libre, `agent` en `ProposeParams`/`SddExploreParams`,
  `project/register`/`project/forget`, payload estructurado del error de
  resolución) con sus tests de conformidad. Ningún campo existente cambia
  de forma.
- `core/meltemid`: el handler de sesión libre compone maquinaria existente
  (resolución, proxy, log, hub de eventos); el registro de proyectos gana
  alta y línea de olvido con plegado last-wins; ningún RPC nuevo para
  descubrimiento conversacional — el stream por sesión ya existe.
- `desktop/ui`: el home reemplaza a NewSession.svelte; SessionDetail
  evoluciona a vista de conversación (compositor, burbujas, conmutador,
  tarjetas de permiso); Sidebar gana la sección Proyectos; **dependencia
  nueva del cliente**: plugin oficial de diálogo de Tauri, justificada en
  el design (§10), confinada al shell — el daemon no gana dependencias.
- `tui/`: paleta (direct interactivo, proyectos), CLI (verbos y flag),
  `docs/paridad-nucleo.md` actualizada; la asimetría vigente de modelo de
  ámbito (filtro textual en la TUI, raíz activa persistida en la GUI) no se
  toca — solo el acceso a los métodos es parejo.
- Fundacionales: enmienda de redacción en `rumbo/product.md` y meltemi.md,
  ratificada por el mantenedor; sin ella la change no archiva. La
  constitución no se modifica.
- Tests: e2e del verbo de sesión libre contra mock-agent (nunca agentes
  reales ni red); escenarios de plegado register/forget sobre el JSONL;
  escenarios de TUI por spec; el smoke visual CDP de la GUI — el método que
  `gui-acabado-y-cierre-sdd` probó — cubre home, conversación y nav, porque
  el cableado por sí solo ya demostró que no ve estos bugs.
- Honestidad de render asumida y nombrada: las burbujas son una lectura del
  log, no otra fuente; todo lo que el plegado no sepa clasificar cae al log
  de operador visible, jamás se omite. La analítica no cambia.
- Supersede: los borradores `sesion-conversacional` y
  `gestion-proyectos-en-superficie` quedan absorbidos aquí y no entran al
  backlog como changes propias.

## Fuera de alcance

- `menu-nativo-aplicacion`: change propia, ya separada — aquí no se toca el
  menú del shell.
- `registro-agentes-en-superficie`: change propia — **registrar** agentes
  es otra cosa que **seleccionarlos**; esta change solo selecciona sobre la
  flota ya detectada.
- Render conversacional de sesiones pasadas/archivadas más allá del
  conmutador de log: el histórico se lee como log de operador; promoverlo
  es futuro con evidencia.
- Cambio de modo, modelo o harness in-sesión: los session modes de ACP no
  están cableados; exclusión ya escrita en `motor-propio-byok` y
  re-afirmada aquí.
- Formas asíncronas de `sdd/gate`, `sdd/review-decide` y
  `worktree/dispatch`: diferidas con razón escrita en
  `eventos-para-tardios`; esta change no las reabre.
- Fijar u ordenar proyectos en el nav (pin): futuro con evidencia de uso;
  el orden inicial es el del registro.
- Reescritura de historia: `project/forget` jamás borra sesiones ni toca el
  disco; qué ocultan los listados de un proyecto olvidado se decide en el
  design, los datos se conservan siempre.
- Toda descarga, plantilla o galería de proyectos: el daemon jamás descarga
  nada; registrar es validar una ruta local.
