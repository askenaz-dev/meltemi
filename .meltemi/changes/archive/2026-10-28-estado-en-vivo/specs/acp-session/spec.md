# acp-session — delta

## ADDED Requirements

### Requirement: El estado de cada sesión se notifica a todo cliente conectado

`meltemid` SHALL notificar a **toda** conexión iniciada cada transición de
estado de **cualquier** sesión, sin que el cliente la pida y sin importar qué
conexión abrió esa sesión. La notificación SHALL afirmar el estado completo
—identificador de sesión, raíz del proyecto, estado, el instante desde el que
la sesión está en ese estado, y cuántos permisos suyos esperan decisión— y NO
SHALL expresar una diferencia respecto de un estado anterior ni llevar número
de secuencia: aplicarla dos veces, o aplicar dos de ellas en cualquier orden,
SHALL converger al último estado afirmado.

Al iniciar, una conexión SHALL recibir el estado actual de cada sesión que el
daemon conoce **antes** de recibir cualquier transición posterior, de modo que
una conexión que llega tarde no dependa de haber visto la transición que la
precedió.

La notificación NO SHALL llevar contenido del proyecto, de la instrucción ni
del turno: lo que un cliente no podía leer antes SHALL seguir sin poder
leerlo.

WHERE una conexión no alcance a leer las notificaciones al ritmo en que se
emiten y alguna se pierda, el daemon SHALL volver a afirmar el estado de cada
sesión conocida a esa conexión, de modo que un hueco se repare sin que el
cliente tenga que advertirlo ni pedir nada. El daemon NO SHALL dejar el hueco
registrado solo de su lado.

#### Scenario: Una transición llega a un cliente que no inició la sesión

- **WHEN** un cliente abre una sesión y otro cliente distinto está conectado al mismo daemon
- **THEN** el segundo cliente recibe la notificación de cada transición de esa sesión sin haberla pedido
- **AND** el estado que recibe es el mismo que devolvería la lista de sesiones en ese instante

#### Scenario: Una conexión nueva recibe el estado actual antes de cualquier transición

- **WHEN** una conexión se inicia mientras ya hay sesiones en curso
- **THEN** recibe una afirmación del estado de cada sesión conocida antes de cualquier transición posterior
- **AND** una transición ocurrida durante ese envío llega después y corrige lo afirmado, sin perderse

#### Scenario: La notificación afirma el estado entero y es idempotente

- **WHEN** una conexión aplica dos veces la misma notificación, o aplica dos notificaciones de una sesión en orden invertido
- **THEN** el estado con el que queda es el de la última notificación afirmada
- **AND** ninguna de las dos lecturas exige un número de secuencia ni un estado previo

#### Scenario: Un rezago se repara reafirmando el estado, no con un hueco

- **WHEN** una conexión pierde notificaciones por no leerlas al ritmo en que se emiten
- **THEN** el daemon vuelve a afirmar el estado de cada sesión conocida a esa conexión
- **AND** el cliente queda con estado válido sin haber pedido nada ni haber advertido el hueco

#### Scenario: La notificación no lleva contenido del proyecto

- **WHEN** se examina el contenido de una notificación de estado
- **THEN** lleva identificador, raíz, estado, instante y conteo de permisos en espera, y nada más
- **AND** no lleva instrucción, ni título, ni texto del turno
