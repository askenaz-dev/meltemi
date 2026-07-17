## 1. Worktrees gestionados

- [ ] 1.1 Envoltorio del git del usuario (spawn+parseo; verificación de versión mínima con diagnóstico) _(design D1)_
- [ ] 1.2 Crear/listar/eliminar worktrees gestionados (nomenclatura estable, registro propio, limpieza segura con confirmación) _(Req: Worktrees gestionados)_

## 2. Sesiones y asignación

- [ ] 2.1 Sesión con cwd en su worktree; columna worktree/rama llena en la tabla _(Req: Sesión ligada)_
- [ ] 2.2 Advertencia de árbol compartido entre sesiones sin worktree _(Req: Sesión ligada — colisión)_
- [ ] 2.3 Asignación N×M desde base común fijada; serialización por solapamiento declarado con informe _(Req: Asignación de tareas)_
- [ ] 2.4 Carreras: misma tarea a ≥2 agentes, sesiones vinculadas, diff por agente contra la base _(Req: Carreras)_

## 3. Merge asistido

- [ ] 3.1 Vista de comparación lado a lado con reflow; elección de base y aplicación selectiva por archivo con confirmaciones _(Req: Merge asistido)_

## 4. Contrato y degradación

- [ ] 4.1 `proto/`: métodos de worktree/asignación (aditivos) + tipos
- [ ] 4.2 Repos no-git: rehusar con remedio; sesión simple con advertencia _(Req: Degradación honesta)_

## 5. Tests y calidad

- [ ] 5.1 Unit: nomenclatura, registro propio (jamás ajenos), solapamiento→serialización
- [ ] 5.2 E2e sobre repos git fixture temporales: dos mock-agents en paralelo sin pisarse; carrera con diffs comparables; limpieza segura; no-git degrada _(constitución: jamás este repo)_
- [ ] 5.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
