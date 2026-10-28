# tui-shell — delta

## ADDED Requirements

### Requirement: Énfasis con el color de la marca

Cuando el color está permitido, el shell SHALL pintar lo que destaca como
bloque —el banner, los contadores de la cabecera, el título del panel
enfocado y la fila seleccionada— con el azul Egeo de la marca de fondo y
texto blanco en negrita, y SHALL pintar los títulos de vista y las cabeceras
de grupo en negrita con ese azul sobre el fondo del terminal, sin bloque. El
azul SHALL ser el exacto (`#2563eb`) cuando el terminal anuncia color de 24
bits (`COLORTERM=truecolor` o `24bit`) y el más cercano de la paleta de 256
colores en otro caso. Sin color, ambos estilos SHALL ser el vídeo inverso, sin
pintar fondo. El significado MUST seguir en glifo, etiqueta y disposición.

#### Scenario: Énfasis con color

- **WHERE** el color está permitido
- **WHEN** se renderiza la vista de sesiones con una petición pendiente y una fila seleccionada
- **THEN** el banner y la fila seleccionada SHALL llevar el acento de fondo y texto blanco en negrita
- **AND** la cabecera de cada proyecto SHALL llevar el acento como color de texto, sin fondo

#### Scenario: El acento según el terminal

- **WHEN** `COLORTERM` vale `truecolor` o `24bit`
- **THEN** el acento SHALL ser `#2563eb` exacto
- **WHEN** `COLORTERM` no está o vale otra cosa
- **THEN** el acento SHALL ser el índice 27 de la paleta de 256 colores

#### Scenario: Monocromo conserva el inverso

- **WHERE** `NO_COLOR` está definida con valor no vacío
- **WHEN** se renderiza cualquier vista
- **THEN** ninguna celda SHALL llevar color de texto ni de fondo
- **AND** el énfasis SHALL ser vídeo inverso

#### Scenario: La captura muestra los colores

- **WHEN** se genera `docs/media/tui-sessions.svg` con `capture_svg`
- **THEN** la captura SHALL ser SVG bien formado
- **AND** SHALL llevar el acento de la marca como `fill` de los tramos en bloque
