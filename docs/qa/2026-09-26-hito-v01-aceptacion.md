<!-- SPDX-License-Identifier: Apache-2.0 -->
# Informe de aceptación v0.1 — corrida manual con agentes reales

**Fecha**: 2026-09-26 00:22–00:42 UTC · **Producto**: `meltemi 0.1.1`, rama
`corrida-integrada` sobre `main` `b4a6d1b` · **Plataforma**: Windows 11 Pro ·
**Agentes**: Claude Code 2.1.280 (nivel 2, por el adaptador propio
`meltemi-claude-acp`) y OpenCode 1.18.32 (nivel 1): dos proveedores distintos ·
Daemon, datos y configuración aislados (las cinco variables); un repositorio
fixture temporal con un proyecto Python mínimo (`src/greet.py`).

La idea: *añadir una función `farewell(name)` junto a `greet` que devuelva
`"goodbye {name}"`*.

Llega tarde: `v0.1.0` ya estaba publicada, y la spec pide que este informe
acompañe al tag. Se corrió sobre el producto de hoy, no sobre el del tag, y así
se lee.

## Qué se ejecutó

Todo por los métodos del producto. Los pasos sin verbo en la CLI —el ciclo de
autoría (`sdd/propose`, `sdd/gate`), la decisión de revisión
(`sdd/review-decide`) y la marca manual de verificación (`sdd/verify-mark`)—
se llamaron por JSON-RPC sobre el endpoint del daemon, que es el contrato que
la TUI y la GUI usan (en la matriz de paridad figuran con «—» en la columna de
la CLI).

| Paso | Método | Agente | Resultado | Tiempo |
| --- | --- | --- | --- | --- |
| 1 | `sdd/propose` | Claude Code | `proposal.md` escrito; gate pendiente | 67 s |
| 2 | `sdd/gate` approve | Claude Code | `specs/greet/spec.md` escrito | 57 s |
| 3 | `sdd/review` + `sdd/review-decide` comment sobre «Farewell message function»: *«Add a scenario for an empty name»* | Claude Code | la spec se reelaboró: ganó «Empty name» y «Empty name is handled identically by greet and farewell» | 67 s |
| 4 | `sdd/review-decide` approve × 4 requisitos | — | revisión cerrable | — |
| 5 | `sdd/gate` approve × 3 | Claude Code | `design.md` y `tasks.md` escritos; ciclo `completed` | 119 s |
| 6 | `worktree/assign` 1.1 → Claude Code y OpenCode, y dos `worktree/dispatch` en paralelo | ambos | turnos completos, **sin commit**: la tarea 1.1, tal como el agente la escribió, solo verifica (desviación 1) | 11 min |
| 7 | `worktree/assign` 1.2 → los mismos, y dos `worktree/dispatch` en paralelo | ambos | **dos commits en dos worktrees desde la misma base**: `b6c06cc` (Claude Code) y `098a3e5` (OpenCode), cada uno con `Meltemi-Task: <change>/1.2` y la identidad git del usuario, sin coautoría | 103 s |
| 8 | `sdd/verify`: 8 escenarios sin test que los enlace (el fixture no tiene Rust); `sdd/verify-mark` × 8 con lo comprobado a mano; `sdd/verify` | — | verificación completa | — |
| 9 | `sdd/archive` | — | la capability `greet` fundida en `.meltemi/specs/`; la change archivada en `changes/archive/2026-09-26-…` | — |

Lo comprobado a mano para las marcas, en los dos worktrees de la 1.2 (idénticos)
con Python 3.12.3: `farewell('world') == 'goodbye world'`,
`farewell('Ada Lovelace') == 'goodbye Ada Lovelace'`, `farewell('') ==
'goodbye '`, `greet('world') == 'hello world'`, `greet('') == 'hello '`, las
dos firmas `(name)`, la importación conjunta desde `src.greet`, y el diff del
commit, que deja intactas las dos líneas de `greet` y añade `farewell` después.

## Veredicto por criterio

| # | Criterio | Veredicto | Evidencia |
| --- | --- | --- | --- |
| C1 | `propose` | **pasa** | propuesta andamiada bajo `.meltemi/` y escrita por el agente |
| C2 | revisión con reelaboración | **pasa** | el comentario reabrió el gate de specs y el agente reelaboró; después se aprobó (desviación 4) |
| C3 | implementación en paralelo | **pasa** (en la tarea 1.2) | dos agentes de proveedores distintos, worktrees separados desde una base común, commits con trazabilidad por tarea |
| C4 | `verify` | **pasa**, por marcas manuales | 8 de 8 escenarios marcados con lo medido |
| C5 | `archive` | **pasa** | delta fundido de forma atómica; change conservada con fecha |
| C6 | presupuestos | **pasa lo medido** | binario de la TUI (release, este commit): 4,28 MB frente a 25 MB; arranque de `meltemi --version`: mediana 24,8 ms (177 ms en frío) frente a 1 s. El instalador de la GUI no se construyó aquí: lo gatea `release.yml` |

**Corrida automatizada**: la suite e2e del hito
(`e2e_hito::the_milestone_cycle_reaches_implemented_verified_archived` y
`e2e_hito::two_distinct_profiles_work_in_parallel_worktrees_with_traceability`)
pasa en el gate del workspace de esta change, con los dos perfiles del agente
simulado y sin red.

**Veredicto de esta corrida**: los seis criterios pasan. Las desviaciones de
abajo no invalidan ninguno, pero dos de ellas (2 y 5) son defectos del producto
y tienen su change.

## Desviaciones

1. **La tarea 1.1 no escribe.** El agente la redactó como una comprobación
   («abre `src/greet.py` y confirma…»); los dos competidores la cumplieron sin
   cambiar nada, que es lo correcto. C3 se demostró en la 1.2, la que añade la
   función.
2. **Los permisos de Claude Code los decidió el cliente, no una regla.** Su
   adaptador envía las peticiones sin `rawInput` ni `locations`, así que la
   regla del propio ciclo (escrituras bajo `.meltemi/`) no puede casar y cada
   petición escala al cliente (hallazgo 2 de la [corrida
   integrada](2026-09-26-corrida-integrada.md)). El guion las decidió una por
   una con una política escrita —lectura o edición dentro del fixture, sí;
   todo lo demás, no—: **86 aprobadas y 32 denegadas** (16 búsquedas sin ruta,
   8 de clase «other», 7 comandos y 1 lectura fuera del fixture: un archivo del
   perfil de usuario del agente). Lo aborda `adaptadores-con-rawinput`.
3. **OpenCode pidió un permiso en toda la corrida**: edita sin preguntar
   (hallazgo 1 de la corrida integrada). Su commit de la 1.2 no pasó por el
   proxy.
4. **La aprobación de la revisión fue por guion.** Tras la reelaboración, los
   cuatro requisitos se aprobaron sin que una persona los leyera. El agente
   interpretó el comentario («`farewell("")` devuelve `"goodbye"`») como
   `"goodbye "` —con el espacio, igual que `greet("")`— y lo justificó en la
   spec; una persona habría decidido si esa lectura le vale.
5. **Los worktrees de tarea no se excluyen del árbol principal.**
   `worktree/assign` los crea dentro de `.meltemi/worktrees/` pero solo los
   talleres (`change/workspace`) se añaden a `.git/info/exclude`: un
   `git add -A` en el repositorio los recoge como repositorios embebidos.
   Change aparte.

## Costo

Siete turnos reales de Claude Code (cinco de autoría y reelaboración, dos despachos) y
dos de OpenCode, en veinte minutos de pared.
