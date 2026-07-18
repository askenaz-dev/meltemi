## Why

"La revisión de specs es la obsesión" (§4.9): si revisar un delta es incómodo,
el humano aprueba sin leer y el método muere. Con el ciclo de autoría (#14)
generando artefactos, hace falta la superficie que haga la revisión *placentera*
en terminal: diff de deltas legible, contradicciones presentadas, checklist
interactiva, y que un comentario vuelva al agente como instrucción.

## What Changes

- **Render de diff de deltas** en terminal: ADDED/MODIFIED/REMOVED/RENAMED por
  requisito y escenario (no por líneas), con la accesibilidad baseline del shell
  (glifo+palabra, ASCII, NO_COLOR).
- **Presentación de contradicciones y huecos**: los diagnósticos del motor
  (estructura + EARS + los semánticos que esta change introduce sobre
  `meltemi-spec`: duplicados, términos en conflicto, requisitos sin cobertura de
  delta) se muestran anclados al requisito.
- **Checklist interactiva de `/review`**: recorre requisito por requisito;
  aprobar / comentar / rechazar por ítem; el estado de revisión persiste en la
  change.
- **Comentario → instrucción**: un comentario de revisión se convierte en prompt
  dirigido al agente autor para reelaborar ese artefacto (bucle con gate).

## Capabilities

### New Capabilities
- `spec-review-ux`: diff de deltas, checklist, comentarios-a-instrucción.

### Modified Capabilities
- `spec-engine`: diagnósticos semánticos (contradicciones/huecos) — la parte
  diferida explícitamente desde `motor-ears-deltas`.
- `sdd-authoring`: `/review` se integra como gate enriquecido del ciclo.

## Impact

- `core/meltemi-spec` (análisis semántico), `core/meltemid` (estado de revisión),
  `tui/` (render + checklist). Sin red, sin deps nuevas previstas.

## Fuera de alcance

- Revisión de **código** línea a línea con LSP (superficie de revisión de la GUI,
  fase 2; en terminal llega con #18/worktrees el diff de código básico).
- Edición in situ de specs en la TUI (cerca de `edit-surface`; el retoque va vía
  comentario→instrucción o `$EDITOR`).
