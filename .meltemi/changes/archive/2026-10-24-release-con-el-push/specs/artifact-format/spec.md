## MODIFIED Requirements

### Requirement: Reglas de contenido de delta
Un delta `MODIFIED` SHALL incluir el bloque completo del requisito actualizado; un delta `REMOVED` SHALL incluir los campos **Reason** y **Migration**; un delta `RENAMED` SHALL usar el formato `FROM:` / `TO:`. Un bloque `REMOVED` nombra un requisito que deja de existir y no es un requisito: SHALL NOT exigírsele escenario ni verbo normativo, y toda validación del motor —la de una change y la del árbol `.meltemi/`— SHALL aplicarle la misma regla.

#### Scenario: Requisito modificado con bloque parcial
- **WHEN** un delta `MODIFIED` incluye solo un fragmento del requisito en lugar del bloque completo
- **THEN** el motor lo reporta como delta inválido

#### Scenario: Requisito eliminado sin migración
- **WHEN** un delta `REMOVED` omite el campo **Migration**
- **THEN** el motor lo reporta como delta inválido

#### Scenario: Requisito eliminado con razón y migración
- **WHEN** un delta `REMOVED` incluye **Reason** y **Migration** y ningún escenario
- **THEN** el motor SHALL aceptarlo tanto al validar la change como al validar el árbol
- **AND** SHALL NOT reportarle falta de escenario ni de verbo normativo
