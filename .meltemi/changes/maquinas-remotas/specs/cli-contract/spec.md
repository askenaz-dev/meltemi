# cli-contract — delta

## ADDED Requirements

### Requirement: Verbo de máquinas

El CLI SHALL ofrecer `machines`, que imprime las máquinas configuradas con su
endpoint y cuál es la activa, y SHALL honrar `--json` y `--yaml` como el resto
de los listados. El CLI NO SHALL ofrecer cambiar la máquina activa: cada
invocación resuelve su endpoint y termina, de modo que no hay a quién
cambiársela, y el verbo SHALL decir por dónde se cambia.

#### Scenario: machines imprime la lista y cuál está activa

- **WHEN** se ejecuta `machines` con máquinas configuradas
- **THEN** SHALL imprimir cada una con su nombre y su endpoint, marcando la
  activa
- **AND** con `--json` SHALL imprimir la lista como listado estructurado

#### Scenario: Sin máquinas configuradas, dice cómo se configuran

- **WHERE** no hay máquinas en la configuración de usuario
- **THEN** SHALL decirlo sin error y nombrar el archivo donde se escriben
