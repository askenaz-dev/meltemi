# remote-access — delta

## ADDED Requirements

### Requirement: Emisor del aviso de espera en el cliente

El usuario MAY configurar, **solo en su configuración de ámbito usuario**, un
comando que un cliente conectado ejecute cuando algo empiece a esperar su
decisión. La configuración SHALL estar desactivada por defecto y NO SHALL
leerse de la configuración de un repositorio. El daemon NO SHALL leerla,
validarla ni ejecutarla.

El comando SHALL recibir, por variables de entorno y por ningún otro medio,
únicamente: la clase de espera —permiso, instrucción, gate o prueba—, el
identificador de sesión, el nombre de la máquina del cliente, el instante en
RFC 3339 y cuántas cosas esperan. NO SHALL recibir ruta de proyecto, nombre de
repositorio, texto de la petición, nombre de herramienta ni título de sesión.
Toda ampliación de ese contenido exige enmienda previa (§9).

El cliente SHALL lanzar el comando sin esperarlo, sin darle su entrada ni su
salida, y SHALL terminarlo al agotarse un tope de tiempo, de modo que un
comando que no responde NO SHALL bloquear la interfaz. El cliente SHALL emitir
un aviso por **evento de espera** y no uno por elemento en cola, y NO SHALL
emitir otro antes de un piso temporal declarado. Cuando la espera se resuelva,
el cliente NO SHALL emitir aviso alguno.

#### Scenario: Sin configurar, no se emite nada

- **WHEN** algo empieza a esperar y el usuario no configuró comando alguno
- **THEN** no se ejecuta nada y nada se envía a ningún sitio

#### Scenario: El aviso lleva el hecho y nada del proyecto

- **WHEN** el cliente emite un aviso de espera
- **THEN** el comando recibe la clase de espera, el identificador de sesión, la máquina, el instante y la cuenta
- **AND** no recibe ruta, ni nombre de proyecto, ni texto de la petición

#### Scenario: Un repositorio no configura tu aviso

- **WHEN** la configuración de un repositorio declara un comando de aviso
- **THEN** el cliente lo ignora y no ejecuta nada

#### Scenario: Una ráfaga de esperas produce un aviso con su cuenta

- **WHEN** varias cosas empiezan a esperar en rápida sucesión
- **THEN** se emite un aviso con la cuenta de lo que espera, no uno por elemento
- **AND** no se emite otro antes del piso temporal

#### Scenario: Un comando que no responde no bloquea al cliente

- **WHEN** el comando configurado no termina
- **THEN** el cliente sigue respondiendo y termina ese comando al agotarse el tope

#### Scenario: Resolver la espera no avisa

- **WHEN** lo que esperaba se decide
- **THEN** no se emite ningún aviso por ello
