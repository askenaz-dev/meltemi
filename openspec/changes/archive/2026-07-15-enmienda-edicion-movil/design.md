# Design: Enmienda — edición utilitaria in situ y compañero móvil

## Context

`meltemi.md` v1.2 (ratificación pendiente) fija en su no-objetivo #1 que la superficie de código es de *revisión, no de edición*, y el roadmap de fase 3 menciona el compañero móvil solo como "supervisar la flota". La exploración del 2026-07-14 concluyó que (a) la cláusula absoluta rompe la experiencia del bucle agéntico — un retoque trivial expulsa al usuario del plano de control — y (b) el móvil sin límites escritos puede derivar en un editor de bolsillo o en presión por abrir transporte de red. Esta change enmienda los documentos (solo documentos) y eleva los límites a specs vivas, siguiendo el precedente de `enmiendas-fundacionales-v1` (ediciones textuales E*, ratificación del mantenedor, spec de gobernanza tipo `method-bootstrap`).

## Goals / Non-Goals

**Goals:**
- Reemplazar la cláusula "revisión, no edición" por edición utilitaria acotada por una cerca normativa, sin renunciar a "no es un editor de propósito general".
- Fijar el principio rector "salir infrecuente, no imposible" como criterio de evaluación de futuras features de edición.
- Precisar el compañero móvil de fase 3: monitorear + aprobar + dirigir, sin edición, solo túnel SSH, regla de subconjunto.
- Dejar los requisitos en specs (`edit-surface`, `mobile-companion`) que gobiernen las changes de implementación de fases 2 y 3.

**Non-Goals:**
- Implementar edición, deep-link o app móvil (código: cero en esta change).
- Resolver la política de concurrencia humano↔agente (se delega, con opciones esbozadas, al design de la change de GUI de fase 2).
- Elegir el componente de edición embebido de la GUI o el mecanismo de notificaciones del móvil.
- Modificar la constitución o el rumbo tech (ya cubren lo necesario).

## Decisions

### D1 — Enmienda quirúrgica, no pivote
Se conserva íntegro "no es un editor de propósito general ni un IDE clásico" y solo se reescribe la cláusula de la superficie de código. *Alternativa rechazada*: redefinir Meltemi como IDE completo — contradice la Decisión 1 fundacional (aplicación independiente, no fork de editor), multiplica el costo por años y compite sin diferenciación donde el producto no gana.

### D2 — La cerca normativa vive en la spec; el fundacional lleva el principio
`meltemi.md` queda con la formulación breve (edición utilitaria + principio "infrecuente, no imposible" + puntero a la spec); la lista DENTRO/FUERA completa y verificable vive en `specs/edit-surface/spec.md`. *Racional*: el fundacional es visión estable; la spec es verdad viva consultable por toda change futura. *Alternativa rechazada*: la cerca completa dentro de `meltemi.md` — infla el documento y duplica la fuente de verdad.

### D3 — Edición como capacidad del daemon, experiencia por superficie
Toda edición in situ se materializa como capacidad del daemon (escritura en worktree + evento `human_edit` en el JSONL de sesión); cada superficie la consume con su experiencia: la GUI con un panel de edición embebido con cliente LSP, la TUI suspendiendo a `$EDITOR` (patrón clásico de las herramientas de terminal) o con mini-edición de hunks. *Racional*: preserva la paridad de núcleo (constitución §4, que ya admite diferencias de *experiencia*) y la trazabilidad (constitución §8). *Alternativa rechazada*: edición puramente client-side sin pasar por el daemon — invisible para el log de sesión, rompe §8 y bifurca el contrato.

### D4 — Móvil como compañero reducido; la constitución no se toca
El móvil se define por la regla de subconjunto: todo lo que hace también existe en TUI y GUI; ninguna capacidad del daemon es exclusiva de una superficie, que es la letra y el espíritu de §4. El acceso remoto vía túnel SSH ya está bendecido por §3. *Alternativa rechazada*: enmendar §4 para enumerar tres superficies con niveles — innecesario (la letra actual ya lo permite) y rigidiza la constitución con detalle de roadmap.

### D5 — Concurrencia humano↔agente: requisito mínimo ahora, política en fase 2
La spec `edit-surface` fija solo el mínimo verificable (advertir antes de guardar sobre un worktree con sesión de agente activa); el mecanismo completo — aviso, soft-lock de archivos abiertos por el agente, notificación al agente vía ACP para relectura — se decide en el design de la change de GUI de fase 2, donde habrá arquitectura concreta que evaluar. *Racional*: decidir política sin superficie construida sería especular.

### D6 — Componente de edición: requisito sí, elección no
Se fija el listón (cliente LSP con autocompletado, diagnósticos y navegación; sin ecosistema de extensiones) y se difiere la elección del componente embebible al design de la change de GUI, donde se justificará como dependencia (constitución §10).

## Ediciones textuales

### E1 — `meltemi.md`, cabecera de versión
Reemplazar la línea de versión por:
> Versión 1.3 — enmendada el 14 de julio de 2026 (`enmienda-edicion-movil`); ratificación de la v1.3 pendiente del mantenedor fundador. Enmiendas previas: v1.2 (`formato-artefactos-meltemi`, ratificación pendiente), v1.1 (`enmiendas-fundacionales-v1`). Base v1.0 ratificada el 11 de julio de 2026 por Guillmar Ortiz (`fase-0-fundacion` 1.2).

### E2 — `meltemi.md` §3, no-objetivo #1
Reemplazar el punto 1 completo por:
> 1. **No es un editor de código de propósito general ni un IDE clásico**: la autoría sostenida de código ocurre en el editor que cada usuario ya usa, siempre a un salto de distancia ("Abrir con…" con archivo:línea exacto). La superficie de código de Meltemi es de *revisión y edición utilitaria* al servicio del bucle agéntico (revisar → retocar → dirigir): Meltemi optimiza para que salir sea **infrecuente, no imposible**. La cerca normativa de lo que la edición incluye y excluye vive en la spec `edit-surface`.

### E3 — `meltemi.md` §6, nueva funcionalidad 13
Añadir tras el punto 12:
> 13. **Edición utilitaria in situ** *(GUI en fase 2; TUI vía `$EDITOR` o mini-edición de hunks)*: retoques y ajustes en contexto con inteligencia LSP (autocompletado, diagnósticos, navegación), edición de hunks en el diff y "Abrir con…" hacia el editor del usuario con archivo:línea. Toda edición in situ pasa por el daemon y queda registrada como evento `human_edit` en el log de sesión.

### E4 — `meltemi.md` §10, fase 2, bullet de GUI
Reemplazar por:
> - GUI Tauri con paridad de núcleo: editor de specs enriquecido, revisión de diffs línea a línea y edición utilitaria in situ con inteligencia LSP, bandeja de permisos, panel de flota. El design de esta fase resuelve la política de concurrencia humano↔agente sobre un mismo worktree.

### E5 — `meltemi.md` §10, fase 3, bullet del compañero móvil
Reemplazar por:
> - Compañero móvil (Tauri móvil): superficie compañera reducida para **monitorear, aprobar y dirigir** la flota — sin edición; acceso únicamente vía túnel SSH; regla de subconjunto respecto de TUI/GUI (spec `mobile-companion`).

### E6 — `.meltemi/rumbo/product.md`, "Qué NO es"
Reemplazar la línea por:
> **Qué NO es**: ni un editor de propósito general (la superficie de código admite edición utilitaria al servicio del bucle agéntico; la autoría sostenida vive en el editor del usuario), ni otro agente de codificación (el motor propio de fase 2 es opcional), ni un servicio en la nube, ni CI/CD, ni un marketplace.

### E7 — `AGENTS.md`
Verificar la proyección manual: hoy no proyecta la sección "Qué NO es" del rumbo de producto, por lo que probablemente no requiere edición; si tras E6 se considera que el matiz es relevante para agentes, añadir una línea en "Qué es este proyecto". Decisión al aplicar.

### E8 — `docs/plan-de-cambios.md`
Anotar en las entradas correspondientes: la change de GUI de fase 2 incorpora edición utilitaria in situ + política de concurrencia humano↔agente; la change de fase 3 del compañero móvil queda gobernada por la spec `mobile-companion`.

## Risks / Trade-offs

- **[Scope creep de la cerca]** Cada futura petición podrá disfrazarse de "una cosita más para no salir" → el principio "infrecuente, no imposible" y el escenario "propuesta fuera de la cerca" de `edit-surface` obligan a tramitar cualquier ampliación como enmienda fundacional, no como feature.
- **[Specs de gobernanza sin código]** `edit-surface` y `mobile-companion` describen requisitos aún no implementables → como `method-bootstrap`, sus escenarios se cubren por verificación documentada (constitución §1) hasta que las changes de fase 2/3 los conviertan en tests.
- **[Encadenar v1.3 sobre v1.2 no ratificada]** Dos versiones pendientes a la vez → la cabecera E1 deja el estado explícito; el mantenedor puede ratificar en orden o en bloque.
- **[Edición concurrente humano↔agente]** El riesgo real de la feature queda abierto → requisito mínimo de advertencia en la spec + decisión obligatoria en el design de la change de GUI (E4 lo deja escrito en el roadmap).

## Open Questions

- Política completa de concurrencia humano↔agente (aviso / soft-lock / notificación ACP) — se resuelve en el design de la change de GUI de fase 2.
- Componente de edición embebido de la GUI — design de la change de GUI (justificación de dependencia per constitución §10).
- Mecanismo de notificaciones del compañero móvil (sondeo sobre el túnel vs. mecanismos push y sus implicaciones de privacidad) — change de fase 3; cualquier opción deberá respetar "sin servicio en la nube" y la constitución §9.
