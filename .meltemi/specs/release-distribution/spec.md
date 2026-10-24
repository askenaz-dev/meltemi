# release-distribution Specification

## Purpose
TBD - created by archiving change distribucion-releases. Update Purpose after archive.
## Requirements

### Requirement: Versionado con política escrita
El proyecto SHALL versionar con SemVer pre-1.0 y versión única de workspace, con
una política escrita que defina qué constituye ruptura (contrato `proto/`,
gramática CLI, formato de artefactos). La versión SHALL escribirse igual en todo
archivo que la porte, y una divergencia MUST fallar la verificación nombrando
los archivos. Toda release SHALL originarse de un commit de la rama principal
que trae una versión sin release publicada, y de su pipeline; el tag de la
versión SHALL crearlo el pipeline sobre ese commit, y ningún tag empujado por
una persona SHALL disparar una publicación.

#### Scenario: Ruptura exige minor
- **WHEN** un release incluye un cambio de contrato definido como ruptura
- **THEN** la versión SHALL incrementar el minor pre-1.0
- **AND** el cambio SHALL constar en las notas del release

#### Scenario: Versión nueva en la rama principal publica
- **WHEN** la rama principal recibe un commit cuya versión no tiene release publicada
- **THEN** el pipeline SHALL correr los gates y publicar la release de esa versión sobre ese commit
- **AND** el tag de la versión SHALL haberlo creado el pipeline, apuntando a ese commit

#### Scenario: Push sin versión nueva no publica
- **WHEN** la rama principal recibe un commit cuya versión ya tiene release
- **THEN** ningún job que cree o publique una release SHALL ejecutarse

#### Scenario: Tag empujado a mano no publica
- **WHEN** una persona empuja un tag de versión
- **THEN** ningún pipeline SHALL publicar una release por ese tag

#### Scenario: Versión divergente entre portadores
- **IF** los archivos que portan la versión no dicen la misma
- **THEN** la verificación SHALL fallar nombrando los que divergen
- **AND** el pipeline SHALL NOT publicar

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
Cada artefacto publicado SHALL acompañarse de su checksum y de una firma
verificable emitida por el workflow que lo publicó, sin clave de firma de largo
plazo. La custodia documentada SHALL ser la del repositorio y la del workflow:
la documentación MUST declarar qué identidad firma, qué controles la protegen y
qué no puede garantizar, y MUST declarar que una cuenta con permiso de escritura
puede publicar una release que verifica. El ancla de confianza SHALL ser la
identidad del workflow firmante, y las instrucciones publicadas MUST NOT tomarla
de la página de release que autentica. MUST NOT existir clave privada de firma
de releases en el repositorio, en la integración continua ni en máquina alguna.
WHERE una release publicada resulte comprometida o defectuosa, el repudio SHALL
consistir en publicar la versión siguiente y declarar la anterior, con fecha, en
sus notas y en la documentación de releases.

#### Scenario: Verificación por el usuario
- **WHEN** un usuario descarga un binario de release
- **THEN** SHALL poder verificar checksum y firma con instrucciones publicadas
- **AND** la firma SHALL verificar contra la identidad del workflow que publicó la release

#### Scenario: Límite de la firma del workflow declarado
- **WHEN** la documentación describe la firma de las releases
- **THEN** SHALL declarar que una cuenta con permiso de escritura puede publicar una release que verifica
- **AND** SHALL nombrar los controles que protegen esa ruta

#### Scenario: Ninguna clave de firma que custodiar
- **WHEN** se inspeccionan el repositorio, los workflows y la documentación de releases
- **THEN** SHALL NOT encontrarse clave de firma de releases ni instrucciones para usar una

#### Scenario: Repudio de una release
- **IF** una release publicada resulta comprometida o defectuosa
- **THEN** el procedimiento documentado SHALL publicar la versión siguiente
- **AND** SHALL declarar la release repudiada, con fecha, en sus notas y en la documentación de releases

### Requirement: Instalador auditable
El proyecto SHALL ofrecer un instalador de una línea por plataforma cuyo script
sea corto, legible y con hash publicado, junto a instrucciones manuales
equivalentes; la instalación SHALL colocar `meltemi` y `meltemid` y crear el
alias `mel`. Cada script instalador SHALL publicarse además como artefacto de la
release, con su checksum dentro del `SHA256SUMS` que cubre la atestación de la
release y alcanzable por la URL de última release, de modo que cualquier
superficie pueda enlazarlo sin literal de versión; el script MUST NOT
hospedarse en ninguna otra ubicación, para que no exista un segundo hash que
verificar.

#### Scenario: Instalación con alias
- **WHEN** el instalador concluye
- **THEN** `meltemi`, `meltemid` y el alias `mel` SHALL quedar disponibles en el PATH del usuario

#### Scenario: Script instalador publicado y atestiguado
- **WHEN** se publica una release
- **THEN** cada script instalador SHALL constar como artefacto con su checksum en el `SHA256SUMS` que cubre la atestación
- **AND** SHALL ser alcanzable por la URL de última release sin literal de versión

### Requirement: Espacios de nombres reservados
Los crates `meltemi`, `meltemid` y `meltemi-proto` SHALL quedar publicados en el
registro de crates — el contrato real y placeholders honestos que apuntan al
repositorio — para asegurar el espacio de nombres del proyecto.

#### Scenario: Crates apuntan al proyecto
- **WHEN** alguien consulta los crates del proyecto
- **THEN** SHALL encontrar el contrato publicado y los placeholders con su referencia al repositorio

### Requirement: Instaladores de la GUI de escritorio
El pipeline de release SHALL producir instaladores de la GUI por plataforma —
MSI en Windows, DMG en macOS, paquete `.deb` en Linux — cubiertos por la misma
atestación y los mismos gates que el resto de los artefactos de la release.
Ningún artefacto publicado SHALL embeber un motor de navegador: el runtime de
webview del sistema se aprovecha, se bootstrapea o se declara como dependencia
del paquete, de modo que el instalador SHALL mantenerse por debajo de 15 MB en
toda plataforma, y el tamaño MUST verificarse como gate bloqueante del
pipeline. WHERE un formato de empaquetado sea autocontenido por construcción y
no permita declarar el motor como dependencia externa, el pipeline MUST NOT
publicarlo.

#### Scenario: Instalador atestiguado por plataforma
- **WHEN** el pipeline publica una release con GUI
- **THEN** cada plataforma SHALL recibir su instalador cubierto por la atestación de la release

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
- **THEN** SHALL recibir el artefacto de la release publicada más reciente

### Requirement: Builds de integración descargables sin crear release
El pipeline SHALL producir, en cada avance de la rama principal, los mismos
artefactos por plataforma que produce el camino de release, y SHALL dejarlos
descargables desde la página de la ejecución que los construyó. Esa ruta MUST
NOT crear, modificar ni publicar release alguna —tampoco borrador ni
prelanzamiento—, MUST NOT emitir firma ni atestación, y MUST NOT alterar la
resolución de la URL de última release. El camino que crea y publica releases
SHALL abrirse únicamente cuando la rama principal recibe una versión sin release
publicada, y la ruta de integración SHALL vivir en su propio workflow, sin el
permiso de escritura que crear una release exige, de modo que cambiar su
cadencia no exija reestructurar el pipeline de release.

#### Scenario: Push a main deja el build descargable
- **WHEN** la rama principal avanza
- **THEN** el pipeline SHALL construir los artefactos por plataforma del camino de release
- **AND** SHALL publicarlos como artefactos de la ejecución, descargables desde su página

#### Scenario: El build de integración no crea release
- **WHEN** la ruta de integración termina de construir
- **THEN** SHALL NOT haberse creado ni modificado release alguna, ni borrador ni prelanzamiento
- **AND** SHALL NOT haberse emitido firma ni atestación sobre esos artefactos

#### Scenario: La última release publicada sigue resolviendo
- **WHEN** un consumidor pide un artefacto por la URL de última release tras una ejecución de integración
- **THEN** SHALL recibir el artefacto de la última release publicada, no el de la ejecución

#### Scenario: La ruta de integración no puede publicar
- **WHEN** se inspecciona el workflow de la ruta de integración
- **THEN** SHALL carecer del permiso de escritura que crear o publicar una release exige

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

### Requirement: Procedencia verificable de la release
Cada release publicada SHALL llevar una atestación de build, emitida por el
workflow que la publicó, que ligue los artefactos publicados al repositorio, al
commit y al workflow de origen, y el bundle de esa atestación SHALL publicarse
como artefacto de la misma release. La verificación MUST poder hacerse con un
comando publicado, con red y sin ella. La documentación MUST declarar con
precisión qué cubre la atestación: WHERE la atestación se emita en un job que
solo agrega artefactos ya construidos, la documentación SHALL decir que
atestigua esa agregación y no cada paso de construcción. WHERE la atestación se
registre en un log de transparencia público, la documentación SHALL declararlo
y SHALL distinguirlo de telemetría.

#### Scenario: Procedencia publicada con la release
- **WHEN** el pipeline publica una release
- **THEN** los artefactos publicados SHALL quedar cubiertos por una atestación del workflow que los publicó
- **AND** el bundle de esa atestación SHALL constar entre los artefactos de la release

#### Scenario: Verificación sin red con el bundle publicado
- **WHEN** un usuario sin red verifica un artefacto con el bundle publicado y una raíz de confianza descargada antes
- **THEN** la verificación SHALL ser posible con el comando publicado en la documentación

#### Scenario: Alcance de la atestación declarado sin exagerar
- **WHEN** la documentación describe la atestación
- **THEN** SHALL decir qué job la emite y qué cubre realmente
- **AND** SHALL NOT afirmar una cobertura de construcción que la atestación no tiene

#### Scenario: Registro público de la atestación declarado
- **WHEN** la atestación se registra en un log de transparencia público
- **THEN** la documentación SHALL declararlo, con qué queda registrado
- **AND** SHALL distinguirlo de telemetría: es metadato de build, nunca dato de usuario

### Requirement: Publicación idempotente y compatible con releases inmutables
El pipeline SHALL reunir todos los artefactos de una release, el bundle de su
atestación incluido, antes de publicarla, y MUST NOT adjuntar nada a una release
ya publicada. Una re-ejecución tras un fallo parcial SHALL completar lo que
falte y saltar lo ya publicado, sin crear una segunda release. Todo paso que
deba seguir a la publicación SHALL correr dentro del mismo pipeline, en
dependencia de la publicación, y MUST NOT depender de eventos que una
publicación hecha por el propio pipeline no dispara. WHERE exista en
`CHANGELOG.md` una sección de la versión publicada, las notas de la release
SHALL ser esa sección.

#### Scenario: Re-ejecución tras fallo parcial termina lo que falta
- **IF** una ejecución falla después de crear la release y antes de publicarla
- **THEN** volver a ejecutarla SHALL subir los artefactos que falten y publicar la release
- **AND** SHALL NOT crear una segunda release

#### Scenario: Nada se adjunta tras publicar
- **WHEN** el pipeline publica una release
- **THEN** todos sus artefactos, el bundle de la atestación incluido, SHALL estar ya adjuntos

#### Scenario: Notas desde el changelog
- **WHEN** `CHANGELOG.md` tiene una sección de la versión que se publica
- **THEN** las notas de la release SHALL ser esa sección

### Requirement: Controles estructurales del camino sin paso humano
WHILE la publicación de una release no exija la intervención de una persona, el
repositorio SHALL sostener controles estructurales declarados en código: toda
acción de terceros de los workflows MUST pinearse por commit completo; todo
workflow SHALL declarar permisos mínimos y solo los jobs que publican SHALL
elevar los suyos; los jobs que publican o usan credenciales de publicación
SHALL correr en un entorno restringido a la rama principal; el pipeline MUST
rehusar publicar un commit no alcanzable desde la rama principal, una versión
que ya tenga tag o una versión no mayor que la mayor publicada; los tags de
versión MUST NOT poder moverse ni borrarse; y la rama principal MUST NOT poder
reescribirse ni borrarse.

#### Scenario: Acción sin pinear rechazada
- **IF** un workflow referencia una acción por tag o por rama
- **THEN** la verificación SHALL fallar nombrando el workflow y la acción

#### Scenario: Workflow sin permisos declarados rechazado
- **IF** un workflow no declara sus permisos
- **THEN** la verificación SHALL fallar nombrando el workflow

#### Scenario: Versión ya publicada no se republica
- **IF** la versión del commit ya tiene tag
- **THEN** el pipeline SHALL NOT publicar

#### Scenario: Versión menor no se publica
- **IF** la versión del commit no es mayor que la mayor ya publicada
- **THEN** el pipeline SHALL NOT publicar

#### Scenario: Commit fuera de la rama principal no se publica
- **IF** el commit no es alcanzable desde la rama principal
- **THEN** el pipeline SHALL NOT publicar

#### Scenario: Credenciales solo en el entorno de publicación
- **WHEN** un job usa una credencial de publicación
- **THEN** SHALL correr en el entorno restringido a la rama principal

### Requirement: Aviso a la web al publicar
El pipeline SHALL disparar el despliegue del sitio del producto, que vive en su
propio repositorio, después de publicar una release y nunca antes. La
credencial del aviso MUST limitarse a lanzar workflows de ese repositorio y MUST
NOT permitir escribir su contenido. El aviso SHALL NOT llevar datos de la
release: el sitio SHALL leer la release publicada de su fuente. IF el aviso
falla, THEN la release publicada MUST permanecer intacta y el aviso SHALL poder
repetirse por sí solo.

#### Scenario: La web se redespliega tras publicar
- **WHEN** la publicación de una release termina
- **THEN** el pipeline SHALL disparar el despliegue del sitio
- **AND** el disparo SHALL ocurrir después de la publicación

#### Scenario: Sin publicación no hay aviso
- **IF** el pipeline no publica una release
- **THEN** SHALL NOT disparar el despliegue del sitio

#### Scenario: Aviso fallido no deshace la release
- **IF** el disparo del despliegue falla
- **THEN** la release SHALL seguir publicada e intacta
- **AND** el aviso SHALL poder repetirse sin repetir la publicación

#### Scenario: El aviso no lleva datos
- **WHEN** el pipeline dispara el despliegue del sitio
- **THEN** el disparo SHALL NOT transportar versión, notas ni artefactos
