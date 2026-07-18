## Context

"Binario único, instalable con un comando" (§4.5) sin un solo release publicado.
Pendientes operativos que esta change materializa: crates reservados, alias
`mel` (diferido desde `cli-contrato`), presupuestos §12 verificados.

## Goals / Non-Goals

**Goals:** SemVer pre-1.0 con política; pipeline de release 3 plataformas con
gates (incluido cargo-deny y presupuestos); firma+checksums+custodia documentada;
instalador auditable; crates publicados.
**Non-Goals:** gestores de paquetes de terceros (post-v0.1); auto-update (jamás
sin opt-in, change futura); GUI (fase 2).

## Decisions

### D1 — Versión única de workspace, SemVer pre-1.0
`0.x`: minor = puede romper, patch = no rompe; la política escrita define qué es
"romper" (contrato proto, gramática CLI, formato de artefactos). El tag dispara
el pipeline.

### D2 — Pipeline con gates duros
Por plataforma (Windows/macOS/Linux): build release, suite completa, clippy/fmt,
**cargo-deny** (constitución §10), presupuestos §12 medidos (binario TUI < 25 MB;
arranque < 1 s en el runner) — cualquier gate rojo aborta el release.

### D3 — Firma y custodia honestas
Checksums SHA-256 publicados junto a los artefactos + firma (minisign o
equivalente ligero — decisión final con el mantenedor, que custodia la clave;
procedimiento documentado: generación, almacenamiento, rotación, revocación).

### D4 — Instalador auditable, no `curl | sh` ciego
Script por SO, corto y legible, con hash publicado y instrucciones manuales
equivalentes al lado. Instala `meltemi`+`meltemid` y crea el alias `mel`
(symlink/copia según SO) — cumpliendo lo ya escrito en `cli-contract`.

### D5 — Crates reservados con contenido honesto
`meltemi-proto` se publica real (tipos del contrato); `meltemi` y `meltemid`
como placeholders mínimos honestos (README que apunta al repo) hasta que el
release binario madure. Acción del mantenedor asistida por esta change.

## Risks / Trade-offs

- **Custodia de claves unipersonal** → documentada con revocación; segunda
  persona cuando exista otro mantenedor (#21 declara el camino).
- **Runners ≠ máquinas reales** para presupuestos → medidos igual (tendencia),
  con margen declarado.

## Migration Plan

Solo CI/CD y scripts; el código no cambia (si el tamaño exige extraer
`meltemi-client`, vuelve como delta con evidencia).

## Open Questions

- Herramienta de firma definitiva (decisión con el mantenedor en el design
  review al frente de la cola).
