# tui-shell — delta

## ADDED Requirements

### Requirement: El drill-in de sesión lee como conversación

El transcript del shell SHALL mostrar lo que los eventos dicen y no solo el
nombre de su tipo: la prosa del agente, su pensamiento y las llamadas a
herramienta con su estado. El pensamiento SHALL distinguirse de la prosa por
palabra y no únicamente por color, con su gemelo ASCII. Los eventos que no
llevan contenido SHALL conservar su línea de tipo, que es lo que son. Sin
pensamiento emitido NO SHALL mostrarse marcador alguno en su lugar.

#### Scenario: El transcript dice lo que el agente dijo

- **WHEN** el transcript recibe prosa del agente
- **THEN** SHALL mostrar su texto y no solo el nombre del tipo de evento

#### Scenario: El pensamiento se distingue de la prosa

- **WHEN** el transcript recibe pensamiento del agente
- **THEN** SHALL marcarlo con una palabra que lo distinga de la prosa
- **AND** la distinción NO SHALL depender únicamente del color

#### Scenario: Un evento sin contenido sigue diciendo su tipo

- **WHEN** el transcript recibe un evento que no lleva contenido
- **THEN** SHALL mostrar su tipo como hasta ahora
