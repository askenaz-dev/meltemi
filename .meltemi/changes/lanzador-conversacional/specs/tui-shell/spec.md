# tui-shell — delta

## MODIFIED Requirements

### Requirement: Paridad de núcleo por la paleta de comandos
La paleta `:` SHALL exponer toda capacidad del daemon alcanzable por tecleo, incluso
antes de que exista una vista dedicada. Todo método RPC nuevo del daemon MUST
registrarse en el autocompletado de la paleta para que ninguna capacidad quede sin
casa. **El verbo de dirección de sesión MUST dejar de anunciarse como reservado y
MUST quedar cableado**: anunciarlo y no atenderlo es una promesa incumplida del
shell, y activarlo MUST NOT cerrar el overlay sin efecto ni diagnóstico. WHERE un
verbo requiera texto libre —una instrucción, una ruta—, la paleta SHALL ofrecer
una entrada que **preserve el texto tal cual se escribe**, y MUST NOT normalizar
mayúsculas ni separadores en el valor que se envía al daemon; el discriminador de
un verbo con argumento libre MUST leerse antes que cualquier atajo de filtro que
comparta su prefijo, para que la paleta no confunda una orden con un filtro.

#### Scenario: Capacidad sin vista dedicada alcanzable
- **WHEN** el usuario abre la paleta y filtra por una capacidad del daemon
- **THEN** el shell SHALL ofrecer invocarla aunque no tenga tecla o vista dedicada

#### Scenario: Registro obligatorio de método nuevo
- **WHEN** el daemon gana un método RPC nuevo
- **THEN** el shell SHALL registrarlo en el autocompletado de la paleta

#### Scenario: El verbo de dirección deja de estar reservado y queda cableado
- **WHEN** el usuario activa el verbo de dirección de sesión en la paleta
- **THEN** el shell SHALL presentarlo como operativo y abrir su entrada de instrucción
- **AND** activarlo SHALL producir efecto o diagnóstico, nunca un cierre silencioso del overlay

#### Scenario: El texto libre llega intacto al daemon
- **WHEN** el usuario introduce una instrucción o una ruta con mayúsculas por la paleta
- **THEN** el valor enviado al daemon SHALL ser idéntico al tecleado
- **AND** SHALL NOT haberse normalizado a minúsculas por el camino

#### Scenario: El discriminador de un verbo no se confunde con un filtro
- **WHEN** el usuario teclea el verbo del registro de proyectos con su discriminador de alta y una ruta
- **THEN** el shell SHALL invocar el método de alta con esa ruta
- **AND** SHALL NOT interpretar la línea como un filtro de ámbito de proyecto

### Requirement: Sesiones agrupadas por proyecto con ámbito conmutable
La vista Sesiones SHALL agrupar las sesiones por proyecto con encabezado de grupo,
y cada fila SHALL mostrar su agente y el nombre de su suscripción cuando la
sesión resolvió por perfil; el filtro `/` vigente SHALL admitir además reducir por
proyecto, y el ámbito de proyecto SHALL ser conmutable desde la paleta sin salir
del shell, con el cwd como ámbito inicial. La vista de proyectos SHALL renderizar
el registro con su raíz, su presencia en disco y sus contadores, y SHALL ofrecer
dar de alta y dar de baja un proyecto tecleando su ruta —la superficie de terminal
no dispone de diálogo nativo y MUST NOT quedar por ello sin acceso a esos métodos
(constitución §4)—. La agrupación MUST honrar la línea base de accesibilidad
(glifo o forma más palabra, gemelo ASCII, `NO_COLOR`) y el degradado de columnas
vigente, sin ocultar datos en silencio.

#### Scenario: Sesiones agrupadas por proyecto
- **WHEN** el usuario abre la vista Sesiones con sesiones en dos proyectos
- **THEN** la tabla SHALL presentarlas bajo un encabezado por proyecto
- **AND** cada fila SHALL indicar su agente y su suscripción

#### Scenario: Filtro por proyecto reduce a un grupo
- **WHEN** el usuario pulsa `/` y teclea parte de la raíz de un proyecto
- **THEN** la vista SHALL reducirse a las sesiones de ese proyecto

#### Scenario: Suscripción legible sin color ni Unicode
- **WHERE** están activos `NO_COLOR` o el modo ASCII
- **THEN** el proyecto y la suscripción de cada sesión SHALL seguir legibles como texto
- **AND** ninguna distinción SHALL depender del color

#### Scenario: Ámbito de proyecto conmutado desde la paleta
- **WHEN** el usuario conmuta el proyecto de ámbito desde la paleta
- **THEN** las consultas con ámbito de proyecto SHALL usar esa raíz
- **AND** el chrome SHALL reflejar el ámbito vigente

#### Scenario: Alta y baja de proyecto tecleando la ruta
- **WHEN** el usuario da de alta un proyecto desde la TUI escribiendo su ruta
- **THEN** el shell SHALL invocar el método de alta del registro con esa ruta
- **AND** el proyecto SHALL aparecer en la vista de proyectos sin reiniciar el shell

#### Scenario: Baja desde la TUI no toca el disco
- **WHEN** el usuario da de baja un proyecto desde la TUI
- **THEN** el shell SHALL invocar el método de baja del registro
- **AND** SHALL declarar que el olvido rige sobre el listado y no borra nada
