## ADDED Requirements

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
