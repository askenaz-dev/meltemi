# acp-session — delta

## MODIFIED Requirements

### Requirement: Terminación sin huérfanos
`meltemid` SHALL garantizar que el subproceso del agente termina cuando la sesión se cancela, finaliza o el daemon se apaga. Terminar al agente SHALL terminar también a todo proceso que ese subproceso haya necesitado para existir: WHERE la plataforma lance el agente a través de un intermediario —un script o un shell que a su vez crea el proceso real—, el daemon SHALL terminar al intermediario y a cuanto creó, y NO SHALL dar por terminado un agente cuyo proceso real siga vivo. WHERE la plataforma permita atar la vida de esos procesos a la del daemon, el daemon SHALL atarla al lanzarlos, de modo que un fin abrupto del daemon tampoco deje huérfanos. El identificador de proceso y el programa efectivo del agente SHALL constar en el log de sesión.

#### Scenario: Cancelación de sesión
- **WHEN** el cliente cancela una sesión activa
- **THEN** el subproceso del agente recibe la cancelación ACP y, de no terminar en un plazo razonable, es terminado por el daemon; no queda ningún proceso huérfano

#### Scenario: Un lanzador intermedio no deja huérfanos
- **WHERE** la plataforma resuelve el agente a un intermediario que crea el proceso real
- **WHEN** el daemon termina al agente
- **THEN** el intermediario y todo proceso que creó SHALL haber terminado
- **AND** el daemon NO SHALL reportar la sesión como terminada mientras alguno siga vivo

#### Scenario: El daemon termina de golpe y nada le sobrevive
- **WHERE** la plataforma permite atar la vida del agente a la del daemon
- **WHEN** el proceso del daemon termina sin ejecutar su apagado
- **THEN** los procesos de los agentes que lanzó SHALL terminar igualmente

## ADDED Requirements

### Requirement: El modo anunciado se lee en cualquiera de sus dos formas

WHERE el agente anuncie sus modos de sesión por el campo de modos del
protocolo y ninguna opción de configuración de categoría modo, el daemon
SHALL exponer esos modos como una opción de configuración de selección con
la categoría de modo, con el modo actual como valor actual, y SHALL fijar el
modo elegido por el verbo de modos del protocolo. WHERE el agente anuncie las
dos formas, la opción de configuración SHALL prevalecer entera y el campo de
modos NO SHALL fusionarse con ella. El contrato hacia las superficies NO
SHALL cambiar: la opción SHALL viajar por la misma forma que cualquier otra
opción anunciada.

#### Scenario: Modos sin opción de configuración se ofrecen como opción

- **WHEN** el agente responde a la apertura de sesión con modos y sin opciones de configuración
- **THEN** la sesión SHALL anunciar una opción de selección de categoría modo
- **AND** sus valores SHALL ser exactamente los modos disponibles, con el modo actual como valor actual

#### Scenario: Con las dos formas gana la opción de configuración

- **WHEN** el agente anuncia una opción de configuración de categoría modo y además el campo de modos
- **THEN** la sesión SHALL anunciar la opción de configuración tal cual
- **AND** ningún valor del campo de modos SHALL añadirse a ella

#### Scenario: El modo elegido viaja por el verbo de modos

- **WHERE** la opción de modo procede del campo de modos
- **WHEN** se fija esa opción por la vía de opciones de sesión
- **THEN** el daemon SHALL enviar el verbo de modos del protocolo con el modo elegido
- **AND** SHALL registrar el modo aceptado como valor actual sin relanzar la sesión

### Requirement: Lo que el agente cambia por su cuenta queda reflejado

WHEN el agente notifique un cambio del modo actual o una lista nueva de
opciones de configuración, el daemon SHALL actualizar lo anunciado de esa
sesión y SHALL registrarlo en el log como lo anunciado ahora, de modo que las
superficies lo vean sin relanzar. Una lista nueva de opciones SHALL
reemplazar entera a la anterior; NO SHALL fusionarse.

#### Scenario: Un cambio de modo del agente actualiza la opción

- **WHEN** el agente notifica un cambio del modo actual
- **THEN** la opción de modo de la sesión SHALL pasar a ese valor
- **AND** el log SHALL registrar lo anunciado ahora

#### Scenario: Una lista de opciones nueva reemplaza a la anterior

- **WHEN** el agente notifica una lista nueva de opciones de configuración
- **THEN** lo anunciado de la sesión SHALL ser exactamente esa lista
- **AND** ninguna opción de la lista anterior SHALL sobrevivir por su cuenta
