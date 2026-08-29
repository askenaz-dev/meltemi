# agent-harness — delta

## ADDED Requirements

### Requirement: Cuatro ámbitos de harness con precedencia declarada

El harness SHALL componerse de cuatro ámbitos, del más general al más
específico: global del usuario, global por agente, del proyecto, y del proyecto
por agente. Lo específico SHALL pisar a lo general y el proyecto SHALL pisar al
usuario.

El eje por agente SHALL ser un directorio nombrado con el identificador del
catálogo de flota. Un identificador que el catálogo no conozca SHALL listarse
como desconocido y NO SHALL proyectarse: proyectar a un agente que el núcleo no
sabe nombrar es escribir a ciegas.

#### Scenario: Lo específico pisa lo general

- **WHERE** una misma regla existe en el ámbito global y en el del proyecto
- **THEN** SHALL regir la del proyecto
- **AND** la global SHALL seguir siendo visible como pisada

#### Scenario: Un agente que el catálogo no conoce no recibe proyección

- **WHERE** existe un directorio por agente cuyo identificador no está en el
  catálogo de flota
- **THEN** SHALL listarse como desconocido con su ruta
- **AND** NO SHALL proyectarse su contenido a destino alguno

### Requirement: El pilar de reglas se lee, no se interpreta

Una regla SHALL ser un `RULE.md` con front-matter y cuerpo Markdown, en un
directorio con su nombre. El daemon SHALL validar su **forma** y SHALL
conservar las claves que no conoce sin alterarlas; NO SHALL derivar
comportamiento propio de la semántica de una regla ni de los agentes que la
regla diga soportar.

El nombre declarado en el front-matter SHALL coincidir con el del directorio;
si no coinciden, la regla SHALL listarse inválida con diagnóstico y NO SHALL
proyectarse.

Una forma de front-matter que el lector no cubra SHALL producir un diagnóstico
que nombre la clave y la forma esperada. NO SHALL leerse a medias: un valor
truncado en silencio es una regla que aplica donde no debía.

#### Scenario: Las claves que el núcleo no conoce se conservan

- **WHEN** una regla declara claves fuera de las que el núcleo usa
- **THEN** SHALL leerse sin error
- **AND** esas claves SHALL mostrarse tal como fueron escritas

#### Scenario: Un nombre que no coincide con su directorio se rehúsa

- **WHERE** el front-matter declara un nombre distinto al del directorio
- **THEN** SHALL listarse inválida con diagnóstico
- **AND** NO SHALL proyectarse

#### Scenario: Una forma no cubierta se diagnostica en vez de leerse a medias

- **WHEN** un valor usa una forma que el lector no cubre
- **THEN** SHALL diagnosticarse nombrando la clave
- **AND** NO SHALL usarse un valor parcial

### Requirement: Vista efectiva con el origen de cada pieza

El daemon SHALL poder responder qué harness aplica —opcionalmente acotado a un
proyecto y a un agente— con, por cada pieza, su capa de origen y su ruta.

La respuesta SHALL incluir también lo que **no** aplica y por qué: lo pisado,
con la capa que lo pisó, y lo inválido, con su diagnóstico. Una vista que solo
enumere lo vigente deja sin responder por qué falta lo que el usuario escribió.

Sin proyecto declarado, SHALL responder únicamente los ámbitos globales.

#### Scenario: Cada pieza dice de qué capa viene

- **WHEN** se consulta el harness efectivo
- **THEN** cada pieza SHALL declarar su capa de origen y su ruta

#### Scenario: Lo pisado y lo inválido también se ven

- **WHERE** una regla fue pisada por una capa más específica o es inválida
- **THEN** SHALL aparecer en la respuesta con su motivo
- **AND** NO SHALL omitirse en silencio

#### Scenario: Sin proyecto solo se responde lo global

- **WHEN** se consulta sin declarar proyecto
- **THEN** SHALL responderse solo los ámbitos globales del usuario
