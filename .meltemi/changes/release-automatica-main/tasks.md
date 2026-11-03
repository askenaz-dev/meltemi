# Tasks

- [x] 1.1 Propuesta, diseño y escenarios según autorización del 2026-11-01.
- [x] 2.1 Validación de CI y reserva inmutable; notas y manifiesto.
- [x] 2.2 Workflows sobre SHA exacto, estampado y recuperación.
- [x] 2.3 Publicación idempotente y protección de latest.
- [x] 3.1 Pruebas de comportamiento con fixtures y controles de workflows.
- [x] 3.2 Documentación y decisiones anteriores reconciliadas.
- [x] 4.1 Verificación local, incluido checkout real estampado a 0.2.123 y
  repetición idempotente; evidencia en docs/qa/2026-11-01-releases-automaticas.md.
- [ ] 4.2 CI multiplataforma de la PR y primera publicación automática tras merge.
- [x] 4.3 Incorporar el parche compatible de async-executor retirado del registry antes de activar las publicaciones y verificar cargo-deny.
- [x] 4.4 Corregir la búsqueda paginada y verificación por ID de borradores; cubrir el comportamiento real de la API y conservar diagnósticos HTTP sin secretos (17 pruebas Node verdes).
- [ ] 4.5 Verificar y finalizar los borradores existentes sin reconstrucción, repetir sus jobs fallidos y comprobar una publicación automática posterior al arreglo.
