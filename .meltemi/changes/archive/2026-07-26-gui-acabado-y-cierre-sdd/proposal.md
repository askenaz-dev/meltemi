# gui-acabado-y-cierre-sdd

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Aprobada por el mantenedor el 2026-07-26 tras la evaluación conducida de la
> GUI (driver CDP sobre el webview, fixture temporal con el mock-agent).

## Why

La primera evaluación conducida de la app de escritorio — recorrido real de
las seis vistas, un `sdd/propose` de punta a punta contra el mock-agent,
edición y guardado — encontró cuatro defectos que ningún test podía ver,
porque los tests de la superficie verifican cableado leyendo el código y el QA
manual mide presupuestos sin mirar la ventana. Los cuatro son de acabado en el
sentido serio: no rompen ninguna función, pero rompen la percepción de
producto terminado en el primer minuto de uso, y uno de ellos además falsea el
histórico y la analítica.

1. **El shell no llena la ventana.** La columna central usa una rejilla de
   cinco filas fijas (`auto auto auto 1fr auto`) que asume presentes el banner
   de daemon caído y los avisos. En el estado más común — daemon conectado,
   sin avisos — solo hay tres hijos, la vista cae en una fila `auto` y la
   barra de estado flota a un tercio de la ventana con el resto en negro.
2. **El árbol del editor lista `.git` entero** (hooks, objects, refs: 314
   entradas de ruido en un repo pequeño), porque `repo/map` camina con
   `hidden(false)` para mostrar `.meltemi/` y arrastra el metadirectorio,
   consumiendo además presupuesto de truncado. La ruta de búsqueda ya lo
   excluye; el mapa no.
3. **Las filas del árbol se recortan verticalmente**: son hijos de una columna
   flex sin `flex-shrink: 0`, y con árboles largos se comprimen por debajo de
   su altura de línea (13.5px medidos contra 18.85px de línea).
4. **Toda sesión de autoría SDD termina listada como «interrumpida»** aunque
   complete con éxito, porque `run_turn` (camino común de explore,
   constitution, propose, plan y las reelaboraciones de gate) da de baja la
   sesión sin pasar por el finalizador compartido: sin `session_ended` no hay
   `ended_at`, el listado clasifica la sesión como interrumpida y la analítica
   la cuenta con 0s de tiempo activo (medido: una sesión de 1m 20s leyó
   ACTIVE TIME 0s en el panel de Consumo).

## What Changes

- `desktop/ui/src/App.svelte`: la columna central pasa de rejilla de filas
  fijas a columna flex — cada barra a su altura natural, la vista enrutada se
  queda con el resto. Invariante ante barras condicionales presentes o
  ausentes.
- `desktop/ui/src/lib/views/Editor.svelte`: las filas del árbol y de los
  resultados de búsqueda declaran `flex: 0 0 auto` — nunca se comprimen por
  debajo de su línea; el excedente se desplaza.
- `core/meltemid/src/repo_map.rs`: el walker de `repo/map` filtra `.git` en
  cualquier nivel. `.meltemi/` y demás contexto oculto siguen presentes. La
  paridad es doble: el mismo mapa sirve a la GUI y a la TUI.
- `core/meltemid/src/sdd_flow.rs`: `run_turn` escribe el registro de inicio en
  el índice y finaliza por `session_finalize::{finalize_ok,finalize_err}`,
  exactamente como `propose` y `session/direct`. Una autoría completada lista
  como finalizada; una fallida cierra con razón de error; el tiempo activo
  cuenta.

## Capabilities

### Modified Capabilities

- `gui-shell`: + requisito «Lienzo completo del shell» (alto de la ventana y
  filas sin recorte).
- `repo-context`: + requisito «Metadirectorio de git fuera del mapa».
- `sdd-authoring`: + requisito «Cierre de sesión de los turnos de autoría».

## Impact

- Superficies: GUI (layout del shell y árbol del editor); TUI se beneficia del
  mapa limpio sin cambio propio.
- Daemon: `repo/map` y el ciclo SDD; el contrato `proto/` no cambia (ningún
  campo nuevo, ningún método nuevo).
- Histórico y analítica: las sesiones de autoría pasan a cerrar honestamente;
  las ya escritas sin `session_ended` siguen listando como interrumpidas — no
  se reescribe historia.
- Tests: dos tests de cableado en `desktop/tests/scenarios_shell.rs`, un
  unitario en `repo_map.rs`, dos e2e en `core/meltemid/tests/e2e_sdd.rs`
  (cierre en éxito y en fallo). Verificación visual conducida por CDP
  publicada en `docs/qa/`.

## Fuera de alcance

- Acciones sobre el resultado de un verbo SDD en la GUI (botones de gate en
  vez de JSON crudo), log de sesión narrativo, orden/filtro de tablas,
  `project/forget` para proyectos con raíz ausente: mejoras reales apuntadas
  por la misma evaluación, cada una con su change.
- Smoke visual automatizado en CI: esta change publica el método (driver CDP)
  y el QA manual; convertirlo en gate de CI es una change propia.
