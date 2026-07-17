## 1. Política y pipeline

- [ ] 1.1 Política de versionado escrita (qué rompe) + versión de workspace y tags _(Req: Versionado)_
- [ ] 1.2 Pipeline de release 3 plataformas: build+suite+clippy+fmt+cargo-deny+presupuestos (§12) con aborto en rojo _(Req: Pipeline con gates)_

## 2. Firma e instalación

- [ ] 2.1 Checksums+firma de artefactos e instrucciones de verificación; procedimiento de custodia documentado con el mantenedor _(Req: Artefactos firmados)_
- [ ] 2.2 Instalador auditable por SO (hash publicado + manual equivalente) que instala binarios y crea `mel` _(Req: Instalador auditable)_

## 3. Namespaces

- [ ] 3.1 Publicación de `meltemi-proto` real y placeholders honestos de `meltemi`/`meltemid` (acción del mantenedor asistida) _(Req: Espacios de nombres)_

## 4. Calidad

- [ ] 4.1 Release candidato de prueba end-to-end (tag → artefactos → verificación de firma → instalación en limpio por plataforma)
