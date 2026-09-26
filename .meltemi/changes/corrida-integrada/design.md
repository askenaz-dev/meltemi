# Design — corrida-integrada

## Context

Leído el código y lo que la máquina tiene instalado antes de diseñar, estos
hechos fijan el diseño:

- **Los CLIs de esta máquina**, el 2026-09-25: Claude Code 2.1.280, OpenCode
  1.18.32 y Codex 0.156.1. No están Gemini, Copilot, Cursor, Kiro, Kilo ni
  ollama. Es una sola máquina y es Windows.
- **Lo que el repositorio sabe de ellos**: `docs/conformidad-manual.md` tiene a
  Claude Code verificado en nivel 2 contra `2.1.167` (la máquina tiene
  `2.1.280`), a Codex en nivel 2 contra `0.156.1` desde `codex-al-dia`, y a
  OpenCode sin corrida con prompt. El informe de aceptación que la spec
  `v01-acceptance` exige no existe —`docs/hito-v01.md` es una plantilla— y
  `v0.1.0` ya está publicada.
- **Aislar un daemon son cinco variables** desde `acceso-remoto-afinado`:
  `MELTEMI_ENDPOINT`, `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR`,
  `MELTEMI_DAEMON_BIN` y `MELTEMI_START_HERE=1`; sin la última, un cliente ya no
  arranca un daemon en un endpoint que no es el propio.
- **Conceder sin humano es una regla.** `.meltemi/permissions.toml` del
  proyecto acepta `[[rule]]` con `effect`, `tool`, `command-prefix` y
  `path-prefix` (prefijo de texto sobre la ruta que el agente declara), y cada
  decisión por regla queda en el log como `decidedBy: rule` con la regla. Sin
  regla y sin terminal, la CLI deniega con el rechazo del agente. Los
  worktrees gestionados viven dentro del repositorio (`.meltemi/worktrees/`),
  así que un prefijo del fixture los cubre.
- **Dos verbos del hito no tienen CLI**: comentar en la revisión
  (`sdd/review-decide`) y marcar un escenario a mano (`sdd/verify-mark`) están
  en la matriz de paridad con «—» en la columna del CLI. Son contrato, y
  `meltemi bridge` es un canal JSON-RPC completo: una corrida sin TUI los llama
  por ahí.
- **`verify` solo enlaza marcadores en archivos `.rs`**: en un fixture sin
  Rust, cada escenario del hito se verifica con una marca manual, que es lo que
  `sdd/verify-mark` existe para registrar.
- **Las trampas ya aprendidas**, en la memoria del proyecto y en el smoke del
  2026-09-24: un `meltemi` en tubería retiene el pipe del daemon que arranca
  (redirigir a archivos), el named pipe solo se alcanza desde la PowerShell de
  esta sesión, y en PowerShell `$OUT` y `$out` son la misma variable.

## Goals / Non-Goals

**Goals**

- Una corrida que se repite igual cada vez —mismo aislamiento, misma matriz,
  mismo informe— para que un CLI nuevo se mida en una hora.
- La evidencia publicada: qué agente, qué versión, qué verbo, qué pasó, y qué
  no se midió.
- El informe de aceptación v0.1 con dos agentes reales de proveedores
  distintos.

**Non-Goals**

- Correr agentes reales en CI (§5) o sin un humano que lo decida.
- Corregir lo que la corrida encuentre: cada hallazgo es su change.
- Instalar CLIs o servicios en esta máquina para ampliar la matriz.

## Decisions

### D1 — Un script del repositorio, no un test

`scripts/smoke-agents.ps1` (Windows, el que esta máquina corre) y
`scripts/smoke-agents.sh` (macOS/Linux, la misma matriz). No es un test porque
gasta turnos de suscripciones reales y el opt-in es correrlo; ningún workflow
lo referencia, y un test lo pinea. El `.sh` no se puede ejercer en esta
máquina —el named pipe no se alcanza desde git-bash— y el informe lo declara.

### D2 — Aislado por las cinco variables, y nada más

El script fija las cinco variables sobre directorios temporales propios, crea
un repositorio fixture temporal con `git init`, y al terminar pide `stop` a su
daemon y detiene solo los procesos cuyo ejecutable es el que él lanzó. El
daemon, los datos y la configuración del usuario no se leen ni se tocan.

### D3 — La matriz: siete verbos por agente

Por cada agente de la flota que el fixture ve `ready`:

| Verbo | Qué mide |
| --- | --- |
| `session` de lectura | un turno que lee y responde |
| `session` de escritura sin regla | que sin humano ni regla se deniega |
| `session` de escritura con regla | que la regla concede y queda registrada |
| `propose` | una propuesta escrita por el agente, con la regla |
| `dispatch` | un turno en un worktree gestionado, con su commit |
| `direct` | dirigir la sesión de lectura: reanudarla o encolar |
| `cancel` | una sesión viva cancelada desde otra orden |

La regla del fixture concede `read` y `edit` con `path-prefix` al fixture, en
sus dos formas de ruta (con `\` y con `/`), y nada más: la ejecución de
comandos queda sin regla, y sin humano se deniega.

### D4 — Se mide lo que el producto registra, sin estimar

De la respuesta JSON de cada orden: código de salida, estado, permisos
denegados, turno vacío. Del registro de la sesión (`meltemi --json log`):
permisos pedidos y quién los decidió, y el uso que el agente reportó por el
protocolo (contexto y costo, como cifras suyas). Del reloj: duración de pared.
Lo que el producto no registra no se inventa.

### D5 — El informe lo escribe el script, los hallazgos una persona

El script escribe `docs/qa/<fecha>-corrida-integrada.md` con la tabla agente ×
verbo y las versiones; quien corre añade los hallazgos y los declarados. Los
agentes del catálogo que no están instalados se listan como **no medidos**,
nunca omitidos.

### D6 — El hito v0.1 con dos agentes reales, sin TUI

Claude Code y OpenCode, proveedores distintos, sobre un fixture mínimo:
`propose` (A) → revisión con un comentario que obliga a reelaborar y luego se
aprueba (por `meltemi bridge`, D-Context) → `assign` de la tarea compartida a
A y B y dos `dispatch` en paralelo → `verify` con marcas manuales por
`bridge` → `archive`. El informe va a `docs/qa/<fecha>-hito-v01-aceptacion.md`
con veredicto por criterio (C1–C6) —C6 con los presupuestos medidos del binario
release— y `docs/hito-v01.md` deja de ser plantilla y lo enlaza. Que el hito
necesite el puente para dos pasos es un hallazgo, no un atajo: se anota.

### D7 — La conformidad manual gana sus piernas

`conformance_real` contra Claude Code 2.1.280 (dos turnos; la corrida vigente
es contra 2.1.167); OpenCode 1.18.32 con prompt, desde la matriz; y los nivel 1
que no están instalados, declarados como no medidos en vez de omitidos.

### D8 — La regla y dónde se ve

`docs/agentes.md` gana una sección con la última corrida por agente (versión,
fecha, resultado, informe) y la regla: la corrida se repite ante todo cambio
de versión de un CLI pineado o de un adaptador propio. En una sección aparte,
no dentro de la de cada agente, porque esas secciones las verifica el registro.

### D9 — Las specs

`integration-levels` gana el requisito de la corrida reproducible;
`v01-acceptance` gana el del informe publicado. Los dos ADDED: ninguno cambia
lo que otro requisito ya dice.

## Risks / Trade-offs

- **R1 — Costo real.** Siete verbos por tres agentes más el hito son unas
  treinta sesiones reales. Se acota con instrucciones de una línea y un fixture
  de tres archivos, y se dice en el informe.
- **R2 — Una regla por prefijo de ruta** depende de la forma en que cada
  agente escribe la ruta; una ruta relativa no casa y se deniega. El informe
  dice qué se denegó y por qué, que es exactamente lo que interesa medir.
- **R3 — La corrida es una foto.** Una versión nueva de un CLI la invalida; por
  eso D8 escribe la regla de repetirla.

## Migration

Nada que migrar: un script nuevo, documentos nuevos y una sección de la guía de
agentes. Nada corre en CI.
