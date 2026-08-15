# attention-notices

## ADDED Requirements

### Requirement: Las superficies piden atención cuando se espera a una persona

Una superficie SHALL pedir la atención del sistema cuando una petición de
permiso queda esperando una decisión humana, cuando una compuerta del método
queda esperando, y cuando una sesión termina o se interrumpe. La petición SHALL
hacerse en la **transición**, nunca por repintado, y varias transiciones
seguidas de lo mismo SHALL colapsar en una sola petición con su recuento.

La superficie de escritorio SHALL pedir atención **solo cuando no tiene el
foco**: con la ventana al frente, la bandeja y los estados que ya se muestran
son el aviso, y pedir las dos cosas a la vez es pedirla en vano. Al recuperar
el foco, la petición SHALL retirarse en las plataformas que la mantienen.

#### Scenario: Se pide atención cuando un permiso queda esperando

- **WHEN** una petición de permiso queda esperando una decisión humana
- **AND** la ventana no tiene el foco
- **THEN** la superficie SHALL pedir la atención del sistema

#### Scenario: Con la ventana al frente no se pide atención

- **WHILE** la ventana tiene el foco
- **THEN** NO SHALL pedirse atención del sistema

#### Scenario: Una compuerta que espera pide atención

- **WHEN** una compuerta del método queda esperando una decisión
- **AND** la ventana no tiene el foco
- **THEN** la superficie SHALL pedir la atención del sistema

#### Scenario: Una sesión que termina pide atención

- **WHEN** una sesión termina o se interrumpe
- **AND** la ventana no tiene el foco
- **THEN** la superficie SHALL pedir la atención del sistema

#### Scenario: Lo mismo no se pide dos veces

- **WHEN** varias transiciones del mismo motivo ocurren seguidas
- **THEN** SHALL pedirse atención una sola vez, con su recuento

### Requirement: El aviso dice el motivo y nada del trabajo

Lo que la superficie muestre fuera de su ventana —el título que acompaña a la
petición de atención— SHALL decir qué espera y cuánto, y NO SHALL contener el
texto de la instrucción ni el de la respuesta del agente. El gestor de ventanas
del sistema conserva ese texto fuera del registro gobernado, y una instrucción
puede llevar exactamente lo que no debe salir del repositorio.

#### Scenario: El motivo viaja, el contenido no

- **WHEN** la superficie pide atención
- **THEN** el título SHALL decir qué espera y cuánto
- **AND** NO SHALL contener texto de la instrucción ni de la respuesta

### Requirement: La campana del terminal se pide, no se impone

El shell de terminal SHALL poder emitir la campana del emulador ante los mismos
momentos, y SHALL estar desactivada mientras no se active por configuración.

#### Scenario: Sin activar, el terminal no suena

- **WHEN** no se ha activado la campana por configuración
- **THEN** el shell NO SHALL emitirla ante ningún momento

#### Scenario: Activada, el terminal suena en los mismos momentos

- **WHILE** la campana está activada por configuración
- **WHEN** ocurre uno de los momentos que piden atención
- **THEN** el shell SHALL emitirla
