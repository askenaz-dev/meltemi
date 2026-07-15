# Delta: edit-surface (enmienda-edicion-movil)

## ADDED Requirements

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
Cuando el worktree destino tiene una sesión de agente activa, la superficie de edición SHALL advertir al usuario antes de aplicar el guardado. La política completa de concurrencia (bloqueo suave, notificación al agente vía ACP) se decide en el design de la change de GUI de fase 2; esta advertencia es el mínimo exigible.

#### Scenario: Edición sobre worktree con agente trabajando
- **WHEN** el usuario intenta guardar una edición in situ en un worktree donde un agente tiene una sesión activa
- **THEN** la superficie advierte del riesgo de conflicto y requiere confirmación explícita antes de aplicar la escritura
