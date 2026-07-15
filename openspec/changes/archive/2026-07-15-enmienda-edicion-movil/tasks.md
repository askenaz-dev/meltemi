# Tareas: Enmienda — edición utilitaria in situ y compañero móvil

> Todas las ediciones están especificadas textualmente en `design.md` (E1–E8). Modifican documentos ratificados: requieren aprobación del mantenedor fundador antes de aplicarse, y la v1.3 queda pendiente de ratificación (spec `method-bootstrap`).

## 1. Enmiendas a `meltemi.md`

- [x] 1.1 Cabecera de versión → v1.3 con nota de enmienda y ratificación pendiente, preservando la cadena v1.2/v1.1 (E1)
- [x] 1.2 §3 no-objetivo #1: reemplazar "revisión, no edición" por edición utilitaria + principio "infrecuente, no imposible" + puntero a la spec `edit-surface` (E2)
- [x] 1.3 §6: añadir la funcionalidad 13 "Edición utilitaria in situ" con trazabilidad `human_edit` (E3)
- [x] 1.4 §10 fase 2: bullet de GUI con edición utilitaria in situ y la decisión de concurrencia humano↔agente asignada al design de esa fase (E4)
- [x] 1.5 §10 fase 3: bullet del compañero móvil precisado — monitorear/aprobar/dirigir, sin edición, túnel SSH, regla de subconjunto (E5)

## 2. Rumbo y proyecciones

- [x] 2.1 `.meltemi/rumbo/product.md`: matizar "Qué NO es" con la edición utilitaria (E6)
- [x] 2.2 `AGENTS.md`: verificar la proyección manual tras E6 y actualizarla solo si el matiz es relevante para agentes (E7) — sin edición: el resumen operativo no proyecta la sección "Qué NO es", así que el matiz no tiene dónde sincronizarse aquí
- [x] 2.3 `docs/plan-de-cambios.md`: anotar el alcance añadido a la change de GUI de fase 2 (edición in situ + política de concurrencia) y la gobernanza de la change móvil de fase 3 por la spec `mobile-companion` (E8)

## 3. Cierre

- [x] 3.1 Releer §3, §6 y §10 de `meltemi.md` completos tras las ediciones para verificar coherencia interna (sin contradicciones entre no-objetivo #1, funcionalidad 13 y roadmap)
- [x] 3.2 Verificación documentada de los escenarios de gobernanza de `edit-surface` y `mobile-companion` (constitución §1: escenarios sin código se cubren por verificación documentada) y confirmación de que la nota de versión deja la ratificación de v1.3 explícitamente pendiente del mantenedor
