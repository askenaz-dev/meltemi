# gui-shell — delta

## ADDED Requirements

### Requirement: La tira de pestañas es una sola fila que se desplaza

La tira de pestañas SHALL presentarse en una sola fila y NO SHALL envolver a un
segundo renglón. Cuando las pestañas no quepan, SHALL encogerse hasta un ancho
mínimo legible y, pasado ese punto, la tira SHALL desplazarse en horizontal.
Los controles de desplazamiento SHALL existir únicamente mientras haya
desbordamiento, y cada uno SHALL deshabilitarse en su extremo en vez de
desaparecer.

#### Scenario: Muchas pestañas no producen un segundo renglón

- **WHEN** hay más pestañas de las que caben en el ancho disponible
- **THEN** la tira SHALL seguir siendo una sola fila
- **AND** las pestañas SHALL encogerse hasta su ancho mínimo antes de que la
  tira se desplace

#### Scenario: Los controles aparecen solo cuando sobran pestañas

- **WHEN** las pestañas caben en el ancho disponible
- **THEN** NO SHALL renderizarse control de desplazamiento alguno
- **WHEN** dejan de caber
- **THEN** SHALL renderizarse los dos controles, cada uno deshabilitado en su
  extremo

#### Scenario: La pestaña activa nunca queda fuera de vista

- **WHEN** la pestaña activa cambia y queda fuera del área visible de la tira
- **THEN** la tira SHALL desplazarse lo mínimo para mostrarla
- **AND** una pestaña ya visible NO SHALL provocar desplazamiento

### Requirement: Grupos de pestañas

Las pestañas SHALL poder agruparse bajo un nombre y un color. Una pestaña SHALL
pertenecer a lo sumo a un grupo, y un grupo sin pestañas SHALL dejar de existir.
El nombre del grupo SHALL viajar en el nombre accesible de cada pestaña que le
pertenece: el color NO SHALL ser el único portador de la pertenencia. Plegar un
grupo NO SHALL cerrar ninguna pestaña ni descartar su trabajo, y el grupo
plegado SHALL declarar como texto cuántas guarda. WHERE la pestaña activa
pertenezca a un grupo que se pliega, la actividad SHALL pasar a una pestaña
visible.

#### Scenario: Una pestaña pertenece a un grupo y lo dice

- **WHEN** una pestaña se une a un grupo
- **THEN** su nombre accesible SHALL incluir el nombre del grupo
- **AND** la pertenencia NO SHALL depender solo del color

#### Scenario: Salir del grupo y el grupo que se queda vacío

- **WHEN** la última pestaña de un grupo lo abandona o se cierra
- **THEN** el grupo SHALL dejar de existir
- **AND** la pestaña SHALL seguir abierta

#### Scenario: Plegar guarda espacio, no trabajo

- **WHEN** un grupo se pliega
- **THEN** SHALL declarar como texto cuántas pestañas guarda
- **AND** ninguna pestaña SHALL cerrarse ni perder su borrador

#### Scenario: Plegar el grupo de la pestaña activa mueve la actividad

- **WHEN** se pliega el grupo al que pertenece la pestaña activa
- **THEN** la actividad SHALL pasar a una pestaña visible fuera del grupo
- **AND** si no queda ninguna, SHALL pasar a la lista
