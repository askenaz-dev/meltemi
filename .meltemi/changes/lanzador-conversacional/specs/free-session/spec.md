# free-session — delta

## ADDED Requirements

### Requirement: Arranque de sesión libre sin change ni gate de spec
El daemon SHALL exponer un verbo de arranque de sesión libre (`session/start`)
que reciba la raíz del proyecto y una instrucción y arranque una sesión de
agente **sin** exigir change, tarea, especificación ni gate del método. La
sesión libre MUST correr con el mismo gobierno que cualquier otra sesión: agente
resuelto desde la flota con la resolución registrada en el log, proxy de
permisos con deny-by-default, registro JSONL apend-only, alta del proyecto en el
registro y publicación al stream de sesión. El daemon MUST NOT relajar ninguna
de esas piezas por tratarse de una sesión sin especificación, y MUST NOT
introducir un camino de permisos distinto del proxy vigente.

#### Scenario: Sesión libre corre gobernada sin change
- **WHEN** un cliente arranca una sesión libre sobre una raíz de proyecto con una instrucción
- **THEN** el daemon SHALL lanzar el agente resuelto sin exigir change, tarea ni gate
- **AND** el binario efectivo y la fuente de la resolución SHALL constar en el log de sesión
- **AND** toda petición de permiso del agente SHALL pasar por el proxy vigente

#### Scenario: Sin cliente conectado la sesión libre no gana privilegios
- **WHILE** una sesión libre está en curso y ninguna conexión la atiende
- **WHEN** el agente pide un permiso que las reglas no resuelven
- **THEN** la petición SHALL denegarse como en cualquier otra sesión
- **AND** la denegación SHALL constar en el registro de la sesión

#### Scenario: El proyecto de una sesión libre queda registrado
- **WHEN** se arranca una sesión libre sobre una raíz que el registro no conocía
- **THEN** el proyecto SHALL quedar dado de alta en el registro con esa raíz
- **AND** SHALL NOT requerir ninguna acción adicional del usuario

### Requirement: La sesión libre es dirigible y cierra por el finalizador compartido
Una sesión libre SHALL quedar habilitada para recibir instrucciones dirigidas
mientras vive, de modo que la instrucción siguiente se despache como el próximo
turno de la misma sesión del agente, y SHALL cerrar por el finalizador
compartido de turnos: eventos terminales en el log, registro de fin en el índice
con los metadatos de reanudación, y baja del registro vivo. Una sesión libre
completada MUST NOT quedar listada como interrumpida.

#### Scenario: Instrucción de seguimiento se despacha como siguiente turno
- **WHEN** el usuario envía una segunda instrucción a una sesión libre en curso
- **THEN** la instrucción SHALL encolarse sin interrumpir el turno vigente
- **AND** al concluir ese turno SHALL despacharse como el siguiente prompt de la misma sesión

#### Scenario: Sesión libre completada no lista como interrumpida
- **WHEN** una sesión libre concluye su último turno
- **THEN** el índice de sesiones SHALL recibir su registro de fin
- **AND** el listado histórico SHALL mostrarla terminada, nunca interrumpida

### Requirement: Dónde opera la sesión libre, declarado y no implícito
La sesión libre SHALL operar sobre la raíz del proyecto elegido, igual que los
demás caminos atendidos por un humano, y MUST NOT crear worktree alguno ni
inventar una tripleta de change y tarea que contamine el modelo de competidores.
A cambio del aislamiento, el daemon SHALL crear un punto de restauración al
arrancar la sesión —una instantánea que no mueve ninguna rama del usuario ni
altera su índice— y MUST declarar en el resultado y en el log si ese punto
existe. WHERE la raíz no sea un repositorio git **o no tenga todavía historia
que fotografiar**, la sesión libre SHALL arrancar igualmente y SHALL declarar
que no hay punto de restauración, con el remedio que corresponda a la causa; el
daemon MUST NOT simular un punto de restauración inexistente, MUST NOT ofrecer
un remedio que no aplique al caso, ni rehusar el arranque por esa causa.

El punto de restauración de una sesión libre es un ref de git y una entrada
listable, **no una reversión ofrecida**: como su árbol es el del usuario y
contiene trabajo humano sin commitear, el verbo de reversión de puntos de
restauración MUST rehusar el de una sesión libre con diagnóstico y con el
remedio de restaurar desde git, y MUST NOT reponer ese árbol ni borrar en él
archivos no rastreados. Ninguna superficie SHALL ofrecer un control de
reversión para el punto de restauración de una sesión libre.

#### Scenario: Punto de restauración creado al arrancar
- **WHEN** se arranca una sesión libre sobre una raíz que es repositorio git con historia
- **THEN** el daemon SHALL crear el punto de restauración antes del primer turno
- **AND** el resultado SHALL declarar su referencia
- **AND** ninguna rama del usuario SHALL moverse por ello

#### Scenario: Raíz sin git arranca y lo declara
- **WHERE** la raíz elegida no es un repositorio git
- **WHEN** se arranca una sesión libre sobre ella
- **THEN** la sesión SHALL arrancar
- **AND** el resultado SHALL declarar que no hay punto de restauración, con su remedio

#### Scenario: Repositorio sin historia arranca y da el remedio que corresponde
- **WHERE** la raíz es un repositorio git que todavía no tiene ningún commit
- **WHEN** se arranca una sesión libre sobre ella
- **THEN** la sesión SHALL arrancar
- **AND** el resultado SHALL declarar que no hay punto de restauración
- **AND** el remedio SHALL apuntar al primer commit, nunca a inicializar un repositorio que ya existe

#### Scenario: El punto de restauración de una sesión libre no es revertible
- **IF** se pide revertir el punto de restauración de una sesión libre
- **THEN** el daemon SHALL rehusar con diagnóstico y con el remedio de restaurar desde git
- **AND** el árbol del usuario SHALL quedar intacto, incluidos sus archivos no rastreados
- **AND** ninguna superficie SHALL haber ofrecido ese control

#### Scenario: La sesión libre no crea worktrees ni competidores
- **WHEN** una sesión libre corre sobre un proyecto con worktrees gestionados
- **THEN** el listado de worktrees SHALL permanecer sin entradas nuevas
- **AND** ningún competidor SHALL aparecer para una tarea que nadie asignó

### Requirement: Elección de agente en el arranque libre
El verbo de arranque libre SHALL admitir un parámetro opcional de agente,
resuelto por el orden vigente de la flota —perfil de lanzamiento, id del
catálogo y, en su defecto, el agente configurado del proyecto—, y la resolución
efectiva MUST registrarse en el log de la sesión para que una reconstrucción
desde el log recupere qué agente corrió. Un agente nombrado que resuelve a un
binario no detectado MUST rehusar con diagnóstico y remedio, y MUST NOT degradar
en silencio a otro proveedor. Omitir el parámetro SHALL comportarse exactamente
como hoy: el agente configurado del proyecto.

#### Scenario: Sesión libre con agente nombrado
- **WHEN** se arranca una sesión libre nombrando un id del catálogo detectado
- **THEN** el daemon SHALL lanzar el binario de ese id
- **AND** la resolución con su fuente SHALL constar en el log de sesión

#### Scenario: Sesión libre con agente no detectado rehúsa sin degradar
- **IF** el agente nombrado resuelve a un binario no detectado
- **THEN** el arranque SHALL rehusarse con diagnóstico y remedio
- **AND** ningún otro proveedor SHALL lanzarse en su lugar

#### Scenario: Sin parámetro se usa el agente configurado
- **WHEN** se arranca una sesión libre sin nombrar agente
- **THEN** SHALL usarse el agente configurado del proyecto
- **AND** la resolución SHALL constar igualmente en el log

### Requirement: Identidad de la sesión conocida antes del primer token
El cliente que arranca una sesión libre SHALL conocer su identificador antes de
que llegue la primera salida del agente, de modo que la superficie pueda navegar
hacia adentro de la conversación sin esperar al final del turno. El resultado
del método SHALL seguir siendo final —estado del turno y conteo de denegaciones—
para los clientes scriptables que no escuchan notificaciones, y el daemon MUST
NOT exigir suscripción alguna al iniciador para hacerle llegar el arranque de su
propia sesión.

#### Scenario: El iniciador recibe el arranque sin pedirlo
- **WHEN** un cliente arranca una sesión libre
- **THEN** SHALL recibir el evento de arranque con el identificador de la sesión antes de la primera salida del agente
- **AND** SHALL NOT haber tenido que declarar interés en esa sesión

#### Scenario: Resultado final honesto para el cliente scriptable
- **WHEN** un cliente scriptable arranca una sesión libre y no escucha notificaciones
- **THEN** el resultado del método SHALL traer el identificador, el estado final del turno y el conteo de denegaciones

### Requirement: El arranque libre tiene casa en las tres superficies
El verbo de arranque de sesión libre SHALL ser invocable desde la CLI, desde la
paleta de la TUI y desde el registro tipado de la GUI por igual (constitución
§4), MUST constar en la matriz de paridad, y el parámetro de agente SHALL estar
disponible en las tres. Ninguna superficie SHALL quedar con una nota al pie que
explique por qué allí no está.

#### Scenario: El arranque libre tiene casa en las tres superficies
- **WHEN** el contrato incorpora el verbo de arranque de sesión libre
- **THEN** el subcomando de la CLI, la paleta de la TUI y el registro de la GUI SHALL ofrecerlo
- **AND** la matriz de paridad SHALL incluir su fila

#### Scenario: La elección de agente es pareja
- **WHEN** una superficie ofrece arrancar una sesión libre
- **THEN** SHALL permitir nombrar agente o perfil de la flota
- **AND** SHALL presentar los perfiles por su nombre de suscripción junto a su agente subyacente
