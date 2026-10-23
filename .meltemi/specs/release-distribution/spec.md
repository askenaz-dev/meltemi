# release-distribution Specification

## Purpose
TBD - created by archiving change distribucion-releases. Update Purpose after archive.
## Requirements

### Requirement: Versionado con política escrita
El proyecto SHALL versionar con SemVer pre-1.0 y versión única de workspace, con
una política escrita que defina qué constituye ruptura (contrato `proto/`,
gramática CLI, formato de artefactos); todo release SHALL originarse de un tag y
su pipeline.

#### Scenario: Ruptura exige minor
- **WHEN** un release incluye un cambio de contrato definido como ruptura
- **THEN** la versión SHALL incrementar el minor pre-1.0
- **AND** el cambio SHALL constar en las notas del release

### Requirement: Pipeline de release con gates duros
El pipeline SHALL construir y probar en Windows, macOS y Linux con gates
obligatorios: suite completa, clippy sin warnings, formato, auditoría de
dependencias y presupuestos de rendimiento medidos (tamaño del binario de la TUI
y arranque); cualquier gate rojo MUST abortar el release sin publicar artefacto
alguno.

#### Scenario: Presupuesto excedido aborta
- **IF** el binario de la TUI supera su presupuesto en el gate
- **THEN** el release SHALL abortarse
- **AND** ningún artefacto SHALL publicarse

### Requirement: Artefactos firmados con custodia documentada
Cada artefacto publicado SHALL acompañarse de su checksum y firma verificables,
y el procedimiento de custodia de la clave (generación, almacenamiento, rotación
y revocación) MUST estar documentado y en manos del mantenedor.

#### Scenario: Verificación por el usuario
- **WHEN** un usuario descarga un binario de release
- **THEN** SHALL poder verificar checksum y firma con instrucciones publicadas

### Requirement: Instalador auditable
El proyecto SHALL ofrecer un instalador de una línea por plataforma cuyo script
sea corto, legible y con hash publicado, junto a instrucciones manuales
equivalentes; la instalación SHALL colocar `meltemi` y `meltemid` y crear el
alias `mel`. Cada script instalador SHALL publicarse además como artefacto de la
release, con su checksum dentro del `SHA256SUMS` firmado y alcanzable por la URL
de última release, de modo que cualquier superficie pueda enlazarlo sin literal
de versión; el script MUST NOT hospedarse en ninguna otra ubicación, para que no
exista un segundo hash que verificar.

#### Scenario: Instalación con alias
- **WHEN** el instalador concluye
- **THEN** `meltemi`, `meltemid` y el alias `mel` SHALL quedar disponibles en el PATH del usuario

#### Scenario: Script instalador publicado y firmado
- **WHEN** se publica una release
- **THEN** cada script instalador SHALL constar como artefacto con su checksum en el `SHA256SUMS` firmado
- **AND** SHALL ser alcanzable por la URL de última release sin literal de versión

### Requirement: Espacios de nombres reservados
Los crates `meltemi`, `meltemid` y `meltemi-proto` SHALL quedar publicados en el
registro de crates — el contrato real y placeholders honestos que apuntan al
repositorio — para asegurar el espacio de nombres del proyecto.

#### Scenario: Crates apuntan al proyecto
- **WHEN** alguien consulta los crates del proyecto
- **THEN** SHALL encontrar el contrato publicado y los placeholders con su referencia al repositorio

### Requirement: Instaladores de la GUI de escritorio
El pipeline de release SHALL producir instaladores firmados de la GUI por
plataforma — MSI en Windows, DMG en macOS, paquete `.deb` en Linux — con la
misma custodia de firmas y los mismos gates del pipeline existente. Ningún
artefacto publicado SHALL embeber un motor de navegador: el runtime de webview
del sistema se aprovecha, se bootstrapea o se declara como dependencia del
paquete, de modo que el instalador SHALL mantenerse por debajo de 15 MB en toda
plataforma, y el tamaño MUST verificarse como gate bloqueante del pipeline.
WHERE un formato de empaquetado sea autocontenido por construcción y no permita
declarar el motor como dependencia externa, el pipeline MUST NOT publicarlo.

#### Scenario: Instalador firmado por plataforma
- **WHEN** el pipeline publica una release con GUI
- **THEN** cada plataforma SHALL recibir su instalador firmado bajo la custodia documentada

#### Scenario: Presupuesto de tamaño como gate
- **WHEN** un build de release produce un instalador de GUI que excede 15 MB
- **THEN** el pipeline SHALL fallar el gate de tamaño

#### Scenario: Formato autocontenido no se publica
- **IF** un formato de empaquetado embebe el motor de navegador en el artefacto
- **THEN** el pipeline SHALL NOT producirlo ni publicarlo
- **AND** la documentación de descargas SHALL NOT nombrarlo

#### Scenario: El paquete declara el motor del sistema
- **WHEN** el pipeline produce el paquete de Linux
- **THEN** el paquete SHALL declarar el runtime de webview del sistema entre sus dependencias
- **AND** la instalación en una máquina sin ese runtime SHALL fallar nombrando lo que falta, en vez de instalar y no arrancar

### Requirement: Nombres de artefacto estables por plataforma
Cada artefacto publicado SHALL llevar un nombre estable por plataforma y libre
de versión, de modo que la URL de descarga de la release más reciente resuelva
siempre al artefacto correcto sin que ningún consumidor tenga que conocer la
versión. WHERE la herramienta de empaquetado emita un nombre con la versión
incrustada —como los instaladores del cliente de escritorio—, el pipeline MUST
normalizarlo al esquema estable antes de publicarlo, y el checksum publicado
SHALL corresponder al nombre normalizado.

#### Scenario: Instalador de escritorio normalizado antes de publicar
- **WHEN** el empaquetado produce un instalador con la versión en su nombre
- **THEN** el pipeline SHALL renombrarlo al nombre estable de su plataforma
- **AND** el `SHA256SUMS` publicado SHALL registrar ese nombre estable

#### Scenario: Descarga sin conocer la versión
- **WHEN** un consumidor pide el artefacto de una plataforma por la URL de última release
- **THEN** SHALL recibir el artefacto de la release firmada más reciente

### Requirement: Publicación del sitio con la release
El pipeline SHALL publicar el sitio estático del producto en el dominio del
proyecto por HTTPS, y esa publicación MUST ocurrir únicamente después de que los
gates duros y el empaquetado hayan publicado sus artefactos: un gate rojo MUST
NOT publicar ni artefactos ni sitio, y la edición anterior del sitio SHALL
permanecer intacta. WHERE la publicación responda solo a un cambio de contenido
del sitio, SHALL exigir igualmente su lint verde y MUST NOT alterar los enlaces
de descarga, que son libres de versión. El alojamiento MUST ser estático, sin
backend y sin analítica.

#### Scenario: Gate rojo no publica el sitio
- **IF** cualquier gate del release falla
- **THEN** ni los artefactos ni el sitio SHALL publicarse
- **AND** el sitio publicado SHALL seguir siendo la edición anterior

#### Scenario: Sitio publicado tras los artefactos
- **WHEN** el empaquetado publica los artefactos de una release
- **THEN** el sitio SHALL publicarse a continuación con sus enlaces de descarga resolviendo a esa release

#### Scenario: Publicación de contenido sin tocar las descargas
- **WHEN** se publica un cambio que solo afecta al contenido del sitio
- **THEN** el lint del sitio SHALL exigirse verde antes de publicar
- **AND** los enlaces de descarga SHALL permanecer libres de versión

## MODIFIED Requirements

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
