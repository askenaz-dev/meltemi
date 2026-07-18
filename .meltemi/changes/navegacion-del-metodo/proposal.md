## Why

La primera fricción real del método post-migración la encontró su primer
usuario: `openspec view` ya no lista nada (el árbol prestado quedó retirado) y
**Meltemi no tiene verbos para navegar su propio método** — no hay forma de
preguntar "¿qué changes existen y en qué estado están?", ni de mostrar una
change o una spec viva, ni de validar sin archivar. La auditoría de paridad
contra OpenSpec **v1.6.0** (la instalada == la última upstream, verificada
2026-07-18) delimita el hueco cubrible: `list`, `show`, `status` y `validate`
son operaciones diarias del método que la herramienta prestada daba y el
producto aún no. El resto de su superficie (stores, worksets, schemas,
dashboard interactivo, completions) es filosofía distinta o comodidad no
esencial, y queda declarado fuera — no en silencio.

## What Changes

- **Listado de changes** (`change/list`): activas y archivadas con su estado
  agregado — artefactos presentes (proposal/design/specs/tasks), progreso de
  tareas (n/m ticks), estado de review (pendientes/decididos) y de verify
  (escenarios verificados/total). El `status` de OpenSpec queda absorbido como
  columnas del listado.
- **Mostrar** (`change/show`, `spec/list`, `spec/show`): una change con sus
  artefactos y deltas; las capacidades de la verdad viva y una spec con sus
  requisitos y escenarios.
- **Validación independiente** (`sdd/validate`): la validación del motor
  (estructura + EARS) más la aplicación en seco de los deltas contra la verdad
  viva — hoy solo corre dentro de `archive` — como verbo propio, por change o
  sobre la verdad viva completa (doctor-lite). Con señal scriptable para CI:
  código de salida nuevo `14` (validación con hallazgos).
- **CLI**: `changes`, `show <change|spec>`, `validate [change]`, todos con
  `--json` (paridad §4: métodos del daemon consumibles por TUI/GUI por igual;
  la vista Proyecto de la TUI consumirá `change/list` como delta futuro).

## Capabilities

### New Capabilities
- `method-navigation`: listar, mostrar y validar el árbol del método.

### Modified Capabilities
- `cli-contract`: la taxonomía de códigos de salida gana `14` (validación con
  hallazgos).

## Impact

- `core/meltemid` (lectores agregados sobre estado ya persistido + validate
  reusando `meltemi-spec` y `archive::dry_run_diagnostics`), `proto/` (métodos
  y tipos aditivos), `tui/` (tres subcomandos + renders).
- Solo lectura: ningún verbo nuevo muta el árbol del método.

## Fuera de alcance

- **Dashboard interactivo** (`openspec view`): la TUI es esa superficie; su
  vista de changes consume `change/list` en un delta propio de `tui-shell`.
- **Stores / worksets / schemas**: modelos de organización de la herramienta
  prestada; Meltemi no los adopta sin evidencia de demanda (fase futura).
- **`feedback`**: un verbo que envía datos fuera contradice la postura §9 (sin
  telemetría); el canal es el repositorio público.
- **Shell completions y `config` viewer**: comodidades, deltas menores futuros.
- **CLI para `review-decide`/`verify-mark`**: hueco observado distinto (decidir
  ítems desde CLI); mini-delta separado si el uso lo pide.
- **`bulk-archive`**: el archivado es uno-a-uno con gate por diseño.
