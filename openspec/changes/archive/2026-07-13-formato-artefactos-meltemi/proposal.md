# Propuesta: Formato canónico de los artefactos `.meltemi/`

## Why

El motor de specs de Fase 1 (`motor-specs-artefactos`, `motor-ears-deltas`) necesita un **formato canónico e inequívoco** que parsear, validar y fundir. Hoy hay una contradicción sin resolver: `meltemi.md` §5.1/§2.1 especifica palabras clave estructurales en español (`## AÑADIDO / MODIFICADO / ELIMINADO`, EARS `Cuando/Mientras`), mientras que las specs vivas reales de `fase-0` usan un híbrido (estructura en inglés + prosa en español). Antes de escribir una línea del motor hay que **fijar el formato** y alinear el documento fundacional con él. Decisión tomada: **híbrido EN/ES** (estructura y EARS en inglés, prosa descriptiva en español).

## What Changes

- **Definir el formato canónico de `.meltemi/`** como spec de la verdad viva (capacidad `artifact-format`): estructura de directorios y nombres de artefactos, cabeceras de operación de delta, canon de palabras clave EARS, sintaxis de requisito/escenario, y front-matter de los archivos de `rumbo/`.
- **Fijar la política bilingüe**: las palabras clave **estructurales y normativas van en inglés** (`## ADDED/MODIFIED/REMOVED Requirements`, `### Requirement:`, `#### Scenario:`, EARS `WHEN/WHILE/IF/WHERE` + `SHALL/MUST`), y la **prosa descriptiva va en español neutro**. Esto reconcilia la práctica de facto, el canon EARS y el ecosistema (OpenSpec/ACP) con el carácter español-primero del método (constitución §11, que se refiere a la prosa).
- **Enmendar `meltemi.md` §5.1 y §2.1** para reflejar el híbrido (hoy muestran cabeceras y EARS en español). Es una enmienda al documento fundacional ratificado → requiere aprobación del mantenedor al aplicar; `meltemi.md` pasaría a **v1.2** (ratificación pendiente).
- **Confirmar conformidad** de las specs vivas de `fase-0` (ya usan el híbrido; se normaliza cualquier residuo).

## Capabilities

### New Capabilities

- `artifact-format`: el formato canónico de los artefactos `.meltemi/` — nombres y estructura, cabeceras de delta (ADDED/MODIFIED/REMOVED/RENAMED), canon EARS, sintaxis de requisito y escenario, front-matter de rumbo, y la política de idioma estructura-EN / prosa-ES. Es el contrato que el motor de specs de Fase 1 implementará y validará.

### Modified Capabilities

<!-- Ninguna. No cambian requisitos de `daemon-lifecycle`, `acp-session`, `propose-flow` ni `method-bootstrap`; sus specs ya son conformes al híbrido. -->

## Impact

- **Documentos**: enmienda a `meltemi.md` (§5.1, §2.1, cabecera → v1.2). Ningún cambio en `.meltemi/constitution.md` (§11 ya habla de prosa) ni en `rumbo/`.
- **Verdad viva**: nueva capacidad `artifact-format` en `openspec/specs/`.
- **Código**: ninguno. El parser/validador es la siguiente change (`motor-specs-artefactos`); esta solo fija el contrato que aquél debe cumplir.
- **Gobernanza**: toca un documento ratificado → **aprobación del mantenedor** requerida al aplicar (regla `method-bootstrap`).
- **Fuera de alcance**: implementar el parser/validador, la detección de contradicciones y huecos, y el ciclo `/review`·`/verify`·`/archive` (changes posteriores de Fase 1).
