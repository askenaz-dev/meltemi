# spec-merge — Parser y aplicador de deltas

## ADDED Requirements

### Requirement: Parseo de un delta a operaciones estructuradas
El motor SHALL parsear un delta spec en operaciones estructuradas, agrupando cada `### Requirement:` bajo su cabecera de operación (`## ADDED/MODIFIED/REMOVED/RENAMED Requirements`), extrayendo el bloque completo del requisito en `ADDED`/`MODIFIED`, los campos `Reason` y `Migration` en `REMOVED`, y las líneas `FROM:` / `TO:` en `RENAMED`.

#### Scenario: Delta con operaciones mixtas
- **WHEN** el motor parsea un delta con secciones `ADDED` y `MODIFIED`
- **THEN** produce una operación por requisito, clasificada según su sección

#### Scenario: Requisito eliminado con motivo y migración
- **WHEN** el motor parsea una sección `REMOVED` con `**Reason**:` y `**Migration**:`
- **THEN** la operación de eliminación conserva ambos campos

### Requirement: Aplicación de requisitos añadidos
El motor SHALL añadir al spec vivo cada requisito de una operación `ADDED`, y SHALL emitir un diagnóstico cuando el nombre del requisito ya exista en el spec vivo.

#### Scenario: Añadir un requisito nuevo
- **WHEN** se aplica una operación `ADDED` con un nombre que no existe en el spec vivo
- **THEN** el spec fundido incluye el requisito nuevo al final

#### Scenario: Añadir un requisito ya existente
- **WHEN** se aplica una operación `ADDED` con un nombre que ya existe
- **THEN** el motor emite un diagnóstico de requisito añadido duplicado y no lo duplica

### Requirement: Aplicación de requisitos modificados
El motor SHALL reemplazar el bloque completo de un requisito existente con el de una operación `MODIFIED`, y SHALL emitir un diagnóstico cuando el requisito a modificar no exista.

#### Scenario: Modificar un requisito existente
- **WHEN** se aplica una operación `MODIFIED` sobre un requisito que existe
- **THEN** el spec fundido contiene la versión modificada en la posición del original

#### Scenario: Modificar un requisito inexistente
- **WHEN** se aplica una operación `MODIFIED` sobre un nombre que no existe
- **THEN** el motor emite un diagnóstico de requisito modificado inexistente

### Requirement: Aplicación de requisitos eliminados
El motor SHALL eliminar del spec vivo un requisito de una operación `REMOVED`, y SHALL emitir un diagnóstico cuando falten `Reason` o `Migration`, o cuando el requisito no exista.

#### Scenario: Eliminar con motivo y migración
- **WHEN** se aplica una operación `REMOVED` con `Reason` y `Migration` sobre un requisito existente
- **THEN** el spec fundido ya no contiene ese requisito

#### Scenario: Eliminar sin migración
- **WHEN** se aplica una operación `REMOVED` sin `Migration`
- **THEN** el motor emite un diagnóstico de eliminación sin motivo o migración

### Requirement: Aplicación de requisitos renombrados
El motor SHALL renombrar un requisito de una operación `RENAMED` de su `FROM` a su `TO`, y SHALL emitir un diagnóstico cuando el `FROM` no exista o el `TO` ya esté en uso.

#### Scenario: Renombrar un requisito
- **WHEN** se aplica una operación `RENAMED` cuyo `FROM` existe y cuyo `TO` no
- **THEN** el spec fundido contiene el requisito con el nuevo nombre

#### Scenario: Renombrar hacia un nombre en uso
- **WHEN** se aplica una operación `RENAMED` cuyo `TO` ya existe
- **THEN** el motor emite un diagnóstico de renombre hacia un nombre en uso

### Requirement: Fusión determinista sobre la verdad viva
La fusión de un delta sobre un spec vivo SHALL ser determinista: preserva el orden de los requisitos vivos, coloca los añadidos al final, y produce un spec fundido equivalente al que produce el archivado de la herramienta durante el bootstrap.

#### Scenario: Paridad con el archivado
- **WHEN** se funde el delta de un cambio ya archivado sobre el spec vivo previo
- **THEN** el spec fundido coincide, en requisitos y escenarios, con la verdad viva resultante
