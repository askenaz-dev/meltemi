# repo-context Specification

## Purpose
TBD - created by archiving change gestion-contexto-repo. Update Purpose after archive.
## Requirements

### Requirement: Mapa del repositorio por contrato
El daemon SHALL exponer `repo/map` con el árbol del repositorio honrando las
reglas de ignorado de git (anidadas), con tamaños por archivo, y parámetros de
profundidad y límite. Todo truncado MUST declararse en la respuesta con la
cantidad de entradas omitidas, nunca de forma silenciosa.

#### Scenario: Ignorados fuera del mapa
- **WHEN** un cliente pide el mapa de un repo con `.gitignore`
- **THEN** la respuesta SHALL excluir lo ignorado
- **AND** SHALL incluir tamaños de los archivos listados

#### Scenario: Truncado declarado
- **WHERE** el repo excede el límite de entradas pedido
- **THEN** la respuesta SHALL marcar `truncated` con el número de omitidas

### Requirement: Expansión determinista de referencias
El daemon SHALL expandir en los prompts las referencias `@archivo` (contenido con
cerca e identificación de ruta) y `@carpeta/` (listado, no contenidos) de forma
determinista, con límites explícitos por archivo y por prompt; el exceso MUST
truncarse con una marca visible dentro del propio prompt enviado, y una
referencia inexistente MUST señalarse sin abortar el envío.

#### Scenario: Archivo inyectado con cerca
- **WHEN** un prompt contiene `@src/lib.rs`
- **THEN** el prompt enviado SHALL contener el archivo cercado e identificado por su ruta

#### Scenario: Exceso truncado con marca
- **IF** la expansión supera el límite por prompt
- **THEN** el contenido SHALL truncarse con una marca visible de truncado
- **AND** el envío SHALL proceder

#### Scenario: Referencia inexistente señalada
- **WHEN** un prompt referencia `@no/existe.rs`
- **THEN** el prompt SHALL incluir la señal de referencia no encontrada
- **AND** el turno SHALL continuar

### Requirement: Auditoría de expansiones
El registro JSONL de la sesión SHALL incluir, por cada prompt, qué referencias se
expandieron (rutas y bytes inyectados), de modo que el contexto entregado al
agente sea reconstruible.

#### Scenario: Expansión registrada
- **WHEN** un prompt con referencias se envía
- **THEN** el log SHALL registrar rutas y tamaños de lo expandido

### Requirement: Autocompletado de referencias en el compositor
El compositor de la TUI SHALL autocompletar `@` contra el mapa del repositorio
(por prefijo), dentro del contrato de captura de texto del shell, y MUST
funcionar bajo la línea base de accesibilidad.

#### Scenario: Completar una ruta
- **WHILE** el usuario escribe `@src/` en el compositor
- **THEN** el shell SHALL ofrecer las entradas del mapa que coinciden por prefijo

### Requirement: Metadirectorio de git fuera del mapa
El mapa del repositorio (`repo/map`) SHALL excluir el metadirectorio `.git`
en cualquier nivel del árbol, sin consumirlo del presupuesto de truncado,
mientras los directorios ocultos que sí son contexto (como `.meltemi/`)
SHALL seguir listándose.

#### Scenario: Metadirectorio de git fuera del mapa
- **WHEN** se construye el mapa de un repositorio git
- **THEN** ninguna entrada SHALL pertenecer a `.git`
- **AND** `.meltemi/` SHALL seguir presente en el mapa

### Requirement: Integridad del texto que rodea a las referencias
La expansión de referencias SHALL entregar el texto no referenciado del prompt
exactamente como el usuario lo escribió, carácter por carácter y sea cual sea
su alfabeto: ningún byte de una secuencia multibyte MUST reinterpretarse como
un carácter propio, ni al expandir ni al registrar la expansión en el log de la
sesión. `@@` SHALL producir un `@` literal sin registrar expansión alguna, y un
`@` que no abre una referencia SHALL viajar como texto.

#### Scenario: Prompt en español íntegro
- **WHEN** un prompt contiene caracteres fuera de ASCII y ninguna referencia
- **THEN** el prompt expandido SHALL contener exactamente los mismos caracteres que el escrito
- **AND** su recuento de caracteres SHALL ser el mismo

#### Scenario: Arroba doble literal
- **WHEN** un prompt contiene `@@` entre texto fuera de ASCII
- **THEN** el prompt expandido SHALL contener un único `@` en su lugar
- **AND** ninguna expansión SHALL registrarse
- **AND** el texto vecino SHALL conservarse intacto

#### Scenario: Referencia pegada a un carácter multibyte
- **WHEN** una referencia va inmediatamente precedida o seguida de un carácter fuera de ASCII
- **THEN** la referencia SHALL expandirse igual que rodeada de ASCII
- **AND** los caracteres vecinos SHALL conservarse intactos

### Requirement: Referencias a rutas fuera de ASCII
El token de una referencia SHALL admitir letras y dígitos de cualquier
alfabeto, además de `/`, `.`, `-` y `_`, de modo que un archivo cuyo nombre
lleva caracteres fuera de ASCII se resuelva como cualquier otro. La puntuación
fuera de ASCII de la prosa MUST NOT absorberse dentro del token: SHALL cerrarlo
igual que lo cierra un espacio, para que ninguna referencia se diagnostique
como inexistente sobre una ruta que el usuario no escribió.

#### Scenario: Ruta con carácter no ASCII resuelta
- **WHEN** un prompt referencia un archivo existente cuyo nombre lleva caracteres fuera de ASCII
- **THEN** el prompt expandido SHALL contener su contenido cercado e identificado por la ruta escrita
- **AND** la expansión SHALL registrarse como encontrada

#### Scenario: Puntuación no ASCII cierra el token
- **WHEN** una referencia va seguida de puntuación fuera de ASCII
- **THEN** el token SHALL terminar antes de la puntuación
- **AND** la puntuación SHALL viajar como texto del prompt
