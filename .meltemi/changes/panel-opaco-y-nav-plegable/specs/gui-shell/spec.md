# gui-shell — delta

## ADDED Requirements

### Requirement: Superficies flotantes con fondo propio

Toda superficie flotante de la GUI —panel, diálogo, cajón o menú que cubra
contenido— SHALL pintar un fondo opaco, de modo que lo que queda debajo no se
lea a través de ella. Los valores de estilo SHALL provenir de variables
definidas: la suite de la superficie de escritorio MUST fallar, nombrando
archivo y línea, cuando una variable de estilo se use sin estar definida.

#### Scenario: Ninguna variable de estilo se usa sin existir

- **WHEN** la suite de la superficie de escritorio se ejecuta
- **THEN** SHALL reunir cada variable de estilo usada y cada una definida
- **AND** SHALL fallar nombrando archivo y línea de cualquiera usada sin
  definición

#### Scenario: El conmutador de proyectos cubre lo que tapa

- **WHEN** el conmutador de proyectos se abre sobre la navegación
- **THEN** su panel SHALL declarar un fondo desde una variable definida
- **AND** el contenido de debajo MUST NOT leerse a través del panel

### Requirement: Barra lateral plegable

La barra de navegación SHALL poder plegarse a un riel angosto y desplegarse
desde un control visible en su cabecera, sin perder entrada alguna: plegada,
cada entrada SHALL conservar su etiqueta accesible y su acceso por teclado, y
el indicador de permisos pendientes SHALL permanecer visible. El estado
plegado SHALL recordarse entre arranques junto a las demás preferencias de
ventana, y un perfil nuevo SHALL arrancar desplegado.

#### Scenario: Plegar y desplegar desde la cabecera

- **WHEN** el usuario acciona el control de pliegue
- **THEN** la barra SHALL pasar a su riel angosto
- **AND** accionarlo de nuevo SHALL devolverla a su ancho completo

#### Scenario: Plegada no pierde alcance

- **WHILE** la barra está plegada
- **THEN** cada entrada de navegación SHALL conservar su etiqueta accesible
- **AND** el indicador de permisos pendientes SHALL seguir visible

#### Scenario: El pliegue se recuerda, el primer arranque no

- **WHEN** el usuario pliega la barra y vuelve a abrir la aplicación
- **THEN** la barra SHALL aparecer plegada
- **AND** un perfil sin preferencia guardada SHALL arrancar desplegado
