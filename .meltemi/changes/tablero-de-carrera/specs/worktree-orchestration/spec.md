# worktree-orchestration — delta

## ADDED Requirements

### Requirement: La carrera consultable con procedencia

El resultado del diff por competidor SHALL declarar, por calle y como
campos aditivos opcionales, la procedencia de su último despacho — fuente
de resolución, perfil cuando aplique y nivel de integración —, la sesión
que corrió ese despacho, su estado de commit (sha cuando exista) y la base
fijada de esa calle. El resultado del despacho SHALL nombrar la sesión que
abrió. La omisión de todos los campos nuevos MUST serializar byte a byte
igual que antes de esta change: un cliente anterior no se rompe.

#### Scenario: La calle declara procedencia, sesión y estado

- **WHEN** un cliente consulta el diff de la carrera después de un despacho
- **THEN** cada calle despachada SHALL traer su fuente de resolución, su
  perfil cuando lo hubo, su nivel, la sesión del despacho y su estado de
  commit
- **AND** el resultado del despacho SHALL nombrar esa misma sesión

#### Scenario: Los campos aditivos no rompen al cliente anterior

- **WHEN** los campos nuevos se omiten por no existir procedencia registrada
- **THEN** la serialización del resultado SHALL ser idéntica a la forma
  previa a esta change
- **AND** una calle sin despacho registrado SHALL presentarse sin
  procedencia, nunca con procedencia inventada

#### Scenario: Bases divergentes visibles por calle

- **IF** dos calles de la misma tarea se crearon con bases distintas
- **THEN** cada calle SHALL declarar su propia base
- **AND** el resultado MUST NOT fundir las bases en una base única: cada
  calle conserva la suya
