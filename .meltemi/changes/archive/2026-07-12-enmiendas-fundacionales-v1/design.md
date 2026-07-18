# Diseño: Enmiendas fundacionales v1

## Context

`meltemi.md` fue ratificado como v1.0 el 2026-07-11. La implementación de `fase-0-fundacion` y la auditoría de planificación dejaron cuatro puntos donde el texto fundacional quedó desalineado con decisiones ya tomadas y ratificadas en otros artefactos (`.meltemi/constitution.md`, `.meltemi/rumbo/`, `brand/README.md`). Esta change los reconcilia mediante ediciones acotadas y textualmente especificadas aquí, para que la aplicación no requiera interpretación.

## Goals / Non-Goals

**Goals:**
- Que `meltemi.md` refleje la realidad ratificada: bootstrap en dos etapas, marca V2, fase de métricas/telemetría, plataforma primaria de desarrollo.
- Que el propio documento fundacional quede modificable por el método vigente (openspec/) sin contradecirse.
- Dejar cada edición especificada al nivel de párrafo, para un `apply` mecánico y un diff revisable.

**Non-Goals:**
- No se cambia ninguna decisión de estrategia, arquitectura ni alcance.
- No se toca `.meltemi/constitution.md` ni `rumbo/` (ya coherentes).
- No se define el formato de `.meltemi/` ni se implementa el motor de specs (Fase 1).

## Decisions

### E1 — Cabecera de versión → v1.1
En la cita de cabecera, sustituir la línea de versión por:

> Versión 1.1 — enmendada el 2026-07-12 (`enmiendas-fundacionales-v1`); ratificación de la v1.1 pendiente del mantenedor fundador. Base v1.0 ratificada el 2026-07-11 por Guillmar Ortiz.

No se auto-ratifica: el stamping definitivo de "v1.1 ratificada" lo hace el mantenedor, como con v1.0.

### E2 — §0 "La marca" → identidad V2
Reemplazar el bloque actual de "La marca" (que describe la *m* monoline de una sola línea) por la descripción V2 vigente, alineada con `brand/README.md`:

> Una *m* minúscula trazada con confianza que se lee, a la vez, como un pequeño velero: dos arcos que son **velas asimétricas** sobre una curva mínima que es el **casco**, y un extremo derecho que es **proa y ráfaga** a la vez. Primera lectura: "m en movimiento"; segunda: "velero impulsado por el viento". El detalle vive en `brand/README.md`.

Mantener el resto de §0 (paleta, nomenclatura) sin cambios.

### E3 — §6.12 y §12: fase y telemetría
En §6.12 (métricas SDD locales), añadir la marca de fase *(fase 2)*, coherente con hooks/cliente MCP. En §12, tras la frase de telemetría opt-in, añadir:

> La telemetría agregada es **post-v1**: la operaría la entidad custodio sin ánimo de lucro (§9.3), con datos y política de privacidad especificados y publicados antes de existir (constitución §9). Hasta entonces, todas las métricas de producto y flota se calculan y quedan en local.

### E4 — Bootstrap en dos etapas: enmienda a §9.3
En §9.3, en la viñeta "Este documento se gobierna con el mismo método que predica", añadir una frase de excepción interina:

> **Excepción interina (bootstrap en dos etapas)**: hasta que el motor de specs de Fase 1 permita a Meltemi hospedar sus propios cambios, las enmiendas a este documento se tramitan con OpenSpec en `openspec/changes/`. La migración a `.meltemi/changes/` es la change `migracion-openspec-a-meltemi`. Esta excepción se ratifica en `enmiendas-fundacionales-v1`.

### E5 — Plataforma primaria de desarrollo
En §7 (o al final de §7.3, donde se decide Rust), añadir una frase:

> **Windows es plataforma primaria de desarrollo**, no un puerto posterior (constitución §7): toda la abstracción de plataforma se diseña y prueba primero allí, donde el aislamiento de procesos y sockets es más restrictivo.

## Risks / Trade-offs

- **[Editar un documento ratificado]** → Mitigación: cada edición está especificada aquí textualmente; la aplicación requiere aprobación del mantenedor (modifica un artefacto ratificado); el diff es pequeño y revisable.
- **[Deriva marca ↔ documento]** → E2 apunta a `brand/README.md` como fuente de detalle, evitando duplicar (y desincronizar) la especificación visual.

## Open Questions

- ¿La v1.1 se ratifica en el mismo acto que se aplica esta change, o en un commit posterior del mantenedor? Decisión del mantenedor al aplicar (E1 deja el stamping pendiente por defecto).
