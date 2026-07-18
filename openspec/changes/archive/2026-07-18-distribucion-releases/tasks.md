## 1. Política y pipeline

- [x] 1.1 Política de versionado escrita (qué rompe) + versión de workspace y tags _(Req: Versionado)_ — `docs/versionado.md` (ruptura = contrato `proto/`, gramática CLI, formato de artefactos); versión única en `[workspace.package]`; releases desde tag.
- [x] 1.2 Pipeline de release 3 plataformas: build+suite+clippy+fmt+cargo-deny+presupuestos (§12) con aborto en rojo _(Req: Pipeline con gates)_ — `.github/workflows/release.yml`: gates duros + gate de presupuesto del binario TUI (`MELTEMI_TUI_BUDGET_BYTES`, `exit 1`).

## 2. Firma e instalación

- [x] 2.1 Checksums+firma de artefactos e instrucciones de verificación; procedimiento de custodia documentado con el mantenedor _(Req: Artefactos firmados)_ — `SHA256SUMS` en el pipeline; `docs/release.md` documenta verificación de checksum+firma y la custodia de clave (del mantenedor, nunca en el repo; el paso de firma corre con los secretos del release).
- [x] 2.2 Instalador auditable por SO (hash publicado + manual equivalente) que instala binarios y crea `mel` _(Req: Instalador auditable)_ — `scripts/install.sh` y `scripts/install.ps1`: cortos, legibles, verifican checksum y rehúsan ante desajuste; instalan `meltemi`+`meltemid` y crean `mel`; equivalente manual en la cabecera.

## 3. Namespaces

- [x] 3.1 Publicación de `meltemi-proto` real y placeholders honestos de `meltemi`/`meltemid` (acción del mantenedor asistida) _(Req: Espacios de nombres)_ — los tres crates llevan `description` + `repository` (metadata publicable); `docs/release.md` describe la reserva de namespace. La publicación efectiva y el flip de `publish` son acción del mantenedor (placeholders con `publish = false` como salvaguarda en el árbol).

## 4. Calidad

- [x] 4.1 Lint de release verde (política, gates+presupuesto, firma/custodia, instaladores con `mel`, metadata de crates) — `core/meltemid/tests/release.rs`. El release candidato end-to-end (tag→artefactos→firma→instalación limpia) es una operación del mantenedor con este pipeline; el lint garantiza que las piezas están presentes y coherentes.
