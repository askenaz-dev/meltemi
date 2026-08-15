# remote-access — delta

## ADDED Requirements

### Requirement: Puente stdio del último metro

El cliente SHALL ofrecer un verbo de puente (`bridge`) que conecta el endpoint
local del daemon de esta máquina con su propia entrada y salida estándar, en
ambas direcciones, hasta que un extremo cierre. El puente SHALL operar sin
terminal interactiva, MUST NOT abrir sockets propios, MUST NOT introducir
transporte de red en el daemon y MUST NOT generar, leer ni almacenar material
de claves. WHERE el daemon no esté accesible, el puente SHALL rehusarse de
inmediato con diagnóstico y remedio, y NO SHALL quedarse esperando. El puente
SHALL funcionar en las tres plataformas, incluido el endpoint de named pipe
que el reenvío estándar no alcanza.

#### Scenario: Un canal remoto completo sobre el puente

- **WHEN** un proceso habla JSON-RPC por el stdio del puente
- **THEN** cada petición SHALL llegar al daemon local y cada respuesta SHALL
  volver por el mismo stdio
- **AND** el daemon NO SHALL distinguir esa conexión de una local

#### Scenario: El puente en la plataforma sin reenvío estándar

- **WHEN** el puente corre en la plataforma cuyo endpoint es un named pipe
- **THEN** SHALL conectar con ese endpoint igual que en las demás plataformas
- **AND** el remedio del rehúso del helper de túnel SHALL nombrar el puente
  como el camino que sí funciona

#### Scenario: Sin daemon, el puente rehúsa sin colgarse

- **IF** el endpoint local no está accesible
- **THEN** el puente SHALL terminar de inmediato con diagnóstico y remedio
- **AND** NO SHALL quedarse esperando a que aparezca

#### Scenario: El cierre de un extremo cierra el puente

- **WHEN** la entrada estándar del puente se cierra, o el endpoint cierra
- **THEN** el puente SHALL terminar ordenadamente
- **AND** NO SHALL dejar procesos ni sockets residuales

### Requirement: El punto de encuentro en dos vías está documentado

La documentación de acceso remoto SHALL declarar el patrón del punto de
encuentro: ambos extremos marcan conexiones salientes hacia infraestructura
del propio usuario, de modo que el acceso funciona con cualquiera de los dos
extremos fuera de su red habitual. SHALL documentar la variante de bastión SSH
(con el túnel inverso permanente y sus precauciones de cuenta dedicada) y la
variante de red privada del usuario (malla con plano de control
autohospedado), incluyendo el estado de licencias de las piezas nombradas. La
documentación SHALL declarar la frontera: esa infraestructura es del usuario —
Meltemi MUST NOT empaquetarla, MUST NOT depender de ella para compilar o
testear, y el transporte final hacia el daemon sigue siendo el SSH del propio
usuario hacia el socket local. Las decisiones que pertenecen a la superficie
móvil de fase 3 (identidad del usuario, selector de máquinas, aviso de espera)
SHALL quedar anotadas como notas de design para esa change, no como
capacidades presentes.

#### Scenario: Los cuatro cuadrantes usan el mismo camino

- **WHEN** un usuario consulta la documentación del patrón
- **THEN** SHALL encontrar que el PC y el cliente remoto marcan hacia afuera
- **AND** que el camino es el mismo con cualquiera de los dos fuera de casa

#### Scenario: La malla del usuario no es una dependencia de Meltemi

- **WHEN** una propuesta de cambio añade al workspace una dependencia de la
  infraestructura de encuentro (malla, bastión, plano de control, IdP)
- **THEN** la propuesta SHALL rechazarse: esa infraestructura es del usuario
- **AND** compilar y testear Meltemi SHALL seguir sin exigir cuenta ni red

#### Scenario: Lo de fase 3 está anotado y no prometido

- **WHEN** un usuario lee la documentación del patrón
- **THEN** las piezas de fase 3 SHALL estar marcadas como notas de design
- **AND** NO SHALL presentarse como capacidades existentes
