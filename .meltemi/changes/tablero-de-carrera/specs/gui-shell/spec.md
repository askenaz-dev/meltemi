# gui-shell — delta

## ADDED Requirements

### Requirement: Tablero de carrera

La superficie de escritorio SHALL presentar, por cambio y tarea, el tablero
de la carrera: las calles de los competidores lado a lado, cada una con su
procedencia visible (agente y perfil/suscripción cuando aplique), su diff
contra la base, su estado de turno, commit y checkpoint declarado con
señal más palabra, y las acciones de la carrera (despachar un turno,
revertir al checkpoint, commit de la tarea, merge asistido por archivo).
Toda acción destructiva MUST exigir confirmación explícita antes de
ejecutarse. El tablero SHALL reflejar únicamente estado persistido o
derivado del daemon — nunca estado inventado: una calle sin procedencia
registrada se muestra sin procedencia. Al concluir un turno despachado
desde la propia superficie, el tablero SHALL actualizarse sin recargar la
aplicación.

#### Scenario: Calles lado a lado con procedencia visible

- **WHEN** el usuario abre el tablero de una tarea con competidores
- **THEN** cada calle SHALL mostrar su agente, su perfil cuando lo hubo,
  su estado y su diff contra la base
- **AND** una calle sin procedencia registrada SHALL mostrarse sin
  procedencia, con ausencia visible

#### Scenario: Acción destructiva solo con confirmación explícita

- **WHEN** el usuario invoca revertir una calle desde el tablero
- **THEN** la superficie SHALL exigir confirmación explícita antes de
  enviar la operación
- **AND** cancelar la confirmación MUST NOT enviar nada al daemon

#### Scenario: El tablero refleja el turno concluido

- **WHILE** un despacho lanzado desde la propia superficie corre su turno
- **WHEN** el turno concluye
- **THEN** el tablero SHALL actualizar la calle afectada sin recargar la
  aplicación

#### Scenario: Carrera sin competidores, estado vacío honesto

- **IF** la tarea no tiene worktrees de competidores
- **THEN** el tablero SHALL mostrar un estado vacío que lo diga
- **AND** SHALL ofrecer el camino para asignar la carrera, no un tablero en
  blanco
