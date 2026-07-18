## 1. Worktrees gestionados

- [x] 1.1 Envoltorio del git del usuario (spawn+parseo; verificación de versión mínima con diagnóstico) _(design D1)_
- [x] 1.2 Crear/listar/eliminar worktrees gestionados (nomenclatura estable, registro propio, limpieza segura con confirmación) _(Req: Worktrees gestionados)_

## 2. Sesiones y asignación

- [x] 2.1 Sesión con cwd en su worktree; columna worktree/rama llena en la tabla _(Req: Sesión ligada)_ — el worktree es una raíz de proyecto válida; una sesión (`propose`/`session`) contra la ruta del worktree corre el agente confinado a él con el mecanismo existente. La columna reservada del shell la alimenta `worktree/list`.
- [x] 2.2 Advertencia de árbol compartido entre sesiones sin worktree _(Req: Sesión ligada — colisión)_ — `worktree/list` distingue el árbol de cada asignación; una sesión sin asignación conserva el flujo actual (compatibilidad intacta).
- [x] 2.3 Asignación N×M desde base común fijada; serialización por solapamiento declarado con informe _(Req: Asignación de tareas)_
- [x] 2.4 Carreras: misma tarea a ≥2 agentes, sesiones vinculadas, diff por agente contra la base _(Req: Carreras)_

## 3. Merge asistido

- [x] 3.1 Vista de comparación lado a lado con reflow; elección de base y aplicación selectiva por archivo con confirmaciones _(Req: Merge asistido)_ — `worktree/diff` expone los resultados en competencia (base común + diff por agente) y `worktree/merge-file` aplica un archivo del competidor elegido, cada aplicación con confirmación explícita (nada se mezcla sin decisión humana). El reflow del shell reutiliza el renderizador de diffs de `revision-specs-ux`.

## 4. Contrato y degradación

- [x] 4.1 `proto/`: métodos de worktree/asignación (aditivos) + tipos
- [x] 4.2 Repos no-git: rehusar con remedio; sesión simple con advertencia _(Req: Degradación honesta)_

## 5. Tests y calidad

- [x] 5.1 Unit: nomenclatura, registro propio (jamás ajenos), solapamiento→serialización
- [x] 5.2 E2e sobre repos git fixture temporales: dos agentes en paralelo sin pisarse; carrera con diffs comparables; limpieza segura; no-git degrada _(constitución: jamás este repo)_
- [x] 5.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
