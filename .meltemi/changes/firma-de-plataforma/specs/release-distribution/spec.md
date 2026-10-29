# release-distribution — delta

## MODIFIED Requirements

### Requirement: Artefactos firmados con custodia documentada
Cada artefacto publicado SHALL acompañarse de su checksum y de una firma
verificable emitida por el workflow que lo publicó, sin clave de firma de
release de largo plazo. La custodia documentada de esa firma SHALL ser la del
repositorio y la del workflow: la documentación MUST declarar qué identidad
firma, qué controles la protegen y qué no puede garantizar, y MUST declarar que
una cuenta con permiso de escritura puede publicar una release que verifica. El
ancla de confianza SHALL ser la identidad del workflow firmante, y las
instrucciones publicadas MUST NOT tomarla de la página de release que
autentica. MUST NOT existir clave privada de firma de releases en el
repositorio, en la integración continua ni en máquina alguna. Las firmas de
plataforma de los instaladores y del APK SHALL admitirse solo con su custodia
escrita en la documentación de releases (qué clave, dónde vive, quién la
custodia, cuándo caduca, cómo se rota y cómo se revoca); su material secreto
MUST vivir en un entorno de publicación limitado a la rama principal o en un
servicio de firma, y MUST NOT ser accesible a ningún job que construya. WHERE
una release publicada resulte comprometida o defectuosa, el repudio SHALL
consistir en publicar la versión siguiente y declarar la anterior, con fecha,
en sus notas y en la documentación de releases.

#### Scenario: Verificación por el usuario
- **WHEN** un usuario descarga un binario de release
- **THEN** SHALL poder verificar checksum y firma con instrucciones publicadas
- **AND** la firma SHALL verificar contra la identidad del workflow que publicó la release

#### Scenario: Límite de la firma del workflow declarado
- **WHEN** la documentación describe la firma de las releases
- **THEN** SHALL declarar que una cuenta con permiso de escritura puede publicar una release que verifica
- **AND** SHALL nombrar los controles que protegen esa ruta

#### Scenario: Ninguna clave de firma de release que custodiar
- **WHEN** se inspeccionan el repositorio, los workflows y la documentación de releases
- **THEN** SHALL NOT encontrarse clave de firma de releases ni instrucciones para usar una

#### Scenario: Clave de plataforma solo con custodia escrita
- **IF** un workflow usa una clave o un servicio de firma de plataforma que la documentación de releases no inventaría
- **THEN** la verificación SHALL fallar nombrando el secreto o el servicio

#### Scenario: Ningún job que construye ve una clave de plataforma
- **IF** un job que hace checkout del código o corre una herramienta de build usa el entorno de firma o sus secretos
- **THEN** la verificación SHALL fallar nombrando el job

#### Scenario: Repudio de una release
- **IF** una release publicada resulta comprometida o defectuosa
- **THEN** el procedimiento documentado SHALL publicar la versión siguiente
- **AND** SHALL declarar la release repudiada, con fecha, en sus notas y en la documentación de releases

## ADDED Requirements

### Requirement: Firma de plataforma de los instaladores y del APK
WHERE la firma de plataforma de una plataforma esté activada, su instalador
SHALL firmarse en un job aislado que recibe el artefacto ya construido, y SHALL
verificarse antes de publicar, en un job sin secretos, contra la identidad
esperada que el repositorio fija; una firma válida de otra identidad MUST
fallar. IF la firma o la verificación de una plataforma activada fallan, THEN
la release MUST NOT publicarse ni sustituir el artefacto por uno sin firmar.
WHERE la firma de una plataforma no esté activada, su instalador SHALL
publicarse como hasta ahora, con la ausencia de firma declarada, y el APK MUST
NOT publicarse. Los builds de integración MUST NOT usar la firma de plataforma.
El estado publicado de las capacidades SHALL declarar firmado solo lo que esta
verificación aceptó.

#### Scenario: Instalador firmado y verificado antes de publicar
- **WHEN** la firma de una plataforma está activada y el pipeline publica una release
- **THEN** su instalador SHALL haberse firmado en un job que no construye
- **AND** SHALL haberse verificado contra la identidad fijada antes de la publicación

#### Scenario: Una firma de otra identidad falla
- **IF** el artefacto firmado verifica, pero con una identidad distinta de la fijada en el repositorio
- **THEN** la verificación SHALL fallar nombrando la identidad esperada y la encontrada
- **AND** la release SHALL NOT publicarse

#### Scenario: Activada y sin firma verificada no se publica
- **IF** la firma de una plataforma activada falla o no verifica
- **THEN** la release SHALL NOT publicarse
- **AND** SHALL NOT publicar ese instalador sin firmar en su lugar

#### Scenario: Sin activar se publica como hoy
- **WHEN** la firma de una plataforma no está activada
- **THEN** su instalador SHALL publicarse sin firma de plataforma con el aviso declarado
- **AND** el APK SHALL NOT publicarse

#### Scenario: Los builds de integración no firman
- **WHEN** corre la ruta de integración
- **THEN** SHALL NOT usar el entorno de firma ni sus secretos

#### Scenario: El estado publicado dice lo que se firmó
- **WHEN** el pipeline publica el estado de las capacidades
- **THEN** SHALL declarar con firma de plataforma solo los assets que la verificación aceptó
