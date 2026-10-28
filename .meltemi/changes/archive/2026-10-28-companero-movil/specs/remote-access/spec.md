# remote-access — delta

## ADDED Requirements

### Requirement: El compañero móvil usa el mismo puente que cualquier cliente

El canal del compañero móvil SHALL ser el mismo que el de cualquier cliente
remoto —SSH del usuario hacia el comando puente que bombea el socket local— y
NO SHALL exigir del daemon ninguna capacidad, puerto ni formato propio. La
diferencia con el escritorio SHALL ser únicamente quién ejecuta el SSH: en el
teléfono lo habla la app, porque no hay un `ssh` del sistema que lanzar.

#### Scenario: El puente es el mismo

- **WHEN** el compañero móvil conecta
- **THEN** SHALL hablar el mismo protocolo por el mismo comando puente que un
  cliente de escritorio con un endpoint de comando
- **AND** el daemon NO SHALL ganar nada para servirlo
