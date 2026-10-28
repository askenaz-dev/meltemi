# remote-access — delta

## ADDED Requirements

### Requirement: Las máquinas del usuario tienen nombre en el cliente

El cliente SHALL poder leer una lista de máquinas de la configuración **de
usuario** —nombre y endpoint, donde el endpoint es el default de la plataforma
o un comando `exec:`— y NO SHALL leerla de ningún repositorio. Esa lista NO
SHALL contener material de claves —ni ruta de clave, ni passphrase, ni puerto,
ni usuario—: esos datos viven en la configuración `ssh` del propio usuario y en
su agente de claves. El cliente SHALL poder cambiar la máquina activa sin
reiniciarse, reconectando contra el endpoint de la nueva, y SHALL advertir del
trabajo sin guardar con la misma guarda que el cambio de proyecto. Ninguna
superficie SHALL agregar sesiones de más de una máquina: lo que se muestra es
lo de la máquina activa, y el cromo SHALL decir cuál.

#### Scenario: Las máquinas se leen del usuario y nunca de un repositorio

- **WHEN** un repositorio declara máquinas en su `.meltemi/config.toml`
- **THEN** esa lista SHALL ignorarse
- **AND** solo la configuración de usuario SHALL poder nombrar máquinas

#### Scenario: La lista no lleva material de claves

- **WHEN** se lee la lista de máquinas
- **THEN** NO SHALL admitirse ruta de clave, passphrase, puerto ni usuario
- **AND** el comando remoto SHALL nombrar un alias que resuelve la
  configuración `ssh` del usuario

#### Scenario: Cambiar de máquina reconecta y lo dice

- **WHEN** el usuario elige otra máquina
- **THEN** el cliente SHALL reconectar contra el endpoint de esa máquina sin
  reiniciarse
- **AND** el cromo SHALL decir en qué máquina está

#### Scenario: Probar una conexión no arranca nada

- **WHEN** el usuario prueba una máquina remota
- **THEN** SHALL abrirse la conexión, hacerse `initialize` y cerrarse,
  informando versión y latencia
- **AND** NO SHALL arrancarse ningún daemon: una máquina remota apagada SHALL
  fallar con su diagnóstico

#### Scenario: La presencia se muestra a pedido y no es una prueba de conexión

- **WHEN** el usuario abre la lista de máquinas
- **THEN** la presencia SHALL leerse en ese momento y NO SHALL sondearse de
  fondo
- **AND** una máquina en línea en la malla NO SHALL presentarse como una
  máquina con daemon: eso se sabe al probar o al conectar

## MODIFIED Requirements

### Requirement: El punto de encuentro en dos vías está documentado

La documentación de acceso remoto SHALL declarar el patrón del punto de
encuentro: ambos extremos marcan conexiones salientes hacia infraestructura
del propio usuario, de modo que el acceso funciona con cualquiera de los dos
extremos fuera de su red habitual. SHALL documentar la variante de bastión SSH
(con el túnel inverso permanente y sus precauciones de cuenta dedicada) y la
variante de red privada del usuario (malla con plano de control
autohospedado), incluyendo el estado de licencias de las piezas nombradas. La
documentación SHALL declarar la frontera: esa infraestructura es del usuario —
Meltemi MUST NOT empaquetarla, MUST NOT depender de ella para compilar o
testear, y el transporte final hacia el daemon sigue siendo el SSH del propio
usuario hacia el socket local. En la variante de malla, la identidad del
usuario y la lista de sus equipos SHALL documentarse como capacidad presente
que Meltemi lee del cliente oficial de la malla, sin cuentas propias. El
selector de máquinas SHALL documentarse como capacidad presente del cliente,
sin que el daemon gane nada. La única decisión que sigue perteneciendo a la
superficie móvil de fase 3 —la identidad por certificados SSH de la variante de
bastión— SHALL quedar anotada como nota de design para esa change, no como
capacidad presente.

#### Scenario: Los cuatro cuadrantes usan el mismo camino

- **WHEN** un usuario consulta la documentación del patrón
- **THEN** SHALL encontrar que el PC y el cliente remoto marcan hacia afuera
- **AND** que el camino es el mismo con cualquiera de los dos fuera de casa

#### Scenario: La malla del usuario no es una dependencia de Meltemi

- **WHEN** una propuesta de cambio añade al workspace una dependencia de la
  malla, del bastión, de un plano de control o de un IdP
- **THEN** SHALL rechazarse
- **AND** la documentación SHALL seguir declarando que esa infraestructura es
  del usuario y que compilar y testear Meltemi no exige cuenta ni red

#### Scenario: Lo de fase 3 está anotado y no prometido

- **WHEN** se lee la nota de fase 3
- **THEN** SHALL decir que lo anotado no existe todavía
- **AND** NO SHALL presentar como presente lo que es una decisión de design
- **AND** la identidad y la lista de equipos de la variante de malla SHALL
  estar descritas como capacidad presente y leída, no creada, por Meltemi
