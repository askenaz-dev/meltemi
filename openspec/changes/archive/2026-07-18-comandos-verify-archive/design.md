## Context

El motor ya funde deltas con paridad demostrada (`apply_delta` reproduce los
archivados reales — test de paridad de `motor-ears-deltas`); la review (#15)
persiste estados por requisito. Faltan los verbos: `/verify` (¿la implementación
cumple?) y `/archive` (fundir a la verdad viva). Son la puerta del principio
constitucional: los escenarios son la definición de terminado.

## Goals / Non-Goals

**Goals:** checklist de verificación por requisito con vínculo a tests;
archivado con validación, fusión atómica, histórico y regeneración de
proyección; gates; verbos operativos.
**Non-Goals:** verificación automática y continua spec↔código (fase 3);
property-based testing desde EARS (fase 3).

## Decisions

### D1 — Verificación por requisito, dos vías
Por cada requisito de la change: (a) **vinculado a tests** — la convención viva
"el nombre del escenario es la fuente del nombre del test" se materializa: el
daemon busca los tests correspondientes y registra su último resultado (corre la
suite del proyecto por comando configurado); o (b) **verificación manual** — el
humano marca el escenario como verificado con nota. El estado persiste en la
change (como la checklist de #15).

### D2 — Archivar = validar + fundir + preservar + proyectar
`/archive`: (1) el motor valida la change completa (estructura+EARS+deltas
aplican en seco sobre la verdad viva actual); (2) fusión con `apply_delta` de
todas las capacidades, **atómica**: o se escriben todas o ninguna (staging +
rename); (3) movimiento a `changes/archive/AAAA-MM-DD-<name>/`; (4) regeneración
de la proyección de contexto (#11). Conflictos de fusión → diagnósticos al
usuario, nada a medias.

### D3 — Gate de archivado
Archivar exige verificación completa (todos los requisitos verificados por test
o manual) **o** excepciones explícitas por requisito con justificación,
registradas en el archivo. Sin silencio: el informe de archivado lista
verificados/exceptuados.

### D4 — Superficies
Verbos `verify` y `archive` des-reservados (delta acumulativo); TUI: flujo en la
vista Proyecto reutilizando el patrón checklist de #15; scriptable por pasos con
`--json`.

## Risks / Trade-offs

- **Suites lentas** → `verify` corre la suite una vez y mapea resultados a
  escenarios; re-verificación selectiva por requisito como delta futuro.
- **Fusión concurrente con edición humana** → el archivado detecta árbol de
  specs sucio (git) y pide confirmación.

## Migration Plan

Aditivo. Prepara #24: cuando estos verbos operan sobre `.meltemi/`, la
herramienta prestada queda redundante.

## Open Questions

- Comando de test configurable por proyecto (`[verify] command`): forma exacta.
