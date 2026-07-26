## ADDED Requirements

### Requirement: Procedencia verificable de la release
Cada release publicada SHALL llevar una atestación de build, emitida por el
workflow que la produjo, que ligue los artefactos publicados al repositorio, al
commit y al workflow de origen. La atestación MUST poder verificarse con un
comando publicado, y la documentación MUST declarar con precisión qué cubre la
atestación y qué no: WHERE la atestación se emita en un job que solo agrega
artefactos ya construidos, la documentación SHALL decir que atestigua esa
agregación y no cada paso de construcción. La atestación SHALL NOT presentarse
como sustituto de la firma del mantenedor, que responde una pregunta distinta.

#### Scenario: Procedencia publicada con la release
- **WHEN** el pipeline publica una release
- **THEN** los artefactos publicados SHALL quedar cubiertos por una atestación del workflow que los produjo
- **AND** la verificación SHALL ser posible con el comando publicado en la documentación

#### Scenario: Alcance de la atestación declarado sin exagerar
- **WHEN** la documentación describe la atestación
- **THEN** SHALL decir qué job la emite y qué cubre realmente
- **AND** SHALL NOT afirmar una cobertura de construcción que la atestación no tiene

#### Scenario: Registro público de la atestación declarado
- **WHEN** la atestación se registra en un log de transparencia público
- **THEN** la documentación SHALL declararlo, con qué queda registrado
- **AND** SHALL distinguirlo de telemetría: es metadato de build, nunca dato de usuario

## MODIFIED Requirements

### Requirement: Artefactos firmados con custodia documentada
Cada artefacto publicado SHALL acompañarse de su checksum y firma verificables,
y el procedimiento de custodia de la clave (generación, almacenamiento y
repudio) MUST estar documentado y en manos del mantenedor. La clave pública
—el ancla de confianza— MUST publicarse en el repositorio y MUST NOT tomarse de
la página de release que autentica, porque quien puede publicar una release puede
editar el texto que la acompaña. La documentación de custodia MUST declarar los
límites reales de la herramienta de firma elegida, y MUST NOT prometer
capacidades que esa herramienta no tenga. La clave privada MUST NOT residir en
este repositorio ni ser accesible a ningún job de integración continua.

#### Scenario: Verificación por el usuario
- **WHEN** un usuario descarga un binario de release
- **THEN** SHALL poder verificar checksum y firma con instrucciones publicadas

#### Scenario: Ancla de confianza fuera de la página que autentica
- **WHEN** un usuario busca la clave pública para verificar una firma
- **THEN** SHALL encontrarla en el repositorio, con su historial de cambios
- **AND** las instrucciones publicadas SHALL NOT remitir a la página de release como origen de la clave

#### Scenario: Límites de la herramienta declarados
- **WHEN** la documentación describe la custodia de la clave
- **THEN** SHALL declarar qué garantías la herramienta de firma no puede dar
- **AND** SHALL definir qué significa repudiar una clave cuando la herramienta carece de revocación
