# arrastrar-referencias

> Vía rápida (fast-forward): solo cliente de escritorio, ningún RPC nuevo, la
> referencia `@ruta` que el daemon ya resuelve. Nace del comparativo con Orca
> del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brecha D3
> en su mitad de texto).

## Why

Orca deja arrastrar archivos o imágenes del explorador al prompt del agente.
Meltemi tiene el mecanismo de referencia de contexto —`@archivo` y
`@carpeta` en los prompts, resueltos por el daemon
(`core/meltemid/src/repo_context`, spec `repo-context`)— pero solo se escribe
a mano: el árbol del editor, la lista de archivos de una calle del tablero y
el explorador del sistema no se pueden soltar sobre el compositor. Para el
tech lead que revisa un diff y quiere decirle al agente «mira este archivo»,
son cuatro pasos donde Orca tiene uno.

El webview ya puede recibirlo: Tauri entrega los eventos de arrastre del
sistema al cliente sin abrir red ni sockets, así que la política del
`gui-shell` (sin red en el webview, capacidades mínimas) no se toca; solo
hay que declarar la capacidad de arrastre y convertir lo soltado en la
referencia que el daemon ya entiende.

## What Changes

- **Soltar sobre el compositor inserta la referencia**: un archivo o carpeta
  del sistema, del árbol del Editor o de la lista de archivos del tablero,
  soltado sobre el compositor de Home o de la sesión, se convierte en
  `@<ruta relativa al proyecto activo>` en el punto del cursor, con un chip
  de vista previa (nombre, tamaño) mientras se arrastra.
- **Fuera del proyecto, rehúso con remedio**: una ruta que no cuelga de la
  raíz del proyecto activo ni de un worktree gestionado no se referencia
  («fuera del proyecto: regístralo o ábrelo primero»); Meltemi no arrastra
  contenido ajeno al agente por accidente.
- **Solo referencias, no bytes**: el log de sesión sigue guardando el prompt
  con la referencia, nunca el contenido; lo que el agente lee lo lee por el
  proxy, como hoy.
- **La imagen se rehúsa hoy, con la razón**: soltar una imagen dice que los
  adjuntos llegan con `adjuntos-en-el-compositor`, condicionados a que el
  agente anuncie `prompt_capabilities.image`.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `gui-shell`: + requisito «Arrastrar al compositor produce una referencia de
  contexto acotada al proyecto».
- `edit-surface`: + escenario: arrastrar desde el árbol del editor y desde el
  tablero.

## Impact

- Archivos: `desktop/ui/src/lib/views/{Home.svelte, SessionDetail.svelte,
  Editor.svelte, Review.svelte}`, `desktop/ui/src/lib/tree.ts`,
  `desktop/capabilities/` (el evento de arrastre, mínimo), `desktop/tests/`.
- Cero dependencias nuevas; cero RPC. Deber de paridad: ninguno — el daemon
  no gana capacidad; la TUI ya acepta `@ruta` escrita.
- Riesgo: rutas con espacios o acentos en Windows; se pinea con un test que
  normaliza y con el smoke visual.

## Fuera de alcance

- Adjuntos de imagen y archivos binarios: `adjuntos-en-el-compositor`.
- Autocompletado de `@` mientras se escribe: si no existe, es otra vía rápida
  (`buscar-y-leer` la contempla).
