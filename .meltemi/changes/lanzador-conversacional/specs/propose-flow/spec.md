# propose-flow — delta

## ADDED Requirements

### Requirement: Elección de agente en propose
El método `propose` SHALL admitir un parámetro opcional de agente, resuelto por
el orden vigente de la flota —perfil de lanzamiento, id del catálogo y, en su
defecto, el agente configurado del proyecto—, y la resolución efectiva MUST
quedar registrada en el log de la sesión, de modo que una reconstrucción desde el
log recupere qué agente redactó la propuesta. Un agente nombrado que resuelve a
un binario no detectado MUST rehusar el arranque con diagnóstico y remedio y MUST
NOT degradar en silencio a otro proveedor. Omitir el parámetro SHALL comportarse
exactamente como antes de esta capacidad: el agente configurado del proyecto, sin
cambio de forma para ningún cliente existente. La elección SHALL estar disponible
en las tres superficies por igual.

#### Scenario: Propose con agente nombrado
- **WHEN** un cliente invoca `propose` nombrando un perfil o un id del catálogo detectado
- **THEN** el daemon SHALL lanzar el binario resuelto por ese nombre
- **AND** la resolución con su fuente SHALL constar en el log de la sesión

#### Scenario: Propose sin agente se comporta como siempre
- **WHEN** un cliente invoca `propose` sin nombrar agente
- **THEN** SHALL usarse el agente configurado del proyecto
- **AND** el resultado SHALL conservar su forma vigente

#### Scenario: Propose con agente no detectado rehúsa sin degradar
- **IF** el agente nombrado resuelve a un binario no detectado
- **THEN** `propose` SHALL rehusarse con diagnóstico y remedio
- **AND** ningún otro proveedor SHALL redactar la propuesta en su lugar
