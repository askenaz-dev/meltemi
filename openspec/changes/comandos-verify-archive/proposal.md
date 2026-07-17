## Why

El ciclo se cierra con dos verbos aún reservados: `/verify` (¿la implementación
cumple la spec?) y `/archive` (fundir los deltas aprobados en la verdad viva).
El motor ya sabe hacer la fusión — `apply_delta` reproduce el archivado de la
herramienta prestada (test de paridad) — pero no está expuesto como comando.
`/verify` es además el guardián del principio constitucional: los escenarios son
la definición de "terminado".

## What Changes

- **`/verify`**: checklist por requisito EARS de la change — cada escenario se
  marca verificado manualmente o **vinculado a tests** (convención: el nombre del
  escenario es la fuente del nombre del test, regla ya vigente en el rumbo); el
  resultado por requisito queda persistido en la change.
- **`/archive`**: valida la change (motor: estructura + EARS + deltas
  aplicables), funde los deltas en `.meltemi/specs/` con `apply_delta`, preserva
  el histórico en `.meltemi/changes/archive/AAAA-MM-DD-<change>/` y regenera la
  proyección de contexto (#11).
- **Gates**: archivar exige verificación completa o confirmación explícita de
  excepciones (con registro).
- **Superficies**: verbos des-reservados en gramática, paleta y vista Proyecto.

## Capabilities

### New Capabilities
- `verify-archive`: la checklist de verificación y el archivado con fusión.

### Modified Capabilities
- `spec-merge`: la fusión pasa de librería a operación de producto (atomicidad,
  conflictos reportados como diagnósticos al usuario).
- `cli-contract`: `verify`/`archive` dejan de estar reservados.
- `sdd-authoring`: el ciclo declara su fase final.

## Impact

- `core/meltemid` + `core/meltemi-spec` (ya listo en lo esencial), `tui/`
  (checklist interactiva reutilizando patrones de #15).
- Prepara #24: cuando `/archive` funciona, la herramienta prestada es redundante.

## Fuera de alcance

- Verificación automática y continua spec↔código (fase 3, §5.2).
- Property-based testing derivado de EARS (fase 3).
