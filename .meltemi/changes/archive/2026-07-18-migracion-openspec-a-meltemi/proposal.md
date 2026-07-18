## Why

El dogfooding definitivo y el cierre del bootstrap en dos etapas (design D9 de
`fase-0-fundacion`): Meltemi se desarrolla hoy con una herramienta prestada
(`openspec/`) porque su propio motor no existía. Ya existe — parsea, valida
(estructura + EARS) y funde deltas con paridad demostrada por test contra los
archivados reales — y con #19 el `/archive` es un comando del producto. Llegó la
hora de que **Meltemi hospede sus propias specs** y la herramienta prestada se
retire.

## What Changes

- **Migración de la verdad viva**: `openspec/specs/*` → `.meltemi/specs/*`
  (formato `artifact-format`, ya compatible), con verificación del motor:
  cada spec migrada parsea idéntica (requisitos y escenarios) antes y después.
- **Migración del histórico**: `openspec/changes/archive/*` →
  `.meltemi/changes/archive/*` preservando fechas y contenido.
- **El método pasa al producto**: el ciclo de este repositorio usa los comandos
  de Meltemi (`propose`/`verify`/`archive` sobre `.meltemi/`); los flujos
  `/opsx:*` quedan retirados.
- **Actualización del bootstrap**: `method-bootstrap` se enmienda para declarar
  cerrada la etapa 1; `AGENTS.md` (ya autogenerado por #11) refleja el método
  nuevo; el design D9 se da por cumplido.
- **Verificación de paridad final**: el test de paridad se invierte — la verdad
  viva migrada revalida contra los archivados históricos.

## Capabilities

### New Capabilities
- _Ninguna nueva de producto_ (usa `spec-engine`, `spec-merge`, `verify-archive`).

### Modified Capabilities
- `method-bootstrap`: la etapa OpenSpec se declara cerrada; el método del
  proyecto es Meltemi mismo.

## Impact

- Movimiento de árboles de artefactos + enmienda de `method-bootstrap` +
  actualización de docs/proyecciones. Cero código de producto nuevo (si la
  migración exige algo, es señal de hueco en #19 y vuelve allí como delta).
- Riesgo controlado: migración en una rama con verificación del motor en cada
  paso; reversible hasta el merge.

## Fuera de alcance

- Cambios de formato de artefactos (ya canónico); tooling de migración para
  proyectos de terceros (change futura si la comunidad lo pide).
