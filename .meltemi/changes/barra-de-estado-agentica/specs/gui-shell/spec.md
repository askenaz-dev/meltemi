# gui-shell — delta

## ADDED Requirements

### Requirement: La barra de estado dice sobre qué se trabaja

La barra de estado SHALL nombrar el proyecto activo y, cuando el proyecto lleva
el método, la change que reclama una decisión junto al artefacto que la espera;
si ninguna la reclama, SHALL decir cuántas hay activas. El recuento de sesiones
SHALL distinguir las que trabajan de las que esperan una decisión del usuario,
porque significan cosas distintas para quien lee. La espera de una compuerta
SHALL presentarse como propiedad de la change y NO como un estado de sesión.

#### Scenario: La barra nombra el proyecto y la compuerta que espera

- **WHEN** el proyecto activo lleva el método y una change espera una decisión
- **THEN** la barra SHALL nombrar el proyecto y esa change con su artefacto

#### Scenario: Sin compuerta pendiente, la barra dice cuántas changes hay

- **WHEN** ninguna change reclama una decisión
- **THEN** la barra SHALL decir cuántas están activas

#### Scenario: Las sesiones que trabajan se distinguen de las que esperan

- **WHEN** hay sesiones activas y sesiones esperando una decisión
- **THEN** la barra SHALL contarlas por separado

### Requirement: El consumo se dice medido o no se dice

La barra SHALL mostrar el consumo medido del proyecto en el día. Cuando el
consumo no está medido NO SHALL mostrar un cero: SHALL callar o declarar que no
fue reportado, con el motivo estable que la analítica entrega. Ningún valor
SHALL estimarse.

#### Scenario: El consumo medido se muestra

- **WHEN** la analítica reporta consumo medido para el proyecto en el día
- **THEN** la barra SHALL mostrarlo

#### Scenario: Sin medición no se inventa un cero

- **WHEN** la analítica no reporta consumo para el periodo
- **THEN** la barra NO SHALL mostrar un cero
- **AND** SHALL callar o decir que no fue reportado

### Requirement: Los segmentos llevan a su vista y ceden en orden declarado

Cada segmento que nombra algo con vista propia SHALL llevar a ella al activarse,
con nombre accesible y alcanzable por teclado. Cuando el ancho no alcanza, los
segmentos SHALL cederlo en un orden declarado; el estado de la conexión y las
decisiones pendientes NO SHALL cederlo nunca.

#### Scenario: Un segmento lleva a su vista

- **WHEN** el usuario activa el segmento del proyecto, la change, las sesiones,
  los permisos o el consumo
- **THEN** la superficie SHALL mostrar la vista correspondiente

#### Scenario: Al estrecharse, lo último que se cae

- **WHEN** el ancho disponible no alcanza para todos los segmentos
- **THEN** SHALL cederse primero el endpoint y después la versión
- **AND** la conexión y las decisiones pendientes SHALL permanecer
