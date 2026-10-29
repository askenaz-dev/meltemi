# gui-shell — delta

## ADDED Requirements

### Requirement: Acerca de dice cómo actualizar
La línea «Acerca de» de Ajustes SHALL mostrar el canal por el que se instaló la
app de escritorio y el comando que la actualiza, copiable, deducidos sin red de
la ruta de la app; WHERE el instalador no distinga el canal, SHALL mostrar los
comandos posibles. Un control «Ver releases» SHALL abrir la página de releases
en el navegador del usuario por el abridor del sistema, y la app MUST NOT hacer
ninguna petición de red para ello.

#### Scenario: El comando de actualización se copia
- **WHEN** el usuario abre Ajustes en una app instalada por un gestor reconocido
- **THEN** «Acerca de» SHALL mostrar el canal y su comando de actualización, copiable

#### Scenario: Ver releases abre el navegador del usuario
- **WHEN** el usuario elige «Ver releases»
- **THEN** la página de releases SHALL abrirse por el abridor del sistema
- **AND** la app SHALL NOT hacer ninguna petición de red

#### Scenario: Canal ambiguo
- **IF** el instalador no permite distinguir el canal
- **THEN** «Acerca de» SHALL mostrar los comandos de los canales posibles, sin elegir uno
