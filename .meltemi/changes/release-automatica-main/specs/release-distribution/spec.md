## ADDED Requirements

### Requirement: Cada push aprobado publica su SHA
Release MUST validar por API un CI completado con success, event push, head_branch
main, workflow ci.yml y repositorio origen idéntico antes de ejecutar su código.

#### Scenario: CI de una PR no publica
- **WHEN** la ejecución procede de pull_request, fork, otra rama o CI fallido
- **THEN** no se reserva tag ni se publica.

#### Scenario: Recuperación valida el CI
- **WHEN** se solicita workflow_dispatch con ci_run_id
- **THEN** se aplica exactamente el mismo control y se empaqueta su head_sha.

### Requirement: Identidad estable de release automática
Cada SHA aprobado MUST reservar un único número patch y tag inmutable; todos los
jobs MUST usar ese SHA y versión sin escribir commits a main.

#### Scenario: Reintento conserva versión
- **WHEN** falla la construcción o un upload después de reservar
- **THEN** reintentar reutiliza tag, versión y base de notas.

#### Scenario: Fuente y binario son reproducibles
- **WHEN** se publica
- **THEN** el manifiesto atestado identifica SHA, CI, versión declarada y estampada.

#### Scenario: Reintento antiguo no retrocede latest
- **WHEN** ya existe una release más nueva o de un descendiente
- **THEN** la recuperación completa su release sin cambiar latest.

#### Scenario: Pushes rápidos conservados
- **WHEN** varios pushes esperan CI o publicación
- **THEN** queue max conserva hasta 100 pendientes y se documenta el límite externo.
