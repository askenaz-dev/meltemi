## ADDED Requirements

### Requirement: Registro de proyectos conocidos persistido y reconstruible
El daemon SHALL mantener un registro de los proyectos que ha visto —clave de
proyecto, raíz absoluta, primera y última vez vista— persistido de forma
apend-only en su directorio de datos, y MUST poder reconstruirlo desde el índice
de sesiones cuando falte o esté dañado: los registros de sesión son la fuente de
verdad. WHERE la raíz de un proyecto ya no existe en disco, el registro MUST
conservar su entrada marcándola como ausente, y MUST NOT borrarla en silencio.

#### Scenario: Registro reconstruido desde el índice de sesiones
- **WHEN** el registro de proyectos falta y existen sesiones indexadas de dos proyectos
- **THEN** el daemon SHALL reconstruir el registro desde esos registros de sesión
- **AND** ambos proyectos SHALL quedar listados con su raíz

#### Scenario: Raíz desaparecida se conserva marcada
- **WHEN** la raíz de un proyecto registrado ya no existe en disco
- **THEN** el proyecto SHALL seguir listado marcado como ausente
- **AND** SHALL NOT desaparecer del registro sin aviso

#### Scenario: Alta repetida no duplica el proyecto
- **WHEN** el mismo proyecto se usa varias veces
- **THEN** el registro SHALL presentar una sola entrada para esa clave
- **AND** SHALL conservar la primera vez vista y actualizar la última

### Requirement: Registro alimentado por el uso real
El daemon SHALL dar de alta un proyecto únicamente en los momentos en que el
usuario ya apuntó Meltemi a ese repositorio: al arrancar una sesión sobre esa
raíz y al resolver el contexto de proyecto por contrato. El daemon MUST NOT
recorrer el disco del usuario en busca de repositorios ni inferir proyectos de
ninguna otra fuente.

#### Scenario: Una sesión estrena el proyecto en el registro
- **WHEN** se arranca la primera sesión sobre una raíz nunca usada
- **THEN** el proyecto SHALL quedar registrado con esa raíz
- **AND** su última actividad SHALL corresponder a esa sesión

#### Scenario: Ningún proyecto aparece sin haberse usado
- **WHEN** se consulta el registro en una máquina con repositorios que Meltemi nunca usó
- **THEN** el registro SHALL listar solo los proyectos usados desde Meltemi
- **AND** el daemon SHALL NOT haber recorrido el disco para poblarlo

### Requirement: Consulta project/list con paridad de superficies
El daemon SHALL exponer `project/list` con los proyectos del registro ordenados
por recencia, cada uno con su clave, su raíz, si sigue existiendo en disco, la
marca de su última actividad y los contadores de sesiones activas y totales, de
modo que ningún cliente necesite leer el disco del daemon. El método SHALL ser
invocable desde la CLI, desde la paleta de la TUI y desde el registro tipado de
la GUI por igual (constitución §4), y MUST constar en la matriz de paridad.

#### Scenario: Dos proyectos listados por recencia
- **WHEN** un cliente invoca `project/list` con sesiones recientes en dos proyectos
- **THEN** la respuesta SHALL incluir ambos con su raíz y sus contadores
- **AND** el más recientemente activo SHALL aparecer primero

#### Scenario: Proyecto sin sesiones vivas sigue presente
- **WHEN** un proyecto registrado no tiene ninguna sesión activa
- **THEN** SHALL aparecer en el listado con cero sesiones activas
- **AND** SHALL conservar su total de sesiones históricas

#### Scenario: Método con casa en las tres superficies
- **WHEN** el contrato incorpora `project/list`
- **THEN** el subcomando de la CLI, la paleta de la TUI y el registro de la GUI SHALL ofrecerlo
- **AND** la matriz de paridad SHALL incluir su fila
