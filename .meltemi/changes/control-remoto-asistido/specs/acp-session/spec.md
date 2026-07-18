## ADDED Requirements

### Requirement: Dirección de una sesión existente
El daemon SHALL aceptar instrucciones dirigidas a una sesión existente
(`session/direct`): sobre una sesión activa la instrucción SHALL encolarse y
despacharse como el siguiente turno de la misma sesión del agente al concluir el
turno en curso, sin interrumpirlo; sobre una sesión terminada y reanudable SHALL
reanudarse con la instrucción como prompt; sobre una sesión inexistente o no
reanudable MUST rehusarse con diagnóstico y remedio. Cada instrucción MUST
registrarse en el log de sesión al encolarse y al despacharse, y el verbo SHALL
ser consumible desde todas las superficies por igual.

#### Scenario: Instrucción a una sesión activa se despacha como siguiente turno
- **WHEN** un cliente dirige una instrucción a una sesión activa
- **THEN** la instrucción SHALL encolarse sin interrumpir el turno en curso
- **AND** al concluir ese turno SHALL despacharse como el siguiente prompt de la misma sesión del agente
- **AND** el encolado y el despacho SHALL constar en el JSONL

#### Scenario: Instrucción a una sesión reanudable la reanuda
- **WHEN** un cliente dirige una instrucción a una sesión terminada cuyo agente anunció capacidad de reanudación
- **THEN** el daemon SHALL reanudar esa sesión con la instrucción como prompt
- **AND** la sesión nueva SHALL quedar enlazada a la original como reanudación

#### Scenario: Sesión no dirigible rehúsa con remedio
- **IF** la sesión no existe o no es reanudable
- **THEN** la dirección SHALL rehusarse con diagnóstico
- **AND** el remedio SHALL orientar a listar las sesiones disponibles

#### Scenario: Dirigir no interrumpe ni cancela
- **WHILE** una sesión ejecuta su turno
- **WHEN** llegan instrucciones dirigidas
- **THEN** el turno en curso SHALL continuar intacto
- **AND** la cancelación SHALL seguir siendo un verbo distinto y explícito
