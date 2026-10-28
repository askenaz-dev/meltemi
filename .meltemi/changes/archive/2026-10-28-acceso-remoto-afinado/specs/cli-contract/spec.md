# cli-contract — delta

## MODIFIED Requirements

### Requirement: Arranque del daemon bajo demanda
Un subcomando respaldado por RPC SHALL reutilizar el arranque bajo demanda del
daemon: si no hay daemon en ejecución en el endpoint propio del usuario, el
binario MUST intentar arrancarlo y conectarse antes de fallar por inalcanzable.
Sobre un endpoint explícito que no es el propio y no responde, el binario MUST
terminar de inmediato con el código de daemon inalcanzable y un remedio que
nombra el túnel y el arranque expreso, sin arrancar nada.

#### Scenario: Arranque bajo demanda al no haber daemon
- **WHEN** se invoca un subcomando respaldado por RPC y no hay daemon en ejecución en el endpoint propio
- **THEN** el binario SHALL intentar arrancar el daemon y conectarse
- **AND** SHALL proceder con el método solicitado si la conexión se establece

#### Scenario: Fallo de arranque se reporta como inalcanzable
- **IF** el daemon no puede arrancarse ni alcanzarse dentro del presupuesto de conexión
- **THEN** el binario SHALL terminar con el código de daemon inalcanzable

#### Scenario: Un endpoint explícito caído se reporta sin arrancar nada
- **WHEN** se invoca un subcomando respaldado por RPC con un endpoint explícito que no es el propio y nada responde en él
- **THEN** el binario SHALL terminar de inmediato con el código de daemon inalcanzable
- **AND** el mensaje SHALL nombrar el túnel y cómo pedir el arranque en ese endpoint
