## ADDED Requirements

### Requirement: Spinner y cronómetro con gemelo ASCII, sin costo en reposo
Mientras alguna sesión trabaja, la cabecera del shell SHALL mostrar un spinner
braille (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) junto al número de sesiones que trabajan y una palabra,
y el detalle de una sesión que trabaja SHALL mostrar el tiempo de la sesión en
`mm:ss`. El spinner SHALL tener gemelo ASCII (`-\|/`) con su propio ciclo bajo
la política de presentación ASCII. El tic que los anima SHALL armarse solo
mientras alguna sesión trabaja: en reposo el pulso MUST NOT añadir redibujos.
Un tiempo que el shell no puede mantener al día MUST NOT mostrarse.

#### Scenario: La cabecera late mientras una sesión trabaja
- **WHEN** al menos una sesión trabaja
- **THEN** la cabecera SHALL mostrar el spinner con el número de sesiones que trabajan y la palabra
- **AND** SHALL retirarlo cuando ninguna trabaja

#### Scenario: El spinner ASCII usa su propio ciclo
- **WHERE** la presentación es ASCII
- **THEN** el spinner SHALL recorrer `-\|/` sin ningún carácter braille

#### Scenario: El detalle cuenta el tiempo de la sesión que trabaja
- **WHEN** el detalle muestra una sesión que trabaja
- **THEN** su línea SHALL mostrar el tiempo de la sesión en `mm:ss`
- **AND** una sesión terminada SHALL mostrar su duración final, y una viva que no trabaja SHALL NOT mostrar tiempo

#### Scenario: En reposo no hay tic
- **WHEN** ninguna sesión trabaja
- **THEN** el shell SHALL NOT armar el tic del pulso
