# initial-docs — delta

## ADDED Requirements

### Requirement: La FAQ empieza por el diagnóstico
La primera respuesta de las preguntas frecuentes SHALL mandar a `meltemi doctor`,
y la plantilla de error SHALL pedir su salida.

#### Scenario: Lo primero que pide la FAQ es el diagnóstico
- **WHEN** corre la verificación de la documentación
- **THEN** SHALL fallar si la primera respuesta de la FAQ o la plantilla de error no nombran `meltemi doctor`
