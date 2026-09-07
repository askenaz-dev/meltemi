# tui-shell — delta

## ADDED Requirements

### Requirement: Dentro de cada proyecto, las sesiones se ordenan por lo que piden

En la vista Sesiones del terminal, dentro de cada grupo de proyecto, las filas
SHALL ordenarse por cubeta de estado en orden de señal —esperan tu decisión,
listas para tu instrucción, trabajando, detenidas— con una cabecera por cubeta
que diga el estado con glifo y palabra y la cuenta; una cubeta vacía NO SHALL
dibujarse. La tabla de cubetas SHALL coincidir estado a estado y en orden con
la de la superficie de escritorio. El cursor SHALL seguir moviéndose fila a
fila, saltando las cabeceras sin detenerse en ellas y sin cambiar de modelo.
Los gestos de enviar y encolar SHALL seguir siendo los del terminal (`Enter` envía, `Tab` alterna el relevo) y el pie de
página SHALL nombrarlos; NO SHALL depender de combinaciones con Ctrl.

#### Scenario: Cubetas dentro del proyecto en el terminal

- **WHEN** un proyecto tiene sesiones en más de un estado
- **THEN** la vista SHALL mostrarlas bajo cabeceras de cubeta en orden de señal
- **AND** cada cabecera SHALL llevar glifo con gemelo ASCII, palabra y cuenta

#### Scenario: El cursor no se entera de las cubetas

- **WHEN** se recorre la lista con las flechas
- **THEN** el cursor SHALL pasar de una fila a la siguiente cruzando cabeceras
- **AND** la sesión seleccionada SHALL ser la misma que sin cubetas

#### Scenario: Enviar y encolar con las teclas del terminal

- **WHEN** se dirige una sesión que trabaja
- **THEN** `Enter` SHALL encolar y `Tab` seguido de `Enter` SHALL relevar
- **AND** el pie de página SHALL decirlo antes de pulsar nada
