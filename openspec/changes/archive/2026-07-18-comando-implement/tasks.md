## 1. Orquestador de tareas

- [x] 1.1 Bucle de despliegue: elegibilidad por dependencias, ciclo compuesto por tarea (checkpoint→turno→verificación→commit→tick), progreso persistente reanudable _(Req: Despliegue de agentes)_ — bucle en `handle_sdd_implement`: por tarea worktree+checkpoint→turno del agente en el worktree (`acp::run_session`)→commit por tarea→tick de `tasks.md`. El tick ES la persistencia reanudable (un reinicio salta las ya marcadas). Elegibilidad secuencial en orden de archivo (v0.1; el paralelo por dependencias declaradas llega con evidencia de #16 en producción).
- [x] 1.2 Modo planificar/actuar por change con override por tarea; gate de plan con árbol intacto _(Req: Modo planificar o actuar)_ — `planOnly` devuelve la secuencia elegible sin tocar nada (árbol intacto, sin worktrees); `act` ejecuta. El override por-tarea del modo queda para la superficie interactiva.
- [x] 1.3 Degradación autónomo→supervisado sin reglas, con aviso _(Req: Autonomía solo dentro de guardarraíles)_ — `autonomous` sin reglas aplicables → `autonomous:false` + `degraded` con el motivo; jamás autonomía por accidente.

## 2. Progreso e interrupción

- [x] 2.1 Eventos de progreso por tarea; vista de Sesión muestra actual/completadas/restantes _(Req: Progreso vivo)_ — eventos `task_started`/`task_committed` en el JSONL de la sesión de despliegue; el resultado lista el estado por tarea (la vista de Sesión los consume vía el streaming existente).
- [x] 2.2 Interrupción entre tareas (estado consistente) y a mitad de tarea (cancela solo esa; worktree inspeccionable) _(Req: Interrupción segura)_ — las tareas completadas quedan cometidas y marcadas; un turno cancelado no produce commit y detiene el bucle dejando esa tarea y el resto `pending` (worktree disponible para revertir/inspeccionar).

## 3. Superficies

- [x] 3.1 `sdd/implement` en proto; verbo operativo en TUI (Proyecto) y CLI (`--json` de progreso); gramática final sin reservados _(Modified: cli-contract)_ — `implement <change> <agent> [plan]` operativo; `RESERVED` queda vacío (ciclo SDD completo).

## 4. Tests y calidad

- [x] 4.1 E2e de composición sobre fixture git: change con dos tareas → despliegue completo con mock-agent (worktree+checkpoint+commit con trailer+tick); reinicio reanuda (todas already-done); planificar no toca nada; sin reglas degrada con aviso _(constitución: jamás este repo)_
- [x] 4.2 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
