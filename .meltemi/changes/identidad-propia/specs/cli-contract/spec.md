# cli-contract — delta

## ADDED Requirements

### Requirement: Verbos de identidad

El CLI SHALL ofrecer `identity`, que consulta la identidad y los equipos por
el método de identidad del contrato y los imprime como listado legible, y
`identity-link [--hostname <nombre>] [--exec]`, que imprime el gesto de
vínculo compuesto por el daemon y, solo con `--exec`, ejecuta el binario del
propio usuario con la entrada y la salida heredadas. Ambos SHALL honrar
`--json` y `--yaml` como el resto de los listados. Sin `--exec`,
`identity-link` NO SHALL lanzar ningún proceso del cliente de malla (el
arranque del daemon bajo demanda sigue rigiendo como en todo verbo respaldado
por RPC).

#### Scenario: identity imprime quién y qué equipos

- **WHEN** se ejecuta `identity` con la malla enlazada
- **THEN** la salida SHALL decir el nombre de login, el proveedor, este equipo
  y cada equipo con su presencia en palabra
- **AND** con `--json` SHALL imprimir la respuesta del contrato tal cual

#### Scenario: identity-link imprime el gesto y solo ejecuta a pedido

- **WHEN** se ejecuta `identity-link` sin `--exec`
- **THEN** SHALL imprimir el gesto en la forma de la plataforma y la pista de
  que abrirá el navegador
- **AND** NO SHALL lanzar ningún proceso del cliente de malla; con `--exec`
  SHALL correr el binario del usuario con entrada y salida heredadas
