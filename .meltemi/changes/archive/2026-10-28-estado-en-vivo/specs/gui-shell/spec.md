# gui-shell — delta

## ADDED Requirements

### Requirement: Los indicadores de la GUI se alimentan del estado vivo

Todo indicador de la GUI que afirme que una sesión está trabajando, esperando
o terminada —el contador de la barra de estado, las cubetas de la barra
lateral y del árbol, el glifo de la pestaña y el anillo del detalle— SHALL
derivar del estado que el daemon notifica, y NO SHALL depender de haber vuelto
a consultar la lista de sesiones. WHERE la GUI consulte esa lista, SHALL ser
para reconciliarse al conectar, al cambiar de proyecto o tras haberse
rezagado, nunca como fuente de la que un indicador dependa para ser cierto.

Una sesión abierta desde otro cliente SHALL aparecer y avanzar en la GUI con
la misma fidelidad que una abierta en ella.

#### Scenario: El contador de la barra cuenta una sesión de otro cliente

- **WHEN** otro cliente abre una sesión y su agente empieza a trabajar
- **THEN** el contador de la barra de estado de la GUI la cuenta como trabajando sin que el usuario haga nada
- **AND** la cubeta de la barra lateral y el anillo del detalle afirman lo mismo

#### Scenario: El sondeo solo reconcilia

- **WHEN** la GUI reconecta al daemon o cambia de proyecto
- **THEN** consulta la lista de sesiones una vez para reconciliarse
- **AND** entre reconciliaciones los indicadores se mueven solo con las notificaciones
