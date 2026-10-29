# release-distribution — delta

## ADDED Requirements

### Requirement: La versión siguiente la propone la integración continua
Tras cada push a la rama principal, la integración continua SHALL calcular la
versión siguiente aplicando la política de versiones a las changes archivadas
que ninguna sección del changelog lista, y SHALL abrir o actualizar una única
petición de cambio que escriba esa sección y esa versión y declare por qué ese
número. La ruptura SHALL medirse contra el último tag en el contrato `proto/` y
en la gramática de la CLI, y SHALL tomarse de la propuesta de la change para lo
que no se mide. La integración continua MUST NOT empujar a la rama principal,
crear tags ni mergear: la release SHALL publicarse solo cuando el mantenedor
mergea esa petición. IF la credencial que abre la petición no existe, THEN el
workflow SHALL declararlo en su ejecución y MUST NOT fallar la rama principal.

#### Scenario: Lo archivado sin release abre la PR de la versión siguiente
- **WHEN** la rama principal recibe un push y hay changes archivadas que ninguna sección del changelog lista
- **THEN** la integración continua SHALL abrir o actualizar una sola petición de cambio que escribe la sección y la versión
- **AND** su descripción SHALL decir qué regla dio el número y con qué evidencia

#### Scenario: Sin nada pendiente no queda PR de release
- **IF** ninguna change archivada falta en el changelog
- **THEN** SHALL NOT quedar abierta una petición de cambio de release

#### Scenario: La PR de release no publica sola
- **WHEN** la integración continua abre o actualiza la petición de release
- **THEN** SHALL NOT empujar a la rama principal, crear un tag ni mergear
- **AND** la release SHALL publicarse solo al mergear el mantenedor esa petición

#### Scenario: Ruptura medida en el contrato
- **IF** desde el último tag un schema del contrato desaparece, pierde una propiedad o un valor de enum, gana un campo requerido, o cambia la versión del protocolo
- **THEN** la versión propuesta SHALL tratarlo como ruptura
- **AND** la sección del changelog SHALL declarar la ruptura

#### Scenario: Ruptura medida en la gramática de la CLI
- **IF** desde el último tag un subcomando o una bandera de la referencia de la CLI desaparece, o cambia la tabla de códigos de salida
- **THEN** la versión propuesta SHALL tratarlo como ruptura

#### Scenario: Ruptura declarada por la propuesta
- **WHEN** la propuesta de una change pendiente declara una ruptura
- **THEN** la versión propuesta SHALL tratarla como ruptura aunque ningún diff la mida

#### Scenario: Sin la credencial de la App no hay PR y la rama principal sigue verde
- **IF** la credencial que abre la petición de release no está configurada
- **THEN** el workflow SHALL declararlo en su resumen
- **AND** SHALL NOT fallar

### Requirement: Un hito probado de punta a punta sube el minor
WHERE desde el último tag se haya añadido un informe de la prueba de un
desarrollador que nombra la última versión publicada y cuyo veredicto la
publica, la versión propuesta SHALL subir el minor aunque nada en ella rompa, y
la sección del changelog SHALL enlazar ese informe. Un informe cuyo veredicto
no publica MUST NOT contar como hito.

#### Scenario: El informe que publica sube el minor
- **WHEN** desde el último tag se añadió un informe que nombra la última versión publicada y cuyo veredicto la publica
- **THEN** la versión propuesta SHALL subir el minor
- **AND** la sección del changelog SHALL abrir con el enlace al informe

#### Scenario: Un informe que no publica no es hito
- **IF** el veredicto del informe dice que no se publica
- **THEN** la versión propuesta SHALL NOT subir el minor por hito
- **AND** la descripción de la petición SHALL decir por qué no cuenta

### Requirement: La preparación para 1.0 se calcula
La petición de release SHALL incluir la lista de preparación para 1.0, con cada
gate y su estado calculado de un hecho nombrado, y con los entregables
informativos aparte. Un gate cuyo hecho no se puede leer MUST figurar como no
cumplido, con el dato que falta. Con major 0 y todos los gates cumplidos, la
versión propuesta SHALL ser 1.0.0. Los gates SHALL cubrir cada criterio que el
documento fundacional nombra para el Hito v1.0.

#### Scenario: Cada PR de release lleva la lista
- **WHEN** la integración continua propone una versión
- **THEN** la descripción de la petición SHALL listar cada gate con su estado y el hecho que lo decide

#### Scenario: Todos los gates en verde proponen 1.0.0
- **WHEN** el major es 0 y todos los gates están cumplidos
- **THEN** la versión propuesta SHALL ser 1.0.0

#### Scenario: Un dato que falta no es un gate cumplido
- **IF** el hecho de un gate no se puede leer
- **THEN** ese gate SHALL figurar como no cumplido, nombrando el dato que falta

#### Scenario: El hito del documento fundacional y la lista no divergen
- **WHEN** el Hito v1.0 del documento fundacional nombra un criterio
- **THEN** la lista SHALL tener un gate para ese criterio, y la verificación SHALL fallar si no lo tiene

### Requirement: Reglas de versión después de 1.0
Con major mayor o igual que 1, una ruptura SHALL subir el major; una change
aditiva o un hito, el minor; y solo correcciones, el patch. Una change SHALL
contar como corrección solo si su propuesta lo declara; sin declaración, SHALL
contar como aditiva. La política de versiones escrita SHALL decir estas reglas
y cómo una propuesta declara una ruptura o una corrección.

#### Scenario: Ruptura después de 1.0 sube el major
- **WHEN** el major es 1 o más y lo pendiente trae una ruptura
- **THEN** la versión propuesta SHALL subir el major

#### Scenario: Solo correcciones suben el patch
- **WHEN** el major es 1 o más y cada change pendiente declara ser corrección
- **THEN** la versión propuesta SHALL subir el patch

#### Scenario: Sin declaración cuenta como aditiva
- **IF** una change pendiente no declara ruptura ni corrección
- **THEN** SHALL contar como aditiva
