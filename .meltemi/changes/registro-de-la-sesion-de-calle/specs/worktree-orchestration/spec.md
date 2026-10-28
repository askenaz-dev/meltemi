# worktree-orchestration — delta

## ADDED Requirements

### Requirement: La raíz gestionada excluida desde el primer worktree
El primer worktree gestionado que el daemon crea en un proyecto, sea de tarea o
de taller, SHALL dejar la raíz gestionada excluida del estado de git del árbol
principal por vía local, sin modificar el `.gitignore` versionado del usuario,
y la exclusión SHALL escribirse una sola vez.

#### Scenario: Un despacho no ensucia el árbol principal
- **WHEN** se despacha una tarea en un proyecto que nunca tuvo un taller
- **THEN** la raíz gestionada SHALL quedar en la exclusión local de git
- **AND** el estado de git del árbol principal SHALL NOT mostrar el worktree de la tarea
