# Tareas: Formato canónico de los artefactos `.meltemi/`

> Esta change no tiene código: fija el contrato de formato (spec `artifact-format`, ya redactado) y alinea `meltemi.md`. Las ediciones a `meltemi.md` modifican un documento ratificado y **requieren aprobación del mantenedor** (regla `method-bootstrap`). Detalle textual en `design.md` (F5).

## 1. Enmienda a `meltemi.md` (v1.1 → v1.2)

- [ ] 1.1 §2.1: mostrar los patrones EARS de ejemplo en el canon inglés (`WHEN`/`WHILE`/`IF … THEN`/`WHERE` + `SHALL`), aclarando que la prosa va en español neutro (F1, F5)
- [ ] 1.2 §5.1: cambiar la línea de deltas del árbol a `## ADDED / ## MODIFIED / ## REMOVED Requirements`; §6.1 (editor de specs) descrito con las mismas cabeceras inglesas (F5)
- [ ] 1.3 §2.3 y prosa relacionada: sustituir `AÑADIDOS/MODIFICADOS/ELIMINADOS` por la referencia al canon inglés donde aparezcan como palabras clave del formato (no en prosa libre)
- [ ] 1.4 Cabecera → **v1.2**, con nota de enmienda (`formato-artefactos-meltemi`) y ratificación de la v1.2 pendiente del mantenedor (no auto-ratificar)

## 2. Conformidad y cierre

- [ ] 2.1 Verificar que las specs vivas de `fase-0` (`acp-session`, `daemon-lifecycle`, `propose-flow`, `method-bootstrap`) cumplen el canon del spec `artifact-format`; normalizar cualquier residuo (p. ej. cabeceras o EARS fuera de canon)
- [ ] 2.2 Verificar que `.meltemi/rumbo/*.md` cumplen el front-matter canónico (`inclusion: siempre | por-patrón | manual`); ajustar `AGENTS.md` si cambia alguna palabra clave proyectada
- [ ] 2.3 Releer §2.1, §2.3, §5.1 y §6.1 tras las ediciones para confirmar coherencia interna y que no quedan palabras clave de formato en español
