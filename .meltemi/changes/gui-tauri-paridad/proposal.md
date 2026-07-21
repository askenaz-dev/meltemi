## Why

Fase 1 está cerrada (hito v0.1 archivado) y la GUI prometida por el documento
fundacional sigue sin existir: meltemi.md la fija como decisión de arquitectura
(D4 §7.4, Tauri) y como corazón de la fase 2 (§10 — "GUI Tauri con paridad de
núcleo"). Hoy toda la potencia del daemon es consumible solo desde la terminal;
la constitución §4 exige que ninguna capacidad viva en una sola superficie, y
los públicos 2 y 3 del rumbo (tech leads, mantenedores) esperan la superficie
visual donde el método brilla: editor de specs enriquecido, revisión de diffs
línea a línea, bandeja de permisos. Además, tres deudas del plan apuntan aquí:
`docs/paridad-nucleo.md` (la matriz viva de paridad) y `docs/ux/design-system.md`
(insumo declarado de esta change) no existen, y la spec `edit-surface` difirió
explícitamente "la política completa de concurrencia humano↔agente" al design
de la change de GUI de fase 2. Esta change salda las tres.

## What Changes

- **Cliente de escritorio `desktop/`** (Tauri 2, miembro del workspace):
  cliente fino cuyo backend Rust es el único dueño de la conexión JSON-RPC al
  socket local (named pipe en Windows, UDS en macOS/Linux); la webview jamás
  abre sockets ni carga contenido remoto (deny-by-default, constitución §3).
- **Shell con paridad**: vistas Sesiones / Proyecto / Permisos / Flota con
  drill-in, paleta de comandos con registro obligatorio de todo método RPC,
  estados vacíos honestos, desconexión ruidosa y reconexión con backoff — las
  mismas garantías que la TUI ya especifica.
- **Superficies ricas de fase 2**: editor de specs enriquecido con findings de
  `validate` en vivo; revisión de diffs línea a línea con edición de hunks;
  bandeja de permisos con prioridad de señales; panel de flota con perfiles.
- **Edición utilitaria in situ** (dentro de la cerca `edit-surface`): árbol,
  pestañas, LSP BYO del usuario; todo guardado pasa por el daemon — método
  aditivo `worktree/apply-edit` con evento `human_edit` — y la política de
  concurrencia humano↔agente queda resuelta: bloqueo suave, nunca bloqueo duro,
  con nota al siguiente turno del agente.
- **Paridad verificable**: `docs/paridad-nucleo.md` (capacidad → RPC → CLI/TUI
  → GUI) con verificación en CI que falla si un método del contrato queda sin
  casa en alguna superficie (hito v1.0: "paridad verificada por CI").
- **Design system**: `docs/ux/design-system.md` derivado de brand V2, con
  tokens compartibles con el compañero móvil futuro.
- **Distribución**: instaladores GUI firmados por plataforma en el pipeline
  existente; presupuestos del fundacional: instalador < 15 MB, RAM en reposo
  < 80 MB, arranque < 1 s.

## Capabilities

### New Capabilities
- `gui-shell`: la superficie de escritorio — shell con paridad, paleta con
  registro de métodos, permisos, editor de specs, revisión de diffs, edición
  in situ con LSP BYO, accesibilidad, i18n ES/EN, presupuestos de huella y
  seguridad deny-by-default.

### Modified Capabilities
- `edit-surface`: la advertencia mínima por sesión activa se sustituye por la
  política completa de concurrencia humano↔agente (bloqueo suave en tres
  niveles + trazabilidad `human_edit` + nota al siguiente turno), saldando la
  deuda que la propia spec declaró.
- `release-distribution`: + instaladores firmados de la GUI por plataforma con
  gate de tamaño.

## Impact

- Nuevo `desktop/` (crate Tauri + frontend); extracción del cliente JSON-RPC de
  `tui/` a un crate compartido (`core/meltemi-client`) sin cambio de contrato;
  `core/meltemid` (estado turno-en-vuelo por worktree, `worktree/apply-edit` +
  evento `human_edit`, nota de ediciones al siguiente turno); `proto/` (método
  y tipos aditivos); `tui/` (paridad del método nuevo: subcomando CLI + registro
  en paleta); CI (matriz de paridad, gate de tamaño de instalador, build GUI en
  las tres plataformas); `docs/` (paridad-nucleo, ux/design-system).
- E2e: contra fixtures temporales con `mock-agent`, como siempre; la lógica del
  shell vive en el backend Rust (testeable sin webview); smoke de la webview
  vía tauri-driver donde la plataforma lo soporte, verificación manual
  documentada donde no.

## Fuera de alcance

- **Changes hermanas de fase 2** ya nombradas en el plan: motor propio BYOK,
  sandbox propio, hooks, plugins/skills/SDK, i18n de más superficies, métricas
  SDD locales, LSP de la superficie de revisión.
- **La zona FUERA de la cerca `edit-surface`**: ecosistema de plugins,
  depurador, emulación de editores — siguen exigiendo enmienda fundacional.
- **Compañero móvil**: fase 3, acotado por `mobile-companion`.
- **Mini-edición de hunks en la TUI**: la paridad de poder del método nuevo
  queda cubierta por CLI y paleta; la experiencia de hunks en TUI es una change
  futura.
- **Empaquetar servidores LSP**: BYO-LSP; la GUI usa los del usuario y degrada
  con honestidad (presupuesto de instalador y constitución §10).
