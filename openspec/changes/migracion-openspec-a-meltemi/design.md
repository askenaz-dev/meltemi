## Context

El cierre del bootstrap en dos etapas (design D9 de `fase-0-fundacion`): el motor
valida y funde con paridad demostrada, `/verify` y `/archive` (#19) operan sobre
`.meltemi/`. La herramienta prestada queda redundante; Meltemi hospeda sus
propias specs.

## Goals / Non-Goals

**Goals:** migrar verdad viva e histórico a `.meltemi/` con verificación del
motor; el ciclo del repo pasa a los comandos de Meltemi; `method-bootstrap`
enmendado declarando cerrada la etapa; proyección regenerada.
**Non-Goals:** cambios de formato (ya canónico); tooling de migración para
terceros (si la comunidad lo pide, change futura).

## Decisions

### D1 — Migración verificada spec a spec
Por cada capacidad: parsear la spec en `openspec/specs/`, copiar a
`.meltemi/specs/`, parsear el destino y comparar modelos (requisitos y
escenarios idénticos). Cualquier diferencia aborta. El histórico
(`changes/archive/*`) se mueve preservando fechas y contenido byte a byte.

### D2 — Corte limpio del método
Tras la migración: los flujos `/opsx:*` se retiran de la configuración del repo,
`AGENTS.md` (proyección #11) refleja el método nuevo, y el plan maestro anota el
cierre. La paridad final se demuestra invirtiendo el test: la verdad viva migrada
revalida contra los archivados históricos migrados.

### D3 — En rama, reversible hasta el merge
Toda la migración ocurre en una rama con la verificación en cada paso; el merge
es la ratificación operativa. Si algo exige código nuevo, es un hueco de #19 y
vuelve allí como delta — esta change no escribe features.

## Risks / Trade-offs

- **Referencias a rutas `openspec/` en docs/tests** → barrido con verificación
  (grep gate en CI de la rama) antes del merge.
- **Nostalgia de herramienta** → el histórico queda íntegro y consultable.

## Migration Plan

Es el plan mismo (D1–D3). Reversión: descartar la rama.

## Open Questions

- Momento exacto del retiro de la carpeta `openspec/` (¿mismo merge o release
  siguiente?): propuesto mismo merge; confirmar con el mantenedor.
