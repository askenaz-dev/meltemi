# verify-archive — delta

## ADDED Requirements

### Requirement: La fusión no arrastra la estructura del delta
El archivado SHALL fundir de cada requisito añadido o modificado solo su
bloque —cabecera, cuerpo y escenarios—, sin las cabeceras de sección del delta
(`## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`,
`## RENAMED Requirements`) ni lo que las sigue hasta el requisito siguiente.
Una cabecera de sección del delta que la verdad viva ya contenga SHALL
desaparecer en la siguiente fusión de esa capacidad.

#### Scenario: Un requisito añadido antes de otra sección no arrastra su cabecera
- **WHEN** se archiva un delta cuyo último requisito añadido va seguido de `## MODIFIED Requirements`
- **THEN** la spec viva SHALL contener ese requisito sin la cabecera
- **AND** todo requisito vivo que la fusión no toca SHALL seguir leyéndose como presente

#### Scenario: Una cabecera arrastrada antes se retira al fundir
- **WHERE** una spec viva contiene una cabecera de sección del delta entre dos requisitos
- **WHEN** se archiva una change que funde en esa capacidad
- **THEN** la spec viva resultante SHALL NOT contener esa cabecera
