# acp-session — delta

## ADDED Requirements

### Requirement: Sesión que espera la siguiente instrucción

Al concluir un turno sin instrucciones pendientes, el daemon SHALL mantener la
sesión viva y en espera —con su conexión ACP y su subproceso intactos— en vez de
terminarla, y SHALL declararla en estado `waiting_instruction` en `status` y en
el listado de sesiones. Una instrucción dirigida a una sesión en espera SHALL
despacharse como su siguiente turno sin reanudar ni relanzar nada.

La espera SHALL ser por señal, sin sondeo. Encolar una instrucción SHALL mutar la
cola y despertar la espera **bajo el mismo lock y en ese orden**, de modo que no
exista ventana en la que la espera se registre después de que la instrucción
llegó.

Esperar NO SHALL contar como trabajar: la sesión en espera SHALL distinguirse de
una sesión que ejecuta un turno en toda superficie que declare estado.

#### Scenario: La sesión sobrevive al turno y espera

- **WHEN** un turno concluye y no hay instrucciones pendientes
- **THEN** la sesión SHALL permanecer viva en `waiting_instruction`
- **AND** su subproceso de agente SHALL seguir vivo
- **AND** NO SHALL emitirse el fin de sesión

#### Scenario: La instrucción despierta la espera sin reanudar

- **WHEN** se dirige una instrucción a una sesión en espera
- **THEN** SHALL despacharse como el siguiente turno de la misma sesión del
  agente
- **AND** NO SHALL crearse una sesión nueva ni relanzarse el subproceso

#### Scenario: Encolar y despertar no dejan ventana

- **WHEN** una instrucción llega justo mientras el borde del turno entra en
  espera
- **THEN** SHALL despacharse igualmente
- **AND** la sesión NO SHALL quedar dormida con trabajo en la cola

#### Scenario: La sesión en espera se cancela como cualquier otra

- **WHEN** se cancela una sesión que espera instrucciones
- **THEN** SHALL terminar y su subproceso SHALL terminar con ella
- **AND** el apagado del daemon SHALL alcanzarla igual

### Requirement: Cotas explícitas de la espera ociosa

La espera ociosa SHALL estar gobernada por cotas configurables con defaults
conservadores: un tiempo máximo de espera sin instrucción y un número máximo de
sesiones esperando a la vez. Alcanzado el tope de sesiones, la espera más
antigua SHALL cerrarse y SHALL decirse; el arranque de sesiones nuevas NO SHALL
rehusarse por esta causa.

Una sesión que espera sin ningún cliente conectado de forma sostenida SHALL
terminar: no hay a quién esperar.

Al vencer cualquiera de las cotas, la sesión SHALL finalizar honestamente con un
motivo que diga que venció la espera, y NUNCA como completada. Desde ahí SHALL
aplicar la reanudación de siempre.

#### Scenario: La espera vencida termina con su motivo

- **WHEN** una sesión espera más que el tiempo configurado
- **THEN** SHALL finalizar con motivo de espera vencida
- **AND** NO SHALL registrarse como completada
- **AND** SHALL quedar reanudable como cualquier sesión terminada

#### Scenario: El tope de esperas cierra la más antigua

- **WHEN** una sesión entra en espera con el tope ya alcanzado
- **THEN** la espera más antigua SHALL cerrarse con su motivo
- **AND** la sesión nueva NO SHALL rehusarse

#### Scenario: Sin clientes sostenidamente, la espera termina

- **WHILE** una sesión espera instrucciones
- **WHEN** no queda ningún cliente conectado durante la gracia configurada
- **THEN** la sesión SHALL terminar con su motivo

### Requirement: Arranque desacoplado de la vida de la sesión

`session/start` SHALL admitir un parámetro aditivo que pida responder en cuanto
la sesión exista, con su identificador, dejando el desenlace al stream de
eventos. Omitido, el verbo SHALL comportarse exactamente como hoy: responder al
terminar la sesión, con el mismo resultado.

#### Scenario: Arranque desacoplado responde con el identificador

- **WHEN** se arranca una sesión pidiendo desacople
- **THEN** la respuesta SHALL llegar en cuanto la sesión exista
- **AND** SHALL llevar su identificador
- **AND** el turno SHALL seguir corriendo y publicándose en el stream

#### Scenario: Sin pedirlo, el arranque responde como siempre

- **WHEN** se arranca una sesión sin pedir desacople
- **THEN** la respuesta SHALL llegar al terminar la sesión
- **AND** SHALL llevar el mismo resultado que hoy

## MODIFIED Requirements

### Requirement: Dirección de una sesión existente
El daemon SHALL aceptar instrucciones dirigidas a una sesión existente
(`session/direct`): sobre una sesión que ejecuta un turno la instrucción SHALL
encolarse y despacharse como el siguiente turno de la misma sesión del agente al
concluir el turno en curso, sin interrumpirlo; sobre una sesión viva que espera
instrucciones SHALL despacharse de inmediato como su siguiente turno; sobre una
sesión terminada y reanudable SHALL reanudarse con la instrucción como prompt;
sobre una sesión inexistente o no reanudable MUST rehusarse con diagnóstico y
remedio. Cada instrucción MUST registrarse en el log de sesión al encolarse y al
despacharse, y el verbo SHALL ser consumible desde todas las superficies por
igual.

#### Scenario: Instrucción a una sesión activa se despacha como siguiente turno
- **WHEN** un cliente dirige una instrucción a una sesión activa
- **THEN** la instrucción SHALL encolarse sin interrumpir el turno en curso
- **AND** al concluir ese turno SHALL despacharse como el siguiente prompt de la misma sesión del agente
- **AND** el encolado y el despacho SHALL constar en el JSONL

#### Scenario: Instrucción a una sesión que espera se despacha de inmediato
- **WHEN** un cliente dirige una instrucción a una sesión en espera
- **THEN** SHALL despacharse como su siguiente turno sin aguardar a que concluya
  nada
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
