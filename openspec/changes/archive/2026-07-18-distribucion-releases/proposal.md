## Why

"Binario único autocontenido, instalable con un comando" (§4.5) es promesa
fundacional y todavía no existe ni un release. Antes del hito v0.1: versionado
disciplinado, artefactos firmados con custodia de claves seria, empaquetado por
plataforma e instalador de una línea. Además hay pendientes operativos que este
release materializa: reservar los crates (anti-squatting) y publicar el alias
`mel` (diferido desde `cli-contrato`).

## What Changes

- **Versionado**: SemVer pre-1.0 con política escrita (qué rompe qué); versión
  única de workspace; `--version` ya la expone.
- **Empaquetado por plataforma**: binarios `meltemi` + `meltemid` para
  Windows/macOS/Linux (arquitecturas según `docs/plataformas.md`); el alias
  `mel` se materializa aquí (symlink/copia según SO).
- **Firmado y custodia**: checksums publicados + firma de artefactos; el
  procedimiento de custodia de claves es del mantenedor y queda documentado
  (quién, dónde, rotación).
- **Instalador de una línea**: script auditable por SO (sin `curl | sh` ciego:
  se publica el hash y el script es legible), más instrucciones manuales.
- **CI de release**: pipeline reproducible que compila las 3 plataformas, corre
  la suite completa y publica; **cargo-deny** en el gate (constitución §10).
- **Crates**: publicación (aunque sea placeholder mínimo honesto) de `meltemi`,
  `meltemid`, `meltemi-proto` para asegurar el namespace — acción del mantenedor
  con soporte de esta change.
- **Deuda tocada**: evaluación de extraer `meltemi-client` (hoy la TUI enlaza
  `meltemid` como lib) si el tamaño del binario lo justifica; presupuestos §12
  verificados en release (TUI < 25 MB).

## Capabilities

### New Capabilities
- `release-distribution`: versionado, empaquetado, firma, instalador y gates.

### Modified Capabilities
- _Ninguna en specs de producto_ (el alias `mel` cumple lo ya escrito en
  `cli-contract`).

## Impact

- CI/CD (workflows), scripts de empaquetado, documentación de instalación.
  Ninguna feature nueva de producto.

## Fuera de alcance

- Gestores de paquetes de terceros (homebrew/winget/apt) — post-v0.1.
- Auto-update del binario (jamás sin opt-in explícito; change futura).
- GUI Tauri e instalador de escritorio (fase 2).
