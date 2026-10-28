# remote-access — delta

## ADDED Requirements

### Requirement: Túnel con latido y fallo visible
El helper de túnel SHALL componer el `ssh` del usuario con latido
(`ServerAliveInterval`, `ServerAliveCountMax`) y con `ExitOnForwardFailure`,
en el comando y en el snippet de configuración, diciendo qué hace cada uno.
Con `--exec` SHALL ejecutar exactamente el comando que imprime y, cuando ese
`ssh` termina, SHALL decir con qué estado terminó y qué significa; un túnel
que terminó con error MUST NOT presentarse como éxito.

#### Scenario: El túnel lleva latido y cae si no puede reenviar
- **WHEN** el helper compone el túnel de un daemon macOS/Linux
- **THEN** el comando y el snippet SHALL llevar el latido y `ExitOnForwardFailure`
- **AND** lo que `--exec` ejecuta SHALL ser exactamente el comando impreso

#### Scenario: Un túnel que muere se reporta
- **WHEN** el `ssh` que `--exec` lanzó termina con un estado de error
- **THEN** el binario SHALL terminar con el código de daemon inalcanzable
- **AND** SHALL decir el estado y que el otro extremo ya no alcanza el daemon

### Requirement: El acceso de hoy está documentado con sus límites
La documentación de acceso remoto SHALL cubrir el PC Windows servido por
OpenSSH Server —con `meltemi bridge`, los verbos de la CLI y la TUI por SSH, y
lo que está medido y lo que no—; el teléfono de hoy —los verbos sin TUI, la TUI
por `ssh -t` con su tamaño mínimo, la campana como aviso local— y lo que no hay
(push sin conexión); y cuánto se espera al humano, con cada ajuste, su valor
por omisión y lo que pasa al vencer. La documentación de plataformas MUST NOT
contradecir al helper de túnel.

#### Scenario: El PC Windows servido está documentado con lo no medido
- **WHEN** un usuario consulta cómo servir su PC Windows
- **THEN** SHALL encontrar cómo instalar el servidor SSH y qué orden correr desde el otro extremo
- **AND** SHALL encontrar declarado qué tramos están medidos y cuáles no

#### Scenario: El teléfono de hoy está documentado con sus límites
- **WHEN** un usuario consulta cómo usar Meltemi desde un teléfono
- **THEN** SHALL encontrar los verbos sin TUI, la TUI por `ssh -t` con su tamaño mínimo y la campana
- **AND** SHALL encontrar declarado que no existe push sin conexión

#### Scenario: Cada espera tiene su ajuste y su valor
- **WHEN** un usuario consulta cuánto espera una petición o una sesión
- **THEN** SHALL encontrar cada ajuste de `[permissions]` y `[sessions]` con su valor por omisión

#### Scenario: Plataformas y el helper dicen lo mismo
- **WHEN** un usuario consulta el acceso por SSH en la documentación de plataformas
- **THEN** SHALL encontrar qué extremo corre `-R` y cuál `-L`, y el puente para un daemon en Windows
- **AND** NO SHALL encontrar una recomendación que contradiga al helper

### Requirement: El acceso por túnel está medido o declarado
El repositorio SHALL guardar un smoke del acceso remoto que registre, con fecha
y versiones, cada tramo medido y declare pendiente, con lo que falta para
medirlo, cada tramo que no se pudo medir —GUI y TUI contra un daemon reenviado,
el puente por un `sshd` real en Windows—. La documentación MUST NOT presentar
como verificado lo que el smoke declara pendiente.

#### Scenario: El smoke dice qué se midió y qué no
- **WHEN** se consulta el smoke del acceso remoto
- **THEN** cada tramo SHALL estar medido con fecha y versiones o declarado pendiente con lo que falta

#### Scenario: La documentación no promete lo no medido
- **WHEN** la documentación de acceso remoto habla de GUI y TUI sobre un túnel
- **THEN** SHALL presentarlo como lo esperado y remitir al smoke para lo medido
