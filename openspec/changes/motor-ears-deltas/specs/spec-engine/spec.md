# spec-engine — Validación EARS (ampliación)

## ADDED Requirements

### Requirement: Validación de marcador EARS en escenarios
El motor SHALL emitir un diagnóstico cuando un escenario no contenga ningún paso con un marcador EARS reconocido (`WHEN`, `WHILE`, `IF`, `THEN` o `WHERE`), señalando la línea del escenario.

#### Scenario: Escenario sin marcador EARS
- **WHEN** el motor valida un escenario cuyos pasos no usan ningún marcador EARS reconocido
- **THEN** emite un diagnóstico de escenario sin marcador EARS con su ubicación

#### Scenario: Escenario con WHEN y THEN
- **WHEN** el motor valida un escenario con un paso `WHEN` y un paso `THEN`
- **THEN** no emite ningún diagnóstico de marcador EARS

### Requirement: Validación de verbo normativo en requisitos
El motor SHALL emitir un diagnóstico cuando la descripción de un requisito no contenga el verbo normativo `SHALL` ni `MUST`, señalando la línea del requisito.

#### Scenario: Requisito no normativo
- **WHEN** el motor valida un requisito cuya descripción no usa `SHALL` ni `MUST`
- **THEN** emite un diagnóstico de requisito sin verbo normativo

#### Scenario: Requisito normativo
- **WHEN** el motor valida un requisito cuya descripción usa `SHALL`
- **THEN** no emite ningún diagnóstico de verbo normativo
