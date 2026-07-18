## Why

El corazón metodológico del producto (§5.2) sigue reservado: `propose` crea hoy un
andamio mínimo y los demás verbos anuncian "próximamente". El motor de specs
(parseo, EARS, deltas) y el shell ya existen; falta el **ciclo de autoría**: que
un agente redacte requirements EARS, design, deltas y tasks **con gates humanos
en cada paso**, y que la disciplina sea proporcional al cambio (modo dual). Es la
change que convierte a Meltemi de lanzador de agentes en plano de control
spec-driven.

## What Changes

- **`/constitution`**: establece o edita los principios del proyecto (crea
  `.meltemi/constitution.md` con plantilla guiada; edición asistida con gate).
- **`/explore`**: socio de pensamiento sin compromiso — lee el repo, sopesa
  opciones, propone rumbo; nunca escribe artefactos.
- **`/propose` completo**: idea → proposal → requirements EARS → design → deltas
  de specs → tasks, cada artefacto redactado por el agente, **validado por el
  motor** (`meltemi-spec`: estructura + EARS) y **aprobado por el humano** antes
  del siguiente (gate por artefacto).
- **`/plan`**: refina design y secuencia tasks por dependencias.
- **Modo dual** con criterio escrito de proporcionalidad: `spec-full` (gates por
  artefacto) y `fast-forward` (todos los artefactos de una vez, un gate final).
- **Superficies**: verbos des-reservados en gramática CLI, paleta y vista
  Proyecto (los gates son interactivos en la TUI; en CLI scriptable, por pasos).

## Capabilities

### New Capabilities
- `sdd-authoring`: los verbos del ciclo de autoría, sus gates y el modo dual.

### Modified Capabilities
- `propose-flow`: de andamio mínimo a ciclo completo con gates (el andamio
  sobrevive como primer paso).
- `cli-contract`: `explore`/`plan` (y `propose` pleno) dejan de estar reservados.
- `tui-shell`: la vista Proyecto ancla las acciones del ciclo (interior que su
  spec dejó reservado).

## Impact

- `core/meltemid` (orquestación del ciclo + validación con `meltemi-spec`),
  `proto/` (métodos/eventos de gate), `tui/` (flujos interactivos).
- Los artefactos generados van a `.meltemi/changes/` del proyecto objetivo
  (formato `artifact-format`).

## Fuera de alcance

- `/review` rico (diffs, contradicciones, checklist): #15.
- `/implement`, `/verify`, `/archive`: #20 y #19.
- Detección semántica de contradicciones/huecos (diferida desde #4; aterriza
  con #15 o change propia del motor).
