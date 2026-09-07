# remote-access — delta

## ADDED Requirements

### Requirement: La identidad es la que la malla del usuario ya conoce

El daemon SHALL poder responder la identidad que el cliente oficial de la
malla del usuario conoce —nombre de login, servidor de login o nombre de la
malla, y el nombre de este equipo— invocando ese cliente en modo de solo
lectura y parseando su salida. El daemon MUST NOT ejecutar ningún login ni
ningún comando que cambie el estado de la malla, MUST NOT abrir conexiones de
red por sí mismo, y MUST NOT leer archivos de estado ni claves privadas del
cliente; de la salida del cliente NO SHALL conservar ni exponer más que los
campos del modelo —sin direcciones ni claves—. WHERE no hay cliente de malla
configurado, no se encuentra, o el equipo no ha iniciado sesión en la malla, la
consulta SHALL rehusarse con un diagnóstico que distinga los tres casos y con
su remedio, sin degradar nada más. La identidad NO SHALL ser requisito para
ningún uso local, y el daemon NO SHALL etiquetar ni distinguir conexiones por
identidad.

#### Scenario: Quién soy, según la malla

- **WHEN** hay un cliente de malla configurado y este equipo está enlazado
- **THEN** la consulta SHALL responder el nombre de login, el servidor o
  nombre de la malla y el nombre de este equipo
- **AND** todo ello SHALL provenir de la salida del cliente oficial, no de
  ningún dato guardado por Meltemi

#### Scenario: Sin cliente de malla, rehúso con remedio

- **WHERE** no hay tabla `[identity]` o el binario no se encuentra
- **THEN** la consulta SHALL rehusarse con diagnóstico y el remedio de
  configurar o instalar el cliente
- **AND** ninguna otra capacidad del daemon SHALL verse afectada

#### Scenario: La identidad nunca es requisito

- **WHEN** un cliente inicia una conexión con el daemon
- **THEN** el saludo NO SHALL exigir ni transportar identidad
- **AND** el daemon NO SHALL distinguir esa conexión de otra por identidad

#### Scenario: Ningún token entra ni sale del daemon

- **WHEN** se lee la identidad de la malla
- **THEN** el daemon SHALL invocar solo un comando de lectura del cliente
- **AND** NO SHALL leer archivos de estado ni claves privadas del cliente, ni
  conservar de su salida nada fuera de los campos del modelo

#### Scenario: Cliente presente, equipo sin enlazar

- **WHEN** el cliente responde que este equipo no tiene sesión en la malla
- **THEN** la consulta SHALL responder sin identidad, con una razón que la
  distinga del cliente ausente
- **AND** SHALL ofrecer el gesto de vínculo como remedio

### Requirement: El vínculo de este equipo se compone y nunca se ejecuta

El daemon SHALL componer, como texto, el gesto que enlaza este equipo a la
malla del usuario: el comando del cliente oficial con el servidor de login
configurado y el nombre del equipo, en forma POSIX y PowerShell, con la pista
de que abrirá el navegador del usuario para iniciar sesión en su propio
proveedor. El daemon MUST NOT ejecutarlo. El gesto MUST NOT contener secretos.
Un cliente MAY ejecutar el gesto únicamente a petición explícita del usuario,
con la entrada y la salida heredadas para que el usuario vea lo que su
cliente imprime.

#### Scenario: El gesto de vínculo se imprime, no se ejecuta

- **WHEN** se pide el vínculo de este equipo
- **THEN** la respuesta SHALL traer el comando en forma POSIX y PowerShell con
  el servidor de login y el nombre del equipo
- **AND** ningún proceso del cliente de malla SHALL haberse lanzado

#### Scenario: Ejecutar es explícito y visible

- **WHEN** el usuario pide ejecutar el gesto desde el CLI
- **THEN** SHALL correr el binario del propio usuario con la entrada y la
  salida heredadas
- **AND** el URL o la instrucción que el cliente imprima SHALL llegar al
  usuario sin filtro

#### Scenario: El gesto no lleva secretos

- **WHEN** se compone el gesto
- **THEN** NO SHALL contener claves, tokens ni claves de preautorización
- **AND** el lint de secretos en claro SHALL aplicarse a la configuración que
  lo alimenta

### Requirement: Las máquinas del usuario se leen, no se registran

El daemon SHALL responder la lista de equipos del usuario tal como la malla la
conoce ahora —nombre, presencia, sistema y última vez visto— marcando cuál es
este equipo, y NO SHALL mantener registro propio de equipos. La presencia
SHALL ser de solo lectura y NO SHALL constituir control: el transporte hacia
un daemon remoto sigue siendo únicamente el túnel SSH del usuario, y si en un
equipo corre un daemon se sabe al conectar, nunca se supone. La lista SHALL
poder consumirse desde el CLI, la TUI y la GUI (constitución §4; la regla de
subconjunto vive en `mobile-companion` y no se duplica aquí).

#### Scenario: Las máquinas son los pares de la malla

- **WHEN** la malla conoce tres equipos del usuario y uno está fuera de línea
- **THEN** la lista SHALL traer los tres con su presencia y su sistema
- **AND** SHALL marcar cuál es este equipo

#### Scenario: Presencia no es control

- **WHEN** un equipo aparece en línea en la lista
- **THEN** ninguna superficie SHALL abrir una conexión hacia él por ese hecho
- **AND** el camino hacia su daemon SHALL seguir siendo el túnel del usuario

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
que Meltemi lee del cliente oficial de la malla, sin cuentas propias. Las
decisiones que siguen perteneciendo a la superficie móvil de fase 3 —la
identidad por certificados SSH de la variante de bastión, el selector que
conecta a un equipo, el aviso de espera— SHALL quedar anotadas como notas de
design para esa change, no como capacidades presentes.

#### Scenario: Los cuatro cuadrantes usan el mismo camino

- **WHEN** un usuario consulta la documentación del patrón
- **THEN** SHALL encontrar que el PC y el cliente remoto marcan hacia afuera
- **AND** que el camino es el mismo con cualquiera de los dos fuera de casa

#### Scenario: La malla del usuario no es una dependencia de Meltemi

- **WHEN** una propuesta de cambio añade al workspace una dependencia de la
  infraestructura de encuentro (malla, bastión, plano de control, IdP)
- **THEN** la propuesta SHALL rechazarse: esa infraestructura es del usuario
- **AND** compilar y testear Meltemi SHALL seguir sin exigir cuenta ni red

#### Scenario: Lo de fase 3 está anotado y no prometido

- **WHEN** un usuario lee la documentación del patrón
- **THEN** la identidad y la lista de equipos de la variante de malla SHALL
  estar descritas como capacidad presente y leída, no creada, por Meltemi
- **AND** las piezas que siguen siendo de fase 3 SHALL estar marcadas como
  notas de design y NO SHALL presentarse como capacidades existentes
