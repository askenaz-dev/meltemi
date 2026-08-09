# gui-shell — delta

## ADDED Requirements

### Requirement: Avisos transitorios y avisos que se quedan

Los avisos SHALL distinguirse por consecuencia. Un aviso informativo —el que
confirma algo que el usuario acaba de hacer— SHALL retirarse solo tras un plazo
breve, y SHALL poder retirarse antes con su control. Un aviso de advertencia o
de error SHALL permanecer hasta que el usuario lo retire, y NO SHALL existir
plazo alguno capaz de retirarlo: la obligación de que un vencimiento o un error
no se descarte en silencio se conserva intacta. Mientras el puntero o el foco
estén sobre un aviso transitorio, su plazo SHALL detenerse, y al salir SHALL
reiniciarse.

#### Scenario: La confirmación se retira sola

- **WHEN** una operación informa de su éxito
- **THEN** su aviso SHALL retirarse solo tras un plazo breve
- **AND** SHALL poder retirarse antes desde su control

#### Scenario: El error se queda hasta que alguien lo retira

- **WHEN** el aviso es de advertencia o de error
- **THEN** NO SHALL existir plazo capaz de retirarlo
- **AND** SHALL permanecer hasta que el usuario lo retire

#### Scenario: Nada desaparece bajo la mano que iba a leerlo

- **WHILE** el puntero o el foco están sobre un aviso transitorio
- **THEN** su plazo SHALL detenerse
- **AND** al salir SHALL reiniciarse

### Requirement: Ninguna superficie flotante se desplaza de lado

Los paneles, cajones y diálogos de la GUI SHALL desplazarse solo en vertical: su
contenido SHALL partirse para caber en el ancho disponible en vez de producir
una barra de desplazamiento horizontal.

#### Scenario: El cajón parte la ruta larga en vez de desplazarla

- **WHEN** un cajón muestra un contenido más ancho que su panel
- **THEN** el contenido SHALL partirse para caber
- **AND** NO SHALL aparecer una barra de desplazamiento horizontal

### Requirement: Todo velo cierra lo que cubre

Toda superficie que se presente sobre un velo SHALL cerrarse al activar el velo,
además de con la tecla de escape. La suite de la superficie de escritorio MUST
fallar, nombrando el componente, cuando exista un velo sin manejador de cierre.

#### Scenario: Hacer clic fuera cierra la paleta

- **WHEN** el usuario hace clic fuera del área de la paleta de comandos
- **THEN** la paleta SHALL cerrarse

#### Scenario: Ningún velo queda sin cierre

- **WHEN** la suite de la superficie de escritorio se ejecuta
- **THEN** SHALL reunir todo velo de la superficie
- **AND** SHALL fallar nombrando el componente de cualquiera sin manejador de
  cierre
