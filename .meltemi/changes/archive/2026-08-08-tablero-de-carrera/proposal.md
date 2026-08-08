# tablero-de-carrera

## Why

La carrera multiproveedor es la razón fundacional del producto («un rumbo,
muchas velas») y el daemon ya la corre entera: worktrees aislados por
competidor, despacho con el binario y la suscripción propios de cada
proveedor (`flota-multiproveedor`), procedencia persistida en los metadatos
de sesión (`multiproyecto-suscripciones`), checkpoints y commit trazable.
Pero **ninguna superficie la muestra como carrera**. La GUI tiene medio
tablero por accidente: el drill-in de revisión compara diffs de
competidores línea a línea, sin procedencia, sin estado de commit, sin
acciones y sin vida. El shell interactivo anuncia los diez verbos de
carrera como «(reservado)» y no renderiza ninguno: la casa existe, está
vacía. La CLI vuelca el diff crudo. La demostración de aceptación del hito
lo dejó en evidencia: para *ver* la carrera que el daemon corría hubo que
construir un tablero externo desechable. El ojo del producto sobre su
propia feature fundacional no existe.

Debajo del hueco visual hay tres huecos de contrato que ninguna superficie
puede rodear: (1) el resultado del despacho no nombra la sesión que abrió
(`WorktreeDispatchResult` sin `session_id`) y los metadatos de sesión no
nombran cambio/tarea/agente, de modo que **ninguna calle puede correlacionar
ni observar su turno**; (2) el diff por competidor no trae procedencia —
quién corrió la calle, con qué perfil, a qué nivel, si comiteó — aunque el
daemon la conoce y la registra en el log; (3) las sesiones de despacho no
asientan registro en el índice de sesiones: aparecen en el histórico solo
por reconstrucción desde logs, con nivel por defecto equivocado. El tablero
no puede inventar lo que el contrato calla.

## What Changes

- **Procedencia por calle en el contrato (aditivo, sin verbo nuevo)**: el
  diff por competidor (`worktree/diff`) gana campos opcionales por calle —
  fuente de resolución, perfil, nivel, sesión del último despacho, estado
  de commit (sha) y base propia de la calle —; el resultado del despacho
  gana la sesión que abrió. Todo `Option`/omitible: un cliente anterior
  recibe bytes idénticos a los de hoy. Sin método nuevo, la matriz de
  paridad no cambia de filas.
- **El despacho asienta registro de primera clase**: la sesión de un
  despacho escribe registro de apertura y cierre en el índice de sesiones,
  con su nivel real y su procedencia (id de catálogo, perfil), como ya
  hacen los demás corredores; la reconstrucción desde logs queda como red
  de seguridad, no como única vía.
- **Tablero de carrera en la GUI**: evolución del drill-in de revisión
  existente — las calles de los competidores lado a lado con procedencia
  visible (agente, perfil/suscripción), diff contra la base, estado del
  turno/commit/checkpoint, y las acciones de la carrera (despachar turno,
  revertir al checkpoint, commit, merge asistido) con los formularios
  tipados y la confirmación explícita ya establecidos. Se actualiza al
  concluir turnos propios vía el stream de eventos de sesión; sin recargar.
- **Tablero de carrera en el shell (TUI)**: la superficie que hoy no
  existe. Alcanzable desde la paleta (el verbo `race` deja de estar
  reservado) sin alterar el contrato de dígitos 1–4; calles con estado en
  glifo+palabra, diff legible con el desplazamiento del shell, y el
  despacho corriendo aparte del bucle de refresco para no congelar el
  shell (el patrón ya probado por la dirección de sesiones).
- **CLI enriquecida gratis**: `race` renderiza los campos nuevos por calle
  (procedencia, sesión, sha) — misma verdad en las tres superficies (§4).
- **Docs en lockstep**: matriz de paridad con los punteros de vista
  actualizados y smoke visual CDP de la superficie nueva publicado en
  `docs/qa/`, como el método de cierre de la GUI estableció.

## Capabilities

### Modified Capabilities

- `worktree-orchestration`: + la carrera consultable con procedencia —
  campos aditivos por calle (fuente/perfil/nivel/sesión/sha/base) y la
  sesión nombrada en el resultado del despacho.
- `session-history`: + el despacho deja registro de primera clase en el
  índice de sesiones (nivel y procedencia reales; reconstrucción como red
  de seguridad).
- `gui-shell`: + el tablero de carrera como evolución del drill-in de
  revisión (calles, procedencia, acciones con confirmación, actualización
  al concluir turnos).
- `tui-shell`: + el tablero de carrera en el shell, alcanzable desde la
  paleta sin tocar el contrato de dígitos; degradación ASCII sin pérdida
  de significado.

## Impact

- `proto/`: campos aditivos en `WorktreeCompetitorDiff` y
  `WorktreeDispatchResult` (+ esquemas + conformidad de tres vías: con
  campo, sin campo, y byte-igualdad de la forma omitida). Sin método nuevo.
- `core/meltemid`: agregación de procedencia en el handler del diff
  (composición de estado ya persistido, precedente de `navigate.rs`);
  registros de índice en el despacho; sin transporte nuevo, sin mutación.
- `desktop/ui`: el drill-in de revisión evoluciona a tablero (extracción
  del parser de diffs a módulo compartido; i18n ES/EN; cero dependencias
  npm nuevas, §10).
- `tui`: verbo `race` des-reservado con su superficie, variantes de
  Effect/Command/Update, render con paneo existente y tests de reducer,
  render y e2e vivo.
- Verificación: escenarios enlazados a tests por marcador (GUI vía
  aserciones de fuente en `desktop/tests/scenarios_shell.rs`, patrón
  establecido); smoke CDP publicado; gates de las tres plataformas.

## Fuera de alcance

- **Canal de eventos de worktree** (suscripción push a diffs): el tablero
  vive con el tick de 2 s del shell, el stream de sesión propio y la
  re-consulta al concluir turnos. Si la práctica lo pide, será su propia
  change (`eventos-para-tardios` ya difiere las formas asíncronas de
  `worktree/dispatch`; esta change no las reabre).
- **Merge automático o puntuación de competidores**: el merge sigue siendo
  decisión humana explícita (spec vigente); el tablero lo hace visible, no
  lo decide.
- **Sexta vista numerada**: ni la GUI ni el TUI amplían su contrato de
  vistas de primer nivel; el tablero entra por las puertas existentes
  (drill-in de revisión; paleta).
- **El tablero HTML de la demo**: fue un artefacto de prueba desechable;
  nada de él se comitea.
