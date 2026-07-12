# Propuesta: Enmiendas fundacionales v1

## Why

La auditoría de planificación (5 lentes, 2026-07-11) y el cierre de `fase-0-fundacion` dejaron al documento fundacional ([meltemi.md](../../../meltemi.md)) con cuatro desajustes menores frente a la realidad ya construida y ratificada. Ninguno cambia la estrategia; todos son reconciliaciones que conviene cerrar antes de arrancar la ingeniería de Fase 1, y esta es la primera enmienda al documento tramitada por su propio método (§9.3). Al ser cambios pequeños y acotados, se usa la **vía rápida** (`fast-forward`): todos los artefactos de una vez.

## What Changes

- **Bootstrap del método (reconcilia §9.3)**: el documento exige que toda modificación de sí mismo entre por `.meltemi/changes/`, pero el desarrollo usa `openspec/` hasta que exista el motor de specs (Fase 1). Se ratifica por escrito esa **excepción interina en dos etapas** y su criterio de migración, para que el propio documento sea modificable por el método real vigente.
- **Marca V2 (§0)**: la sección "La marca" describe el logo V1 monoline ya superado; se actualiza a la identidad V2 vigente (velas asimétricas sobre casco mínimo), coherente con `brand/README.md`.
- **Métricas y telemetría (§6.12, §12)**: se asigna fase explícita a las métricas SDD locales y se aclara que la telemetría agregada es post-v1, operada por la entidad custodio, con política de privacidad publicada — resolviendo la tensión con el no-objetivo "sin backend".
- **Plataforma primaria de desarrollo**: se hace explícito en el documento fundacional que **Windows es plataforma primaria de desarrollo** (ya vive en la constitución §7 y en `.meltemi/rumbo/tech.md`; `fase-0-fundacion` design D9 la citaba sin fuente en el fundacional).
- **Versión**: `meltemi.md` pasa de v1.0 a **v1.1**, con nota de enmienda; la ratificación de la nueva versión corresponde al mantenedor fundador (no se auto-ratifica).

## Capabilities

### New Capabilities

- `method-bootstrap`: gobernanza del método durante el bootstrap — la excepción interina (enmiendas vía `openspec/` hasta la migración a `.meltemi/`) y la regla de ratificación de enmiendas fundacionales. No es una capacidad de código; es la regla de gobernanza que E4 ratifica, elevada a spec de la verdad viva.

### Modified Capabilities

<!-- Ninguna. No cambian requisitos de `daemon-lifecycle`, `acp-session` ni `propose-flow`. -->

## Impact

- **Documentos**: `meltemi.md` (§0, §6.12, §9.3, §12, cabecera de versión). Ningún cambio en `.meltemi/constitution.md` ni `rumbo/` (ya son coherentes; el documento fundacional se alinea a ellos).
- **Código**: ninguno. No toca `core/`, `proto/` ni los tests.
- **Gobernanza**: primer ejercicio real del proceso de enmienda de §9.3. Requiere **aprobación del mantenedor fundador** antes de aplicarse (modifica un documento ratificado).
- **Fuera de alcance**: el formato canónico de `.meltemi/` y el motor de specs (changes `formato-artefactos-meltemi` y `motor-specs-artefactos` de Fase 1); la migración efectiva `openspec/ → .meltemi/` (change `migracion-openspec-a-meltemi`).
