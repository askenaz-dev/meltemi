# initial-docs — delta

## ADDED Requirements

### Requirement: Guion de la prueba de un desarrollador
El repositorio SHALL publicar un guion que una persona sigue desde los assets de
una release, sin leer el repositorio, para probar el producto de punta a punta
con sus propios agentes. Cada paso SHALL decir qué hacer, qué se espera ver,
dónde queda registrado y qué desviación conocida puede aparecer, y el guion
SHALL traer la plantilla del informe de la corrida. La documentación CI SHALL
comprobar que cada verbo de la CLI que el guion nombra existe en el binario
construido y SHALL correr sus pasos locales contra él.

#### Scenario: El guion se sigue desde los assets
- **WHEN** una persona abre el guion de la prueba
- **THEN** el primer paso SHALL instalar desde los assets de una release, no compilar
- **AND** cada paso SHALL decir qué se espera ver, dónde queda registrado y su desviación conocida

#### Scenario: Los verbos del guion existen en el binario
- **WHEN** la documentación CI corre
- **THEN** cada verbo de `meltemi` que el guion nombra SHALL existir en la ayuda del binario construido
- **AND** sus pasos locales SHALL correr contra ese binario
