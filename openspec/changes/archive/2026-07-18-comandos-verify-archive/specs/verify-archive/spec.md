## ADDED Requirements

### Requirement: Verificación por requisito con vínculo a tests
El verbo `verify` SHALL recorrer los requisitos de la change como checklist donde
cada escenario se verifica por vínculo a tests (mediante la convención de nombre
escenario→test, corriendo el comando de verificación del proyecto y registrando
el resultado) o por verificación manual con nota; el estado MUST persistir en la
change y ser reanudable.

#### Scenario: Escenario vinculado a su test
- **WHEN** verify corre sobre una change cuyo escenario tiene test homónimo
- **THEN** el resultado del test SHALL registrarse como verificación de ese escenario

#### Scenario: Verificación manual con nota
- **WHEN** el humano marca un escenario como verificado manualmente
- **THEN** el estado SHALL persistir con su nota
- **AND** el informe SHALL distinguir manual de vinculado-a-test

### Requirement: Archivado con fusión atómica
El verbo `archive` SHALL validar la change completa con el motor (estructura,
EARS y aplicación en seco de los deltas sobre la verdad viva), fundir todas las
capacidades con la aplicación de deltas del motor de forma atómica — o se
escriben todas o ninguna —, preservar la change en el histórico con fecha, y
regenerar la proyección de contexto. Todo conflicto MUST reportarse como
diagnóstico sin dejar la verdad viva a medias.

#### Scenario: Fusión total o nada
- **IF** la fusión de una capacidad falla tras haber preparado otras
- **THEN** la verdad viva SHALL quedar exactamente como estaba
- **AND** los diagnósticos SHALL señalar el conflicto

#### Scenario: Histórico y proyección
- **WHEN** un archivado concluye con éxito
- **THEN** la change SHALL quedar preservada en el histórico con fecha
- **AND** la proyección de contexto SHALL regenerarse

### Requirement: Gate de verificación para archivar
El archivado SHALL exigir la verificación completa de los requisitos de la change
o excepciones explícitas por requisito con justificación registrada; el informe
de archivado MUST listar verificados y exceptuados, y un requisito sin verificar
y sin excepción MUST bloquear el archivado.

#### Scenario: Bloqueo sin verificación
- **IF** un requisito no está verificado ni exceptuado
- **THEN** el archivado SHALL rehusarse indicando cuál falta

#### Scenario: Excepción justificada registrada
- **WHEN** el humano exceptúa un requisito con justificación
- **THEN** el archivado SHALL proceder
- **AND** la excepción SHALL constar en el informe y en el histórico

### Requirement: Verbos verify y archive operativos
Los subcomandos `verify` y `archive` SHALL ser operativos en TUI (flujo de la
vista Proyecto con la checklist) y en modo scriptable por pasos con `--json`, sin
esperas interactivas sin TTY; el archivado MUST advertir cuando el árbol de specs
tiene cambios locales sin commitear antes de fundir.

#### Scenario: Archivado con árbol sucio advertido
- **WHEN** se invoca archive con cambios sin commitear en las specs vivas
- **THEN** el verbo SHALL advertirlo y exigir confirmación explícita
