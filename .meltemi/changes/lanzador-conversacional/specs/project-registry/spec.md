# project-registry — delta

## ADDED Requirements

### Requirement: Alta explícita de un proyecto por contrato
El daemon SHALL exponer `project/register`, que da de alta una raíz de proyecto
recibida del cliente: MUST validar que la ruta existe y es un directorio,
rehusando con diagnóstico y remedio cuando no; MUST canonicalizarla antes de
guardarla, de modo que la raíz que el registro presenta sea siempre la forma
canónica y no la que se tecleó, y que dos formas equivalentes de la misma
carpeta queden en una sola entrada; y SHALL ser idempotente, conservando la
primera vez vista y actualizando la última. El método MUST NOT exigir que la raíz contenga `.meltemi/`
—registrar es apuntar la herramienta a un directorio, no iniciarlo como
proyecto—, MUST NOT crear ni modificar nada en disco, y MUST NOT recorrer el
disco del usuario: recibe una ruta y la valida. El diálogo de selección de
carpeta, cuando exista, vive en el cliente; el daemon MUST NOT abrir ventanas ni
enumerar el sistema de archivos fuera de la ruta entregada.

#### Scenario: Alta de un directorio que nunca corrió una sesión
- **WHEN** un cliente registra una raíz existente que el registro no conocía
- **THEN** el proyecto SHALL quedar listado con esa raíz canonicalizada
- **AND** SHALL NOT requerir que la raíz contenga artefactos del método

#### Scenario: Alta repetida no duplica ni pierde la primera vez
- **WHEN** se registra dos veces la misma raíz, la segunda escrita de otra forma equivalente
- **THEN** el registro SHALL presentar una sola entrada, con la raíz en su forma canónica
- **AND** SHALL conservar la primera vez vista y actualizar la última

#### Scenario: Ruta inexistente rehusada con remedio
- **IF** la ruta a registrar no existe o no es un directorio
- **THEN** el alta SHALL rehusarse con diagnóstico y remedio
- **AND** el registro SHALL quedar sin cambios

#### Scenario: El alta no toca el disco del usuario
- **WHEN** se registra una raíz
- **THEN** el daemon SHALL NOT crear ni modificar archivo alguno dentro de esa raíz
- **AND** SHALL NOT recorrer directorios fuera de la ruta entregada

### Requirement: Baja de registro que jamás toca el disco ni la historia
El daemon SHALL exponer `project/forget`, que da de baja un proyecto **solo del
registro**: SHALL apendar una línea de olvido al registro apend-only que el
plegado last-wins resuelve, y MUST NOT borrar ni modificar nada en disco, ni
sesiones, ni registros de sesión, ni el árbol del proyecto. La baja MUST NOT
exigir que la raíz exista, porque un proyecto ausente en disco es precisamente
el que se quiere olvidar. Un proyecto olvidado SHALL dejar de aparecer en el
listado del registro y SHALL seguir presente en el histórico de sesiones, en la
lectura de sus logs y en la analítica; y SHALL reaparecer en el listado en
cuanto se use o se registre de nuevo. La documentación del método MUST declarar
que el olvido rige sobre el listado y no es una promesa de permanencia: WHERE el
registro se reconstruya desde los registros de sesión por faltar o estar dañado,
el proyecto SHALL reaparecer, porque los registros de sesión son la fuente de
verdad.

#### Scenario: Olvidar oculta del listado y conserva todo lo demás
- **WHEN** un cliente olvida un proyecto con sesiones históricas
- **THEN** el listado del registro SHALL dejar de incluirlo
- **AND** sus sesiones SHALL seguir listándose y sus logs SHALL seguir leyéndose
- **AND** ningún archivo del proyecto SHALL modificarse

#### Scenario: Olvidar una raíz que ya no existe en disco
- **WHEN** un cliente olvida un proyecto cuya raíz desapareció
- **THEN** la baja SHALL completarse
- **AND** SHALL NOT rehusarse por no poder canonicalizar la ruta

#### Scenario: Un proyecto olvidado reaparece al volver a usarse
- **WHEN** se arranca una sesión sobre la raíz de un proyecto olvidado
- **THEN** el proyecto SHALL volver a aparecer en el listado
- **AND** SHALL conservar su primera vez vista

#### Scenario: Todo olvidado no dispara la reconstrucción
- **WHEN** el registro contiene entradas y todas fueron olvidadas
- **THEN** el listado SHALL quedar vacío
- **AND** el daemon SHALL NOT reconstruir el registro desde los registros de sesión por ese motivo

## MODIFIED Requirements

### Requirement: Registro alimentado por el uso real
El daemon SHALL dar de alta un proyecto en los momentos en que el usuario ya
apuntó Meltemi a ese repositorio: al arrancar una sesión sobre esa raíz, al
resolver el contexto de proyecto por contrato, y al darlo de alta explícitamente
por contrato con la ruta que el propio usuario entregó. El daemon MUST NOT
recorrer el disco del usuario en busca de repositorios ni inferir proyectos de
ninguna otra fuente: un alta explícita es una ruta recibida y validada, jamás un
descubrimiento.

#### Scenario: Una sesión estrena el proyecto en el registro
- **WHEN** se arranca la primera sesión sobre una raíz nunca usada
- **THEN** el proyecto SHALL quedar registrado con esa raíz
- **AND** su última actividad SHALL corresponder a esa sesión

#### Scenario: Un alta explícita estrena el proyecto sin correr nada
- **WHEN** el usuario registra una raíz por contrato sin haber corrido ninguna sesión en ella
- **THEN** el proyecto SHALL quedar registrado con esa raíz
- **AND** SHALL NOT haberse ejecutado ningún agente para conseguirlo

#### Scenario: Ningún proyecto aparece sin haberse usado
- **WHEN** se consulta el registro en una máquina con repositorios que Meltemi nunca usó
- **THEN** el registro SHALL listar solo los proyectos usados o registrados desde Meltemi
- **AND** el daemon SHALL NOT haber recorrido el disco para poblarlo

### Requirement: Consulta project/list con paridad de superficies
El daemon SHALL exponer `project/list` con los proyectos del registro ordenados
por recencia, cada uno con su clave, su raíz, si sigue existiendo en disco, la
marca de su última actividad y los contadores de sesiones activas y totales, de
modo que ningún cliente necesite leer el disco del daemon. Los tres métodos del
registro —`project/list`, `project/register` y `project/forget`— SHALL ser
invocables desde la CLI, desde la paleta de la TUI y desde el registro tipado de
la GUI por igual (constitución §4), y MUST constar en la matriz de paridad. El
diálogo nativo de selección de carpeta es cromo del cliente y MUST NOT contar
como la vía de acceso de ninguna de las tres superficies: donde no haya diálogo,
la ruta se entrega como texto.

#### Scenario: Dos proyectos listados por recencia
- **WHEN** un cliente invoca `project/list` con sesiones recientes en dos proyectos
- **THEN** la respuesta SHALL incluir ambos con su raíz y sus contadores
- **AND** el más recientemente activo SHALL aparecer primero

#### Scenario: Proyecto sin sesiones vivas sigue presente
- **WHEN** un proyecto registrado no tiene ninguna sesión activa
- **THEN** SHALL aparecer en el listado con cero sesiones activas
- **AND** SHALL conservar su total de sesiones históricas

#### Scenario: Método con casa en las tres superficies
- **WHEN** el contrato incorpora un método del registro de proyectos
- **THEN** el subcomando de la CLI, la paleta de la TUI y el registro de la GUI SHALL ofrecerlo
- **AND** la matriz de paridad SHALL incluir su fila

#### Scenario: Alta sin diálogo nativo en la superficie de terminal
- **WHERE** la superficie no dispone de diálogo nativo del sistema
- **WHEN** el usuario quiere dar de alta un proyecto
- **THEN** SHALL poder entregar la ruta como texto
- **AND** el alta SHALL comportarse igual que la iniciada desde un diálogo
