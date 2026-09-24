# adjuntos-en-el-compositor

> Vía completa (proposal → design → specs → tasks): el prompt gana bloques
> de imagen condicionados a lo que el agente anuncia, el log gana una forma
> de registrar un adjunto sin sus bytes, y el CLI su paridad. Nace del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha D3, mitad de
> imágenes). Depende de `arrastrar-referencias` y de `buscar-y-leer`.

## Why

«Drag-drop files or Finder images into an agent prompt», «image attachments
in chat»: para un agente de código, una captura de pantalla de la UI rota
vale más que tres párrafos. ACP lo contempla —bloques de contenido de imagen
en el prompt, condicionados a `promptCapabilities.image`— y el compositor
de Meltemi es solo texto. Con `arrastrar-referencias` una imagen soltada
dice «todavía no»; esta change es el «ya».

Dos reglas de la casa deciden la forma: solo si el agente lo anuncia
(constitución §5: nada se asume del proveedor; si no anuncia imagen, se
rehúsa con la razón), y el log apend-only registra el prompt sin bytes —
un adjunto queda como huella (tipo, tamaño, hash y una copia en el
directorio de datos), no como carga dentro del JSONL.

## What Changes

- **Adjuntar una imagen** por arrastre, pegado o selector, en el compositor
  de Home y de la sesión; chip con miniatura, nombre y tamaño; se envía como
  bloque de imagen ACP junto al texto cuando el agente anunció la
  capacidad, y se rehúsa con remedio cuando no («este agente no acepta
  imágenes por ACP»).
- **Huella en el log**: `prompt_sent` gana `attachments[{kind, bytes,
  sha256, storedAt}]`; la copia vive en `<data_dir>/attachments/` con el hash
  como nombre, y el transcript la muestra con la vista previa de
  `buscar-y-leer`.
- **Paridad en el CLI**: `--attach <ruta>` en `session`, `direct` y
  `propose`, con el mismo rehúso cuando el agente no lo admite; la TUI
  ofrece el selector en su compositor.
- **Límites declarados**: tamaño máximo por adjunto y por prompt, formatos
  admitidos, y qué pasa con los adjuntos al exportar una sesión.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «Adjuntos de imagen condicionados a la
  capacidad anunciada, con huella en el log y sin bytes en el JSONL».
- `cli-contract`: + `--attach`.
- `gui-shell` y `tui-shell`: + el chip y el selector.

## Impact

- Archivos: `core/meltemid/src/{acp.rs, session_log.rs, free_session.rs,
  propose.rs}`, `proto/schemas/v1/{session-start, session-direct,
  session-event}.schema.json` (aditivos), `desktop/ui/src/lib/views/{Home,
  SessionDetail}.svelte`, `tui/src/`, `docs/`.
- Cero dependencias nuevas.
- Riesgo: el tamaño del directorio de adjuntos con el tiempo; el design fija
  retención y la muestra en `meltemi usage`.

## Fuera de alcance

- Archivos que no sean imágenes: para texto ya existe `@ruta`; para
  binarios no hay caso de uso declarado.
- Capturar pantalla desde Meltemi: es del sistema del usuario.
