# fleet-catalog — delta

## ADDED Requirements

### Requirement: El perfil declara su modelo y su esfuerzo

Un perfil de lanzamiento SHALL poder declarar modelo y esfuerzo por defecto.
Lo declarado explícitamente en la sesión SHALL prevalecer sobre el default del
perfil, y un perfil que no declare nada NO SHALL imponer nada.

#### Scenario: La sesión pisa el default del perfil

- **WHERE** un perfil declara un modelo
- **WHEN** la sesión declara otro
- **THEN** SHALL regir el de la sesión

#### Scenario: Un perfil sin declaración no impone nada

- **WHERE** un perfil no declara modelo ni esfuerzo
- **THEN** la sesión SHALL correr como si el perfil no existiera en ese punto
