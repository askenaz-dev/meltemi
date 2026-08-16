# acp-session — delta

## ADDED Requirements

### Requirement: El modo de la sesión queda escrito

Una sesión que declara modo SHALL registrarlo en su log al arrancar, y cada
decisión de permiso SHALL registrar bajo qué modo se tomó. Un histórico con
modos que no los registre no puede explicar sus propias decisiones.

#### Scenario: El arranque registra el modo

- **WHEN** una sesión arranca declarando modo
- **THEN** su log SHALL registrarlo

#### Scenario: Cada decisión dice bajo qué modo se tomó

- **WHEN** se resuelve una petición de permiso en una sesión con modo
- **THEN** el registro de la decisión SHALL nombrar el modo vigente
