# edit-surface Specification

## Purpose
TBD - created by archiving change enmienda-edicion-movil. Update Purpose after archive.
## Requirements

### Requirement: Cerca de la edición utilitaria
La superficie de edición de código de Meltemi SHALL limitarse a la edición utilitaria al servicio del bucle agéntico (revisar → retocar → dirigir), definida por esta cerca.

**DENTRO** (admisible por el cauce normal de changes):
- Árbol del proyecto; abrir, editar y guardar archivos del worktree.
- Resaltado de sintaxis e inteligencia LSP: autocompletado, diagnósticos, ir-a-definición, renombrar, formatear, referencias.
- Pestañas multi-archivo y búsqueda en el proyecto.
- Edición de hunks en la vista de diff (cambio sugerido aplicable).
- Deep-link "Abrir con…" hacia el editor del usuario (ver requisito propio).

**FUERA** (para siempre; su inclusión exige enmienda fundacional previa):
- Ecosistema de plugins o extensiones de editor.
- Depurador integrado.
- Emulación completa de otros editores o esquemas de keybindings configurables estilo IDE.
- Cualquier capacidad cuya justificación sea migrar la autoría sostenida del usuario a Meltemi.

Toda propuesta de capacidad de edición SHALL evaluarse contra el principio rector: **Meltemi optimiza para que salir sea infrecuente, no imposible**.

#### Scenario: Propuesta de edición dentro de la cerca
- **WHEN** una propuesta de cambio introduce una capacidad de edición listada como DENTRO
- **THEN** se tramita por el cauce normal de changes, sin enmienda fundacional

#### Scenario: Propuesta de edición fuera de la cerca
- **WHEN** una propuesta de cambio introduce una capacidad de edición listada como FUERA, o justificada como "para no tener que salir de Meltemi" sin servir al bucle agéntico
- **THEN** la propuesta se rechaza salvo que una enmienda fundacional aprobada amplíe la cerca previamente

### Requirement: Deep-link al editor del usuario
Las superficies SHALL ofrecer "Abrir con…" hacia el editor que el usuario ya usa, con archivo y línea exactos, desde la vista de diff y desde el árbol del proyecto. En la TUI, la apertura SHALL realizarse suspendiendo al editor definido por el usuario (`$EDITOR` o configuración equivalente) y retornando a la sesión al cerrarlo.

#### Scenario: Apertura desde la GUI con posición exacta
- **WHEN** el usuario invoca "Abrir con…" sobre una línea de un diff o un archivo del árbol
- **THEN** el editor configurado por el usuario se abre en ese archivo y línea, sin cerrar ni bloquear la sesión de Meltemi

#### Scenario: Apertura desde la TUI
- **WHEN** el usuario invoca la apertura externa desde la TUI
- **THEN** la TUI se suspende, cede la terminal al editor del usuario y retoma la sesión al salir de este

### Requirement: Trazabilidad de ediciones humanas
Toda edición in situ SHALL materializarse como capacidad del daemon: la escritura al worktree pasa por `meltemid` y queda registrada como evento `human_edit` (archivo, sesión, marca temporal) en el log de sesión JSONL apend-only. Ninguna superficie SHALL escribir al worktree por fuera de esta capacidad.

#### Scenario: Guardado de una edición in situ
- **WHEN** el usuario guarda una edición realizada en la superficie de edición de cualquier cliente
- **THEN** el daemon aplica la escritura al worktree y registra un evento `human_edit` en el log de la sesión correspondiente

### Requirement: Advertencia por sesión de agente activa
La política de concurrencia humano↔agente sobre un mismo worktree SHALL ser de
bloqueo suave, nunca bloqueo duro. El daemon SHALL exponer el estado del
worktree en tres niveles — libre, sesión activa sin turno en vuelo, turno en
vuelo — y las superficies de edición SHALL comportarse según ese estado: con
turno en vuelo, el guardado MUST exigir confirmación reforzada que advierta el
riesgo de conflicto; con sesión activa sin turno en vuelo, el guardado SHALL
advertir y pedir confirmación simple; con worktree libre, el guardado procede
sin fricción. Toda edición aplicada SHALL registrarse como `human_edit`, y el
daemon MUST anteponer al siguiente turno del agente una nota con los archivos
editados por el humano desde su último turno — la nota viaja en el prompt del
turno; no se inventa una notificación push fuera de ACP.

#### Scenario: Edición con turno en vuelo
- **WHEN** el usuario guarda una edición in situ en un worktree cuyo agente tiene un turno en vuelo
- **THEN** la superficie SHALL exigir confirmación reforzada advirtiendo el riesgo de conflicto
- **AND** al confirmar, la escritura SHALL aplicarse vía el daemon y registrarse como `human_edit`

#### Scenario: Edición entre turnos
- **WHEN** el usuario guarda una edición in situ con sesión activa pero sin turno en vuelo
- **THEN** la superficie SHALL advertir y pedir confirmación simple antes de aplicar

#### Scenario: Worktree libre sin fricción
- **WHEN** el usuario guarda una edición in situ en un worktree sin sesión de agente activa
- **THEN** la escritura SHALL aplicarse sin confirmación adicional
- **AND** SHALL registrarse igualmente como `human_edit`

#### Scenario: Nota al siguiente turno del agente
- **WHEN** el agente inicia su siguiente turno tras ediciones humanas en su worktree
- **THEN** el daemon SHALL anteponer al turno una nota con los archivos editados desde el último turno
- **AND** la nota SHALL quedar evidenciada en el log de sesión
