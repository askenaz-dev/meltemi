

### Requirement: Helper de túnel auditable
El cliente SHALL ofrecer un helper de túnel (`tunnel`) que compone la invocación
del `ssh` del propio usuario para llevar el endpoint local del daemon al otro
extremo: SHALL imprimir el comando exacto por plataforma, el snippet de
configuración SSH equivalente y el valor de endpoint que el extremo remoto debe
usar, y MAY ejecutar ese `ssh` de forma visible cuando el usuario lo pida
explícitamente. El helper MUST NOT abrir sockets propios, MUST NOT introducir
transporte de red en el daemon y MUST NOT generar, leer ni almacenar material de
claves del usuario.

#### Scenario: Composición del túnel por plataforma
- **WHEN** el usuario invoca el helper de túnel hacia un host remoto
- **THEN** el helper SHALL imprimir el comando `ssh` exacto de reenvío del endpoint local de esta plataforma
- **AND** SHALL incluir el snippet de configuración y el endpoint que el extremo remoto debe usar

#### Scenario: Ejecución visible solo a pedido
- **WHEN** el usuario pide ejecutar el túnel
- **THEN** el helper SHALL lanzar el `ssh` del usuario como proceso visible
- **AND** SHALL NOT dejar túneles en segundo plano sin que el usuario lo haya pedido

#### Scenario: Plataforma sin reenvío posible rehúsa honesta
- **IF** la plataforma del daemon no admite el reenvío estándar de su endpoint
- **THEN** el helper SHALL rehusarse con diagnóstico y remedio
- **AND** SHALL NOT imprimir un comando que no puede funcionar

### Requirement: Frontera honesta del acceso remoto
La documentación de acceso remoto SHALL declarar la frontera vigente: todas las
capacidades del daemon operan en remoto únicamente a través de un túnel vivo del
usuario hacia el socket local, y no existe notificación ni control alguno sin
conexión establecida; toda propuesta de relé en la nube, puerto de red o push
sin túnel MUST rechazarse salvo enmienda fundacional previa.

#### Scenario: La frontera está documentada como postura
- **WHEN** un usuario consulta la documentación de acceso remoto
- **THEN** SHALL encontrar declarado que el control remoto exige túnel vivo
- **AND** la ausencia de push sin conexión SHALL estar explicada como postura de privacidad, con su porqué

#### Scenario: Propuesta de push sin túnel se rechaza
- **WHEN** una propuesta de cambio introduce notificaciones o control sin túnel establecido
- **THEN** la propuesta SHALL rechazarse salvo enmienda fundacional aprobada
