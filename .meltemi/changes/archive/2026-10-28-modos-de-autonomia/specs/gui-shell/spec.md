# gui-shell — delta

## ADDED Requirements

### Requirement: El modo de la sesión es visible y elegible

El lanzador SHALL permitir elegir el modo antes de arrancar, y la sesión SHALL
mostrar el modo vigente junto al compositor. WHERE la sesión corre sobre el árbol
del usuario sin worktree, el modo semi NO SHALL presentarse como contención: la
superficie SHALL nombrar el ámbito real.

#### Scenario: El modo se elige al lanzar y se ve en la sesión

- **WHEN** se lanza una sesión eligiendo modo
- **THEN** la sesión SHALL mostrar ese modo junto al compositor

#### Scenario: Semi sin worktree dice cuál es su ámbito real

- **WHERE** la sesión corre sobre el árbol del usuario sin worktree
- **THEN** la superficie NO SHALL presentar semi como contención al worktree
