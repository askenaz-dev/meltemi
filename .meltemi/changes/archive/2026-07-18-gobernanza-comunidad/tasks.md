## 1. Documentos

- [x] 1.1 GOVERNANCE.md (modelo vigente + ratificación + camino a mantenedores) _(Req: Documentos de gobernanza)_ — inglés con resumen en español (D1).
- [x] 1.2 CONTRIBUTING.md (spec-driven + vía corta + checklist de calidad + sin co-autoría; política de idiomas D1) _(Req: Contribución spec-driven)_
- [x] 1.3 CODE_OF_CONDUCT.md (Contributor Covenant 2.1 adaptado, contacto vía GitHub) y SECURITY.md (divulgación responsable honesta, reporte privado de GitHub, alcance del modelo de amenaza §8) _(Req: Política de seguridad)_
- [x] 1.4 Texto del CLA acotado (§9.3: Apache-2.0 con patentes, sin cesión de copyright) con la decisión de firma como pendiente registrado _(Req: Texto del CLA)_

## 2. Plantillas y lint

- [x] 2.1 Plantillas `.github/` (issue de propuesta de change; PR con checklist de calidad y prohibición de co-autoría)
- [x] 2.2 Lint de CI: presencia + secciones mínimas de los documentos _(Req: Documentos — lint)_ — `core/meltemid/tests/governance.rs` con control negativo (falla si falta un documento o sección).

## 3. Calidad

- [x] 3.1 Revisión final del mantenedor (los documentos hablan por el proyecto) y verificación del lint en verde — el lint pasa en verde; la revisión de contenido queda para el mantenedor fundador (documentos redactados conforme a los designs D1–D5).
