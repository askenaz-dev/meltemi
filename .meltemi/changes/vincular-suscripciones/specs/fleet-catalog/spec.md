# fleet-catalog — delta

## ADDED Requirements

### Requirement: El vínculo de suscripción de primera clase

El daemon SHALL vincular suscripciones con nombre propio sobre agentes del
catálogo cuya entrada declara su variable de contexto de autenticación:
vincular SHALL crear un perfil de lanzamiento persistido en un archivo
gestionado por el daemon, cargado antes que la configuración manual, de modo
que un perfil homónimo escrito a mano SHALL ganar por la fusión por nombre
vigente. Desvincular SHALL retirar únicamente perfiles del archivo gestionado
y MUST rehusar con remedio cuando el perfil vive en configuración manual. Un
nombre de vínculo MUST validarse como componente seguro de ruta antes de
nombrar un directorio.

#### Scenario: Vincular crea el perfil y la sesión lo honra

- **WHEN** un cliente vincula una suscripción con nombre sobre un agente con
  variable declarada
- **THEN** el catálogo SHALL listar el perfil nuevo como fila propia
- **AND** una sesión que nombra ese perfil SHALL resolver al binario del
  agente subyacente con el contexto del vínculo

#### Scenario: Lo escrito a mano gana y no se desvincula por superficie

- **WHILE** existe un perfil manual con el mismo nombre que un vínculo
- **THEN** la resolución SHALL honrar el perfil manual
- **AND** desvincular ese nombre MUST rehusar con un remedio que apunta a la
  configuración manual

#### Scenario: Vincular sobre un agente sin variable declarada rehúsa

- **IF** la entrada del catálogo no declara variable de contexto
- **WHEN** un cliente intenta vincular una suscripción sobre ella
- **THEN** el daemon SHALL rehusar con diagnóstico
- **AND** el remedio SHALL nombrar la vía manual documentada

#### Scenario: El nombre inválido como ruta rehúsa

- **WHEN** un cliente vincula con un nombre que no es componente seguro de
  ruta
- **THEN** el daemon MUST rehusar antes de crear directorio o perfil alguno

### Requirement: La variable de contexto como dato del registro

La instantánea del registro SHALL poder declarar, por entrada, la variable de
entorno que redirige el contexto de autenticación del binario oficial y el
gesto de login que el proveedor documenta; ambas SHALL ser datos de la
instantánea versionada, y el código MUST NOT conocer proveedores por nombre
para componerlas.

#### Scenario: El registro declara la variable por entrada

- **WHEN** el catálogo se construye desde una instantánea con variable de
  contexto declarada
- **THEN** la entrada SHALL exponer esa variable y su gesto de login como
  datos consultables

#### Scenario: Registro sustituido declara sus propias variables

- **WHEN** una instantánea sustituida declara otra variable para una entrada
- **THEN** el vínculo SHALL componerse con la variable de esa instantánea
- **AND** ninguna variable SHALL provenir de conocimiento fijado en el código

### Requirement: El login compuesto, jamás ejecutado

El resultado de vincular SHALL entregar el gesto de autenticación completo —
la variable, el valor del contexto del vínculo y el gesto documentado del
proveedor — como datos para que el humano lo ejecute. El daemon MUST NOT
ejecutar login alguno, MUST NOT leer ni listar el contenido del directorio de
contexto, y desvincular MUST NOT borrar ese directorio; la respuesta de
desvincular SHALL nombrar la ruta que queda atrás.

#### Scenario: El vínculo entrega el gesto de login

- **WHEN** un vínculo se crea
- **THEN** la respuesta SHALL traer la variable, el valor y el gesto del
  proveedor
- **AND** el directorio de contexto recién creado SHALL quedar vacío

#### Scenario: Desvincular deja el contexto intacto

- **WHEN** un vínculo se deshace
- **THEN** el perfil SHALL desaparecer del catálogo
- **AND** el directorio de contexto SHALL permanecer con su contenido
- **AND** la respuesta SHALL nombrar esa ruta

### Requirement: Contexto duplicado advertido

- **WHILE** dos perfiles del mismo agente resuelven su variable de contexto
  al mismo valor, la carga de configuración SHALL emitir un diagnóstico de
  advertencia que los nombre, sin rehusar ninguno; y vincular con un nombre
  ya vinculado MUST rehusar con remedio.

#### Scenario: Mismo contexto dos veces se advierte

- **WHEN** la configuración carga dos perfiles del mismo agente con el mismo
  valor de contexto
- **THEN** los diagnósticos SHALL advertir que son la misma suscripción con
  dos nombres
- **AND** ambos perfiles SHALL seguir resolviendo

#### Scenario: Nombre ya vinculado rehúsa

- **WHEN** un cliente vincula con un nombre que ya existe en el archivo
  gestionado
- **THEN** el daemon MUST rehusar con un remedio que ofrece desvincular o
  renombrar
