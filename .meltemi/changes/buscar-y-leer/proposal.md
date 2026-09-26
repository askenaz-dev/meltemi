# buscar-y-leer

> Vía completa (proposal → design → specs → tasks): un RPC nuevo de búsqueda
> en transcripts con su paridad, más el render de Markdown y las vistas
> previas de imagen en la superficie de escritorio. Nace del comparativo con
> Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brechas D7, D5 y C-A). Absorbe las propuestas nombradas allí como
> `paleta-que-encuentra`, `buscar-en-las-sesiones`, `markdown-que-se-lee` y
> `previsualizar-binarios`.

## Why

Tres cosas que un usuario espera de una superficie de 2026 y que hoy están a
medias o ausentes:

- **Buscar**: hay filtro por texto en Sesiones, `Ctrl+P` en el Editor y la
  paleta de comandos; no hay una búsqueda que cruce sesiones, transcripts,
  archivos y comandos desde un solo sitio, ni búsqueda de texto dentro de
  los transcripts guardados. Orca la llama «Native Search» y en su changelog
  «cross-machine session search» (la mitad «cross-machine» es agregación,
  fuera por §3; la mitad «session search» no).
- **Leer**: los mensajes del agente se pintan como texto plano. Un agente
  responde en Markdown —listas, código, tablas— y el transcript lo muestra
  con los asteriscos. Orca renderiza y añade botones de copiar en el código.
- **Ver**: no hay ninguna `<img>` en `desktop/ui/src`; el Editor abre una
  imagen como texto y un PDF también, aunque el CSP del webview ya admite
  `img-src 'self' data:`. Sin esto, `adjuntos-en-el-compositor` no tendría
  dónde mostrar lo que adjunta.

## What Changes

- **`session/search`**: búsqueda de texto en los registros de sesión del
  proyecto activo (título, prompt, mensajes del agente, títulos de
  herramienta), con paginación y resaltado de coincidencias; ejecutada por
  el daemon sobre sus propios JSONL. Paridad: la paleta de la TUI y
  `meltemi sessions --grep <texto>` en el CLI.
- **Paleta que encuentra**: un solo campo (`Ctrl+K`) con grupos —comandos,
  sesiones, resultados en transcripts, archivos del proyecto (vía
  `lectura-por-el-daemon` cuando el daemon es remoto; disco local si no)— y
  el mismo orden por frecuencia que ya tiene.
- **Markdown que se lee**: render sanitizado de los mensajes del agente
  (encabezados, listas, código con botón de copiar, tablas); sin imágenes
  remotas (CSP) y con las locales resueltas por el daemon para que sirva
  también por túnel. Un conmutador «ver texto» conserva el crudo.
- **Vista previa de imágenes en el Editor** (PNG, JPG, SVG, WebP) y del
  Markdown del repositorio renderizado; los PDF abren con «Abrir con…» del
  sistema, no se embebe visor.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `session-history`: + requisito «Búsqueda de texto en los registros de
  sesión» (`session/search`).
- `cli-contract`: + `sessions --grep`.
- `tui-shell`: + búsqueda de sesiones en la paleta.
- `gui-shell`: + paleta unificada; + render Markdown sanitizado con
  conmutador a texto.
- `edit-surface`: + vista previa de imágenes y de Markdown del repositorio.

## Impact

- Archivos: `core/meltemid/src/{session_index.rs, server.rs}`,
  `proto/schemas/v1/session-search.schema.json` (nuevo), registros de
  paleta (gate de paridad), `desktop/ui/src/lib/{components/Palette.svelte,
  conversation.ts, views/SessionDetail.svelte, views/Editor.svelte}`,
  `tui/src/shell/palette.rs`, `tui/src/run.rs`, `docs/paridad-nucleo.md`.
- Dependencia candidata: un renderizador de Markdown en el frontend. Se
  justifica en el design contra el presupuesto de tamaño del instalador y
  la política de dependencias (§10); si no cabe, se escribe el subconjunto
  necesario.
- Riesgo real: Markdown sin sanear es inyección de HTML en el webview; el
  design fija el saneado y un test con carga hostil.

## Fuera de alcance

- Búsqueda entre máquinas: §3 (un daemon por túnel).
- Adjuntar imágenes al prompt: `adjuntos-en-el-compositor`.
- Visor de PDF embebido: fuera de la edición utilitaria del rumbo.
