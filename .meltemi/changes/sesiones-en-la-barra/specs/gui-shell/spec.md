# gui-shell — delta

## ADDED Requirements

### Requirement: Las sesiones de cada proyecto se agrupan por lo que piden

Dentro de cada nodo de proyecto del árbol de la barra lateral, las sesiones
SHALL agruparse en cubetas por estado, en orden de señal: las que esperan una
decisión (`waiting_permission`), las que esperan una instrucción
(`waiting_instruction`), las que trabajan (`active`, `starting`) y las
detenidas (`ended`, `interrupted`). Cada cubeta SHALL llevar glifo, palabra y
cuenta; una cubeta vacía NO SHALL dibujarse. Cada fila SHALL mostrar el
avatar del agente y el título de la sesión —o el agente y el identificador
corto cuando no hay título—, SHALL conservar el nombre de su suscripción como
pastilla y la palabra canónica del estado como nombre accesible, y SHALL
marcar con forma y palabra si la sesión está abierta en una pestaña. La cubeta de detenidas SHALL acotarse a las más
recientes y ofrecer «Ver las {n}» hacia la vista Sesiones. Un cambio de estado
SHALL mover la fila a su cubeta sin animar el layout: nada se mueve bajo el
cursor mientras se decide un permiso.

#### Scenario: Cuatro estados, cuatro cubetas en orden de señal

- **WHEN** un proyecto tiene sesiones en `waiting_permission`, `active`,
  `waiting_instruction` y `ended`
- **THEN** su nodo SHALL listarlas en cuatro cubetas en este orden: esperan tu
  decisión, listas para tu instrucción, trabajando, detenidas
- **AND** cada cabecera SHALL decir el estado con glifo y palabra y la cuenta
  de la cubeta

#### Scenario: Una cubeta vacía no ocupa sitio

- **WHEN** ninguna sesión del proyecto está en uno de los cuatro estados
- **THEN** la cabecera de esa cubeta NO SHALL dibujarse

#### Scenario: La fila dice el título y si está abierta

- **WHEN** una sesión con título y suscripción está abierta en una pestaña
- **THEN** su fila SHALL mostrar el avatar, el título y la pastilla de
  suscripción
- **AND** SHALL marcar que está abierta con forma y palabra, no con color solo
- **AND** activarla SHALL enfocar su pestaña sin crear otra

#### Scenario: Las detenidas no desbordan la barra

- **WHEN** un proyecto tiene más sesiones detenidas que el tope de la cubeta
- **THEN** la cubeta SHALL mostrar solo las más recientes
- **AND** SHALL ofrecer «Ver las {n}» que lleva a la vista Sesiones con el
  proyecto conmutado

#### Scenario: Un cambio de estado salta de cubeta sin animarse

- **WHEN** una sesión que trabaja pasa a esperar un permiso
- **THEN** su fila SHALL aparecer en la cubeta de decisión en el siguiente
  refresco
- **AND** ningún elemento de la barra SHALL animar su posición

### Requirement: Las pestañas abiertas se gobiernan desde la barra

La barra lateral SHALL listar las pestañas de sesión abiertas en una sección
propia, con la cuenta, en el mismo orden que la tira. Activar una fila SHALL
traer su pestaña al frente; cada fila SHALL ofrecer un control de cierre con
nombre accesible, y `Delete` sobre una fila enfocada SHALL cerrarla. Cerrar
desde la barra SHALL comportarse como cerrar desde la tira: el foco cae en la
vecina y, sin ninguna, queda la lista. La fila de la pestaña que está al frente
SHALL marcarse por forma y palabra. Cerrar una pestaña NO SHALL terminar la
sesión: sigue en su cubeta. Con la barra plegada a riel la sección NO SHALL
dibujarse y la tira SHALL seguir siendo el camino a cada pestaña.

#### Scenario: Seleccionar en la barra trae la pestaña al frente

- **WHEN** hay dos pestañas abiertas y se activa en la barra la que no está al
  frente
- **THEN** esa pestaña SHALL pasar al frente en la tira
- **AND** su fila SHALL quedar marcada como la actual

#### Scenario: Cerrar desde la barra cae en la vecina

- **WHEN** se cierra desde la barra la pestaña que está al frente
- **THEN** el foco SHALL pasar a la vecina de la izquierda, o a la última si no
  la hay
- **AND** si no queda ninguna SHALL quedar la lista en pantalla

#### Scenario: Cerrar no es olvidar

- **WHEN** se cierra desde la barra la pestaña de una sesión que trabaja
- **THEN** la sesión SHALL seguir en su cubeta con su estado
- **AND** activarla SHALL volver a abrir su pestaña con su lectura

#### Scenario: Plegada, la tira sigue siendo el camino

- **WHEN** la barra está plegada a riel
- **THEN** la sección de pestañas abiertas NO SHALL dibujarse
- **AND** cada pestaña SHALL seguir alcanzable desde la tira con el teclado

### Requirement: Nueva sesión nace como pestaña con el cursor dentro

Pedir una sesión nueva —desde la acción primaria, la entrada de la barra, la
acción rápida de un proyecto o su atajo— SHALL abrir en la tira una pestaña
«Nueva sesión» con el compositor, traerla al frente y poner el foco en el
campo. Pedirla de nuevo mientras existe SHALL enfocarla, no duplicarla. Enviar
SHALL convertir esa pestaña en la pestaña de la sesión creada, en el mismo
sitio y sin abrir otra, y su rótulo SHALL pasar a ser el título derivado. La
vista de llegada SHALL ser esa pestaña al frente con el foco en el campo. La
pestaña nueva SHALL contar contra el tope de pestañas. Cerrarla con borrador
escrito SHALL pedir decisión; vacía, NO SHALL preguntar.

#### Scenario: Pedir una sesión nueva abre su pestaña y da el foco

- **WHEN** se pide una sesión nueva desde cualquiera de sus puertas
- **THEN** la tira SHALL mostrar una pestaña «Nueva sesión» al frente
- **AND** el foco SHALL estar en el campo del compositor sin ningún clic más

#### Scenario: Pedirla de nuevo enfoca, no duplica

- **WHEN** ya hay una pestaña «Nueva sesión» abierta y se vuelve a pedir
- **THEN** esa pestaña SHALL pasar al frente con el foco en el campo
- **AND** NO SHALL crearse una segunda

#### Scenario: Enviar convierte la pestaña en la sesión

- **WHEN** se envía la primera instrucción desde la pestaña «Nueva sesión»
- **THEN** la misma pestaña SHALL pasar a representar la sesión creada
- **AND** su rótulo SHALL ser el título derivado de la instrucción
- **AND** NO SHALL abrirse otra pestaña

#### Scenario: Llegar es llegar a la pestaña nueva

- **WHEN** la superficie arranca sin última vista recordada
- **THEN** SHALL mostrar el compositor como pestaña «Nueva sesión» al frente,
  con el foco en el campo

#### Scenario: Cerrar con borrador pide decisión

- **WHEN** se cierra la pestaña «Nueva sesión» con texto escrito
- **THEN** la superficie SHALL pedir decisión antes de descartarlo
- **AND** sin texto SHALL cerrarla sin preguntar

### Requirement: Dos acordes en el compositor: despachar ahora y encolar

Todo compositor SHALL atender `Ctrl+Enter` (`Cmd+Enter` en macOS) como
*despachar ahora* y `Ctrl+Shift+Enter` como *encolar*; `Enter` a secas SHALL
insertar un salto de línea. Cuando la sesión espera una instrucción, despachar
ahora SHALL enviarla como siguiente turno. Cuando un turno está en vuelo,
despachar ahora SHALL **relevar** —interrumpir el turno y enviar la instrucción
como su relevo— y encolar SHALL dejarla detrás del turno sin interrumpir nada;
en ese estado el botón primario SHALL llamarse «Interrumpir y enviar» y
«Encolar» SHALL estar a la vista, cada uno con su atajo en la afordancia
`kbd`, y con el compositor vacío NO SHALL ofrecerse ninguno de los dos. Los
manejadores que atienden `Ctrl+Enter` SHALL comprobar que `Shift` no está
pulsado, de modo que los dos acordes nunca disparen la misma acción.

#### Scenario: Ctrl+Enter despacha cuando la sesión espera

- **WHEN** la sesión espera una instrucción y se pulsa `Ctrl+Enter` con texto
- **THEN** el texto SHALL enviarse como siguiente turno
- **AND** el campo SHALL quedar vacío y con el foco

#### Scenario: Ctrl+Shift+Enter encola y nunca interrumpe

- **WHEN** un turno está en vuelo y se pulsa `Ctrl+Shift+Enter` con texto
- **THEN** la instrucción SHALL quedar encolada detrás del turno con su
  posición declarada
- **AND** el turno en vuelo NO SHALL interrumpirse

#### Scenario: Ctrl+Enter mientras trabaja avisa antes y releva

- **WHEN** un turno está en vuelo y hay texto en el compositor
- **THEN** el botón primario SHALL leerse «Interrumpir y enviar» con su atajo
  y «Encolar» SHALL estar a la vista con el suyo
- **AND** pulsar `Ctrl+Enter` SHALL interrumpir el turno y enviar el texto como
  el siguiente

#### Scenario: Con el compositor vacío no hay nada que relevar

- **WHEN** un turno está en vuelo y el compositor está vacío
- **THEN** NO SHALL ofrecerse interrumpir y enviar
- **AND** ningún acorde SHALL despachar ni encolar nada

#### Scenario: Los acordes viejos no se pisan

- **WHEN** se pulsa `Ctrl+Shift+Enter` en la paleta, en el compositor de
  llegada o en el de una sesión
- **THEN** NO SHALL dispararse la acción de `Ctrl+Enter`

#### Scenario: Enter sigue siendo salto de línea

- **WHEN** se pulsa `Enter` sin modificadores en un compositor
- **THEN** SHALL insertarse un salto de línea
- **AND** nada SHALL enviarse

## MODIFIED Requirements

### Requirement: Varias sesiones abiertas a la vez en pestañas

La vista de sesiones SHALL permitir varias sesiones abiertas simultáneamente en
una tira de pestañas que contiene únicamente sesiones abiertas y, cuando se
pide una sesión nueva, su compositor: la lista es la vista Sesiones, no una
pestaña, y SHALL estar en pantalla cuando ninguna pestaña está al frente. Abrir una sesión ya abierta SHALL enfocar su pestaña
sin crear otra. Una pestaña que no está en pantalla SHALL conservar su
transcript, su lectura y su borrador sin enviar, y SHALL declarar cuántos
eventos llegaron sin leer. Cada pestaña SHALL declarar el estado de su sesión
con símbolo y palabra, nunca con color solo. La tira SHALL recorrerse entera
con el teclado. Alcanzado el tope de pestañas, la superficie SHALL rehusar
abrir otra sin cerrar ninguna y SHALL nombrar el remedio. Cada sesión abierta
SHALL declarar su interés por su propia sesión y NO SHALL mostrar los eventos
de otra.

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

#### Scenario: La lista es la vista, no una pestaña

- **WHEN** hay al menos una sesión abierta
- **THEN** la tira SHALL contener solo pestañas de sesión, sin una pestaña de
  lista
- **AND** la entrada Sesiones de la barra SHALL devolver la tabla completa con
  su filtro dejando las pestañas abiertas

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
- **AND** tras reiniciar la superficie SHALL arrancar sin pestañas de sesión,
  con la lista en pantalla si la vista recordada era Sesiones

#### Scenario: Cada sesión abierta lee su propio registro y su propio flujo

- **WHEN** varias sesiones están abiertas a la vez
- **THEN** cada una SHALL declarar su interés por su propia sesión y SHALL
  sembrar su transcript desde el registro persistido
- **AND** ninguna SHALL mostrar los eventos de otra
