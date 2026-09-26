# anotar-el-diff

> Vía completa (proposal → design → specs → tasks): cierra una promesa
> fundacional (meltemi.md §6.8, «revisión línea a línea con comentarios que
> vuelven al agente como instrucciones») sobre RPC que ya existen, en las
> dos superficies. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brecha D6). Aprovecha
> `herramientas-a-la-vista` para los diffs del transcript.

## Why

El tablero de carrera muestra las calles lado a lado, el diff por hunk y el
merge por archivo (`desktop/ui/src/lib/views/Review.svelte`). Lo que no
tiene es la mitad humana de la revisión: **decir algo sobre una línea y que
el agente lo reciba**. Hoy el revisor lee el diff, vuelve al compositor,
escribe «en `src/parser.rs` línea 42 el manejo de error se traga el
contexto» a mano y espera haber copiado bien la ruta. Orca lo resuelve con
comentarios Markdown sobre cualquier línea, agrupados y enviados al agente
en un solo mensaje.

La pieza de Meltemi que lo hace barato ya existe: `session/direct` encola
una instrucción como siguiente turno de la sesión y la registra en el log
(`docs/acceso-remoto.md`, «Dirigir»). Un lote de comentarios es una
instrucción compuesta con las citas del hunk. Ningún RPC nuevo; ninguna
interpretación de Meltemi: el texto es del humano, el agente decide.

## What Changes

- **Comentar una línea**: en el diff de una calle del tablero y en el diff
  de una herramienta del transcript, un clic (GUI) o la tecla `c` (TUI) en
  una línea abre un cuadro de comentario; el comentario queda anclado a
  `archivo:línea` y al hunk.
- **Lote antes de enviar**: los comentarios se acumulan en un panel lateral
  («3 comentarios sin enviar»), editables y borrables, con vista previa de
  la instrucción que se compondrá: por cada comentario, la ruta, la línea,
  las líneas del hunk citadas en un bloque y el texto del revisor.
- **Enviar al agente**: compone la instrucción y la encola con
  `session/direct` en la sesión del competidor (o de la herramienta); si la
  sesión terminó y es reanudable, la reanuda, como `direct` hace hoy. El
  lote queda en el log como turno humano, con lo que trazabilidad no gana
  agujeros. Sin sesión reanudable, el botón dice por qué y ofrece copiar el
  texto.
- **Guarda de trabajo sin enviar**: cerrar la vista con comentarios en el
  lote pide confirmación, con la misma guarda que ya protege ediciones sin
  guardar.
- **Paridad**: la TUI ofrece el mismo lote en el drill-in del tablero; el
  CLI no necesita verbo nuevo: `direct` ya acepta el texto compuesto.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `worktree-orchestration`: + requisito «Comentarios anclados a líneas del
  diff, agrupados y devueltos al competidor como instrucción».
- `edit-surface`: + escenario de comentario sobre el diff de una herramienta
  del transcript.
- `gui-shell` y `tui-shell`: + el panel del lote y su guarda.

## Impact

- Archivos: `desktop/ui/src/lib/{diff.ts, views/Review.svelte,
  views/SessionDetail.svelte}` y un componente de lote, `tui/src/shell/`,
  `docs/` (la página del tablero).
- Cero dependencias nuevas; cero RPC nuevos; el daemon no cambia.
- Riesgo: un lote enviado a un competidor mientras otro turno está en vuelo;
  `direct` ya encola y el panel lo dice («se enviará al terminar el turno»).

## Fuera de alcance

- Sugerencias aplicables con un clic (estilo «suggestion» de GitHub): sería
  edición humana disfrazada; la edición va por `worktree/apply-edit` con su
  traza, como siempre.
- Comentarios persistentes fuera del log de sesión: el log es la verdad.
