# repo-context — delta

## ADDED Requirements

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
