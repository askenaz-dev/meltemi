

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
La documentación de acceso remoto SHALL declarar la frontera vigente: todas
las capacidades del daemon operan en remoto únicamente a través de un túnel
vivo del usuario hacia el socket local, y el daemon NO SHALL emitir tráfico de
red jamás. Sin conexión establecida no hay control alguno. Como única
apertura, el usuario MAY configurar un **aviso de espera** opt-in: cuando algo
queda esperando su decisión, un cliente conectado o un proceso del propio
usuario — nunca el daemon — MAY emitir un aviso mínimo a un endpoint que el
usuario opera. El aviso SHALL estar desactivado por defecto, su contenido
exacto SHALL estar especificado públicamente antes de existir (constitución
§9) y SHALL NOT transportar contenido del proyecto ni de la petición: solo el
hecho de que una decisión espera. Toda propuesta de relé en la nube de
terceros, puerto de red del daemon o push fuera de esta puerta MUST rechazarse
salvo enmienda fundacional previa.

#### Scenario: La frontera está documentada como postura
- **WHEN** un usuario consulta la documentación de acceso remoto
- **THEN** SHALL encontrar declarado que el control remoto exige túnel vivo
- **AND** la política del aviso de espera SHALL estar explicada como postura de privacidad, con su porqué

#### Scenario: Aviso de espera desactivado por defecto
- **WHEN** el usuario no ha configurado endpoint alguno de aviso
- **THEN** ningún componente de Meltemi emite nada hacia red alguna

#### Scenario: El aviso mínimo no transporta el proyecto
- **WHEN** el aviso de espera configurado se emite
- **THEN** su contenido se limita al hecho de que una decisión espera
- **AND** SHALL NOT incluir contenido del proyecto, de la petición ni código

#### Scenario: El daemon jamás emite el aviso
- **WHEN** una propuesta de cambio sitúa la emisión del aviso en el daemon
- **THEN** la propuesta se rechaza (constitución §3: el daemon no abre transporte de red)

#### Scenario: Propuesta de push fuera de la puerta se rechaza
- **WHEN** una propuesta de cambio introduce notificaciones por relé de terceros o control sin túnel establecido
- **THEN** la propuesta SHALL rechazarse salvo enmienda fundacional aprobada
