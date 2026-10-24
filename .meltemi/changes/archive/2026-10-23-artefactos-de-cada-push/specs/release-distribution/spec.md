## ADDED Requirements

### Requirement: Builds de integración descargables sin crear release
El pipeline SHALL producir, en cada avance de la rama principal, los mismos
artefactos por plataforma que produce el camino de release, y SHALL dejarlos
descargables desde la página de la ejecución que los construyó. Esa ruta MUST
NOT crear, modificar ni publicar release alguna —tampoco borrador ni
prelanzamiento—, MUST NOT emitir firma ni atestación, y MUST NOT alterar la
resolución de la URL de última release. El camino que crea y publica releases
SHALL seguir disparándose únicamente desde un tag, y la ruta de integración
SHALL vivir con su propio disparador, de modo que cambiar su cadencia no exija
reestructurar el pipeline de release.

#### Scenario: Push a main deja el build descargable
- **WHEN** la rama principal avanza
- **THEN** el pipeline SHALL construir los artefactos por plataforma del camino de release
- **AND** SHALL publicarlos como artefactos de la ejecución, descargables desde su página

#### Scenario: El build de integración no crea release
- **WHEN** la ruta de integración termina de construir
- **THEN** SHALL NOT haberse creado ni modificado release alguna, ni borrador ni prelanzamiento
- **AND** SHALL NOT haberse emitido firma ni atestación sobre esos artefactos

#### Scenario: La última release firmada sigue resolviendo
- **WHEN** un consumidor pide un artefacto por la URL de última release tras una ejecución de integración
- **THEN** SHALL recibir el artefacto de la última release firmada, no el de la ejecución

#### Scenario: El camino de release sigue siendo solo de tag
- **IF** la ejecución no proviene de un tag de versión
- **THEN** los jobs que crean o publican una release SHALL NOT ejecutarse

### Requirement: Identidad y caducidad de los artefactos de integración
Cada artefacto de una ejecución de integración SHALL nombrarse de modo que se
distinga de un artefacto de release sin abrirlo: el nombre MUST declarar su
condición de build sin firmar y MUST identificar el commit que lo produjo. Los
archivos contenidos SHALL conservar los nombres estables y libres de versión del
camino de release, para que la ruta ensaye también el paso de normalización de
nombres. La retención de estos artefactos MUST estar acotada y declarada en el
propio pipeline.

#### Scenario: Nombre del artefacto declara commit y build
- **WHEN** la ruta de integración sube su artefacto
- **THEN** el nombre SHALL declarar que es un build sin firmar
- **AND** SHALL identificar el commit que lo produjo
- **AND** los archivos contenidos SHALL llevar los nombres estables por plataforma

#### Scenario: Retención acotada y declarada
- **WHEN** el pipeline sube un artefacto de integración
- **THEN** SHALL declarar explícitamente su plazo de retención
- **AND** ese plazo SHALL ser acotado, no indefinido

### Requirement: Presupuestos de tamaño en toda ruta que empaquete
Los presupuestos de tamaño MUST aplicarse en cualquier ruta del pipeline que
empaquete artefactos, publique o no. WHERE una ruta produzca binarios,
adaptadores empaquetados o instaladores de escritorio, el pipeline SHALL medirlos
contra los mismos límites que el camino de release y MUST fallar el job cuando
alguno los exceda. Los límites SHALL ser los mismos valores en todas las rutas,
para que ninguna medición dependa de qué archivo la ejecute.

#### Scenario: Presupuesto excedido falla el build de integración
- **IF** una ruta que no publica produce un artefacto que excede su presupuesto de tamaño
- **THEN** el job SHALL fallar
- **AND** el exceso SHALL quedar registrado con el tamaño medido y el límite aplicado

### Requirement: Ausencia de firma declarada donde se descarga
Un artefacto que no lleva firma ni atestación SHALL declararlo donde el humano
lo descarga, y no únicamente en la documentación. La declaración MUST decir qué
garantías no acompañan al artefacto y MUST remitir a la release publicada como
la vía de instalación, y SHALL NOT presentar checksums no firmados como
verificación de origen.

#### Scenario: Aviso de build sin firmar donde se descarga
- **WHEN** la ruta de integración termina
- **THEN** la página de la ejecución SHALL declarar que sus artefactos no están firmados ni atestiguados
- **AND** SHALL remitir a la release publicada como vía de instalación

#### Scenario: El artefacto lleva su propio aviso
- **WHEN** alguien descarga y abre un artefacto de integración
- **THEN** SHALL encontrar junto a los binarios la declaración de que no está firmado
- **AND** los checksums que acompañen al artefacto SHALL NOT presentarse como verificación de origen
