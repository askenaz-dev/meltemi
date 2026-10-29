# cli-contract — delta

## ADDED Requirements

### Requirement: El cliente dice cómo se instaló y cómo actualizarse
`meltemi version --verbose` SHALL imprimir, además de la versión, el canal por
el que se instaló el cliente y el comando que lo actualiza por ese canal,
deducidos de la ruta de su propio ejecutable y sin red; con `--json`, los tres
SHALL ser campos. IF el canal no se reconoce, THEN SHALL decirlo y remitir a la
página de releases, y MUST NOT inventar un comando. `meltemi version` sin la
bandera SHALL seguir imprimiendo solo la versión.

#### Scenario: Instalado por un gestor
- **WHEN** el ejecutable vive en la ruta de un gestor reconocido y se pide `meltemi version --verbose`
- **THEN** SHALL imprimir la versión, el gestor y su comando de actualización

#### Scenario: Canal desconocido
- **IF** la ruta del ejecutable no corresponde a ningún canal reconocido
- **THEN** SHALL decir que el canal es desconocido y remitir a la página de releases
- **AND** SHALL NOT imprimir un comando de actualización

#### Scenario: La salida corta no cambia
- **WHEN** se pide `meltemi version` sin `--verbose`
- **THEN** SHALL imprimir solo la versión, como antes
