# tui-shell — delta

## ADDED Requirements

### Requirement: Tablero de carrera en el shell

El shell interactivo SHALL ofrecer el tablero de la carrera como superficie
alcanzable desde la paleta de comandos, sin alterar el contrato de dígitos
de las vistas de primer nivel. Las calles SHALL declarar su estado con
glifo y palabra — nunca solo color — y su procedencia (agente, y perfil
cuando exista) con la ausencia visible, y el diff de cada calle SHALL ser
legible con el desplazamiento del shell bajo un tope declarado. Un
despacho lanzado desde el tablero MUST NOT congelar el shell: la petición
larga corre aparte del bucle de refresco y el tablero refleja su
conclusión.

#### Scenario: El verbo de carrera abre el tablero

- **WHEN** el usuario invoca el verbo de carrera desde la paleta
- **THEN** el shell SHALL abrir el tablero con las calles de la tarea
- **AND** cada calle SHALL mostrar su procedencia cuando esté registrada
- **AND** el verbo MUST NOT seguir anunciado como reservado

#### Scenario: El despacho no congela el shell

- **WHILE** un despacho corre su turno completo desde el tablero
- **THEN** el bucle de refresco del shell SHALL seguir atendiendo estado,
  sesiones y bandeja de permisos
- **AND** al concluir el turno el tablero SHALL reflejar el resultado

#### Scenario: El tablero degrada a ASCII sin perder significado

- **WHEN** el shell corre bajo presentación ASCII o sin color
- **THEN** cada estado de calle SHALL conservar su palabra y su gemelo
  ASCII del glifo
- **AND** ningún significado SHALL depender solo del color
