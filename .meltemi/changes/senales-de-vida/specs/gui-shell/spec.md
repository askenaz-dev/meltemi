## ADDED Requirements

### Requirement: Vocabulario de movimiento acotado
La GUI SHALL animar solo con las clases del vocabulario de movimiento de
`docs/ux/design-system.md`, cada una con su lugar, su token de duración
(`--dur-1`, `--dur-2`, `--dur-pulse`, `--dur-ambient`, con `--ease-out`) y su
equivalente sin movimiento. Las animaciones con nombre SHALL ser exactamente las
de una lista cerrada —`composer-wind`, `arrive`, `skeleton-pulse` y
`mark-draw`— y SHALL animar solo opacidad o transform, con la única excepción
del trazo de la marca, que dibuja su contorno. Ninguna SHALL animar altura,
posición ni color de fondo; la bandeja de permisos, las tarjetas de permiso,
los avisos y la barra lateral MUST NOT animarse; y todo archivo que anima SHALL
retirar su animación bajo `prefers-reduced-motion`.

#### Scenario: Una animación fuera de la lista rompe la suite
- **WHEN** un archivo de la interfaz declara una animación con nombre que no está en la lista cerrada
- **THEN** la suite SHALL fallar nombrando el archivo y la animación

#### Scenario: Toda animación se retira con movimiento reducido
- **WHEN** un archivo de la interfaz declara una animación
- **THEN** el mismo archivo SHALL declarar la rama de `prefers-reduced-motion` que la retira

#### Scenario: Las duraciones salen de los tokens
- **WHEN** una animación de la interfaz declara su duración
- **THEN** SHALL tomarla de un token del vocabulario y no de un literal

#### Scenario: Las superficies de decisión no se mueven
- **WHEN** se recorre la fuente de la bandeja de permisos, de la tarjeta de permiso del transcript, de los avisos y de la barra lateral
- **THEN** ninguna SHALL declarar animación ni transición

#### Scenario: La marca se dibuja una vez en la bienvenida
- **WHEN** se abre la bienvenida
- **THEN** el contorno de la marca SHALL dibujarse una vez, en 600 ms
- **AND** bajo movimiento reducido SHALL aparecer ya dibujado

### Requirement: Estados de carga con esqueleto y umbral
Una vista que espera datos del daemon SHALL mostrar un esqueleto en lugar de la
palabra de carga: filas de 32 px con barras de anchos fijos, marcadas
`aria-busy="true"` y con el texto de carga del catálogo para lectores de
pantalla. El esqueleto SHALL aparecer solo cuando la espera supera 150 ms,
SHALL latir con un pulso de opacidad y MUST NOT llevar un gradiente en barrido.
Bajo movimiento reducido las barras SHALL quedar quietas y el texto de carga
SHALL verse. Un botón cuya etiqueta cambia mientras espera SHALL conservar su
ancho.

#### Scenario: Una carga rápida no parpadea
- **WHEN** los datos llegan antes de 150 ms
- **THEN** la vista SHALL NOT haber mostrado el esqueleto

#### Scenario: Una carga lenta muestra el esqueleto
- **WHEN** la espera supera 150 ms
- **THEN** la vista SHALL mostrar el esqueleto con `aria-busy` y el texto de carga accesible
- **AND** SHALL retirarlo cuando llegan los datos

#### Scenario: El esqueleto se queda quieto con movimiento reducido
- **WHERE** el sistema declara preferencia de movimiento reducido
- **THEN** las barras SHALL quedar quietas y el texto de carga SHALL verse

#### Scenario: Ninguna vista espera con una palabra suelta
- **WHEN** se recorre la fuente de Project, Review, Fleet, Settings y Usage
- **THEN** ninguna espera de bloque SHALL ser solo la palabra de carga
- **AND** los botones que la usan como etiqueta SHALL declarar un ancho mínimo

### Requirement: Señal de vida del turno
Mientras el agente de una sesión trabaja, el transcript SHALL decir que el
turno está en curso: antes del primer fragmento, una línea «trabajando…» con el
tiempo del turno en `m:ss`, que avanza cada segundo; con fragmentos, un cursor
fijo tras el último, que MUST NOT parpadear. La señal SHALL retirarse cuando el
agente deja de trabajar —turno cerrado, sesión que espera una decisión o una
instrucción, sesión terminada— y un turno cerrado sin partes SHALL seguir
diciendo «sin salida». El tiempo de la sesión en la cabecera SHALL avanzar cada
segundo mientras la sesión vive, en cifras tabulares, y quedar fijo en su
duración final cuando termina.

#### Scenario: Antes del primer fragmento se dice que trabaja
- **WHEN** el agente trabaja y el turno aún no tiene fragmentos
- **THEN** el transcript SHALL mostrar «trabajando…» con el tiempo transcurrido desde la instrucción, y no «sin salida»

#### Scenario: El cursor sigue al último fragmento sin parpadear
- **WHEN** el turno en curso tiene fragmentos
- **THEN** un cursor fijo SHALL seguir al último fragmento
- **AND** SHALL desaparecer cuando el turno se cierra

#### Scenario: Esperar una decisión no es trabajar
- **WHEN** la sesión espera una decisión de permiso o una instrucción
- **THEN** el transcript SHALL NOT mostrar la señal de vida

#### Scenario: El tiempo de la sesión corre mientras vive
- **WHEN** la sesión vive
- **THEN** su tiempo en la cabecera SHALL avanzar cada segundo en cifras tabulares
- **AND** cuando termina SHALL quedar fijo en su duración final

### Requirement: Ventana visible tras el primer frame
La ventana principal SHALL crearse oculta, recibir la geometría restaurada
mientras está oculta y mostrarse cuando la interfaz montó su primer árbol, o a
los 500 ms si la interfaz no lo señala, lo que ocurra antes, y una sola vez.

#### Scenario: La ventana nace oculta y con su geometría
- **WHEN** arranca la aplicación
- **THEN** la ventana SHALL crearse oculta
- **AND** la geometría restaurada SHALL aplicarse antes de mostrarla

#### Scenario: La interfaz lista muestra la ventana
- **WHEN** la interfaz monta su primer árbol
- **THEN** SHALL pedir al anfitrión que muestre la ventana

#### Scenario: Una interfaz muda no deja la ventana invisible
- **WHEN** la interfaz no lo señala en 500 ms
- **THEN** el anfitrión SHALL mostrar la ventana igualmente
- **AND** una señal tardía SHALL NOT volver a mostrarla
