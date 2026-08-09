# gui-shell — delta

## ADDED Requirements

### Requirement: Reparto ajustable entre la navegación y los proyectos

La barra lateral SHALL repartir su alto entre las entradas de navegación y el
árbol de proyectos por un separador que el usuario puede mover, operable con
puntero y con teclado, con nombre accesible y con su valor y sus límites
declarados. El reparto SHALL respetar un mínimo por cada zona; cuando la barra
no dé para ambos mínimos, las entradas SHALL conservar el suyo y el árbol
SHALL desplazarse con scroll. El reparto SHALL recordarse entre arranques junto
a las demás preferencias de ventana, y un perfil nuevo SHALL arrancar en el
reparto por defecto. Plegada la barra a su riel, el separador NO SHALL
renderizarse ni el reparto guardado SHALL aplicarse.

#### Scenario: Arrastrar la línea reparte el alto

- **WHEN** el puntero arrastra el separador entre las entradas y el árbol
- **THEN** las entradas SHALL tomar el alto arrastrado
- **AND** el árbol SHALL tomar el resto
- **AND** ningún control SHALL desplazarse por animación de layout

#### Scenario: El reparto tiene suelo por los dos lados

- **WHEN** se pide un reparto que dejaría a una de las dos zonas por debajo de
  su mínimo
- **THEN** el reparto SHALL acotarse a ese mínimo
- **AND** en una barra demasiado corta para ambos mínimos las entradas SHALL
  conservar el suyo y el árbol SHALL desplazarse con scroll

#### Scenario: El reparto se ajusta con el teclado

- **WHEN** el separador tiene el foco
- **THEN** las flechas SHALL moverlo un paso y Home y End SHALL llevarlo a sus
  extremos
- **AND** el separador SHALL exponer su rol, su nombre accesible y su valor
  actual con sus límites

#### Scenario: El reparto se recuerda, el primer arranque no

- **WHEN** la superficie se reabre tras haberse movido el separador
- **THEN** SHALL restaurar el reparto guardado junto a las demás preferencias
  de ventana
- **AND** un perfil sin preferencia guardada SHALL arrancar en el reparto por
  defecto

#### Scenario: Una ventana más pequeña no deja el reparto inservible

- **WHEN** la barra se reduce por debajo del reparto recordado
- **THEN** el reparto SHALL reajustarse a lo que la barra puede dar
- **AND** ninguna de las dos zonas SHALL quedar por debajo de su mínimo

#### Scenario: Plegada la barra, no hay reparto que hacer

- **WHILE** la barra lateral está plegada a su riel
- **THEN** el separador NO SHALL renderizarse
- **AND** el reparto guardado NO SHALL aplicarse, y SHALL volver intacto al
  desplegar

#### Scenario: Ninguna entrada se pierde al encoger la navegación

- **WHEN** el reparto deja a las entradas menos alto del que ocupan
- **THEN** el excedente SHALL desplazarse con scroll dentro de la navegación
- **AND** cada entrada SHALL conservar su etiqueta accesible, su foco y su
  dígito

### Requirement: Barras de desplazamiento de la superficie

Toda región desplazable de la GUI SHALL presentar una barra de desplazamiento
angosta y sin botones de paso, con su color tomado de los mismos tokens que el
resto del cromo, de modo que siga al tema sin una declaración por tema ni por
región. La declaración SHALL usar propiedades estándar de CSS; la superficie
MUST NOT depender de selectores de barra específicos de un motor. Estrechar la
barra MUST NOT comprimir ninguna fila: el excedente SHALL seguir
desplazándose.

#### Scenario: El árbol de proyectos desplaza sin comerse la columna

- **WHEN** el árbol de proyectos renderiza más filas de las que caben
- **THEN** su barra de desplazamiento SHALL ser angosta y sin botones de paso
- **AND** cada fila SHALL conservar al menos su altura de línea
- **AND** el excedente SHALL desplazarse, nunca comprimirse

#### Scenario: La barra sigue el tema sin una segunda declaración

- **WHEN** el usuario conmuta entre tema claro y tema oscuro
- **THEN** la barra SHALL tomar su color del mismo token que el resto del cromo
- **AND** NO SHALL existir una regla de barra por tema ni por región

### Requirement: Varias sesiones abiertas a la vez en pestañas

La vista de sesiones SHALL permitir varias sesiones abiertas simultáneamente en
una tira de pestañas cuya primera pestaña es la lista y NO SHALL ser cerrable.
Abrir una sesión ya abierta SHALL enfocar su pestaña sin crear otra. Una
pestaña que no está en pantalla SHALL conservar su transcript, su lectura y su
borrador sin enviar, y SHALL declarar cuántos eventos llegaron sin leer. Cada
pestaña SHALL declarar el estado de su sesión con símbolo y palabra, nunca con
color solo. La tira SHALL recorrerse entera con el teclado. Alcanzado el tope de
pestañas, la superficie SHALL rehusar abrir otra sin cerrar ninguna y SHALL
nombrar el remedio. Cada sesión abierta SHALL declarar su interés por su propia
sesión y NO SHALL mostrar los eventos de otra.

#### Scenario: Abrir una segunda sesión no reemplaza la primera

- **WHEN** se abre una sesión estando otra abierta
- **THEN** ambas SHALL quedar abiertas como pestañas
- **AND** la recién abierta SHALL quedar enfocada
- **AND** la anterior SHALL conservar su lectura

#### Scenario: Abrir dos veces la misma sesión enfoca, no duplica

- **WHEN** se pide abrir una sesión que ya tiene pestaña
- **THEN** el foco SHALL moverse a esa pestaña
- **AND** NO SHALL crearse una segunda
- **AND** su contador de eventos sin leer SHALL volver a cero

#### Scenario: La lista es la primera pestaña y nunca se cierra

- **WHEN** hay al menos una sesión abierta
- **THEN** la tira SHALL presentar la lista como primera pestaña sin control de
  cierre
- **AND** seleccionarla SHALL devolver la tabla completa con su filtro

#### Scenario: Una pestaña de fondo conserva su lectura y su borrador

- **WHEN** se conmuta a otra pestaña y se vuelve
- **THEN** el transcript, la lectura elegida y el borrador sin enviar SHALL
  estar como se dejaron
- **AND** si se estaba al pie SHALL volver al pie

#### Scenario: La pestaña de fondo dice que llegó algo

- **WHEN** una sesión que no está en pantalla recibe eventos
- **THEN** su pestaña SHALL declarar cuántos llegaron sin leer
- **AND** enfocarla SHALL poner ese contador a cero

#### Scenario: El estado de cada pestaña se lee sin color

- **WHEN** una pestaña representa una sesión activa, una esperando permiso o una
  finalizada
- **THEN** SHALL declarar ese estado con símbolo y palabra
- **AND** el color NO SHALL ser el único portador

#### Scenario: La tira se recorre entera con el teclado

- **WHEN** la tira de pestañas tiene el foco
- **THEN** las flechas SHALL moverse entre pestañas con ciclo y Home y End SHALL
  ir a los extremos
- **AND** cada pestaña SHALL nombrar el panel que controla

#### Scenario: Cerrar la pestaña activa cae en la vecina

- **WHEN** se cierra la pestaña que está en pantalla
- **THEN** el foco SHALL pasar a la vecina de la izquierda, o a la última si no
  la hay
- **AND** si no queda ninguna SHALL quedar seleccionada la lista

#### Scenario: El tope se rehúsa nombrando el remedio

- **WHEN** se pide abrir una sesión más habiendo alcanzado el tope
- **THEN** la superficie SHALL rehusar sin cerrar ninguna pestaña
- **AND** SHALL decir cuántas caben y qué hacer para abrir otra

#### Scenario: Cambiar de vista no cierra las pestañas; reiniciar sí las olvida

- **WHEN** se navega a otra vista de primer nivel y se vuelve a Sesiones
- **THEN** las pestañas SHALL seguir abiertas
- **AND** tras reiniciar la superficie SHALL arrancar sin pestañas de sesión y
  con la lista en pantalla

#### Scenario: Cada sesión abierta lee su propio registro y su propio flujo

- **WHEN** varias sesiones están abiertas a la vez
- **THEN** cada una SHALL declarar su interés por su propia sesión y SHALL
  sembrar su transcript desde el registro persistido
- **AND** ninguna SHALL mostrar los eventos de otra
