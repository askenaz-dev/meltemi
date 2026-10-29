# release-distribution — delta

## ADDED Requirements

### Requirement: Una release que marca un hito probado publica su informe
WHERE una release marca un hito que una persona probó de punta a punta, las
notas de esa release SHALL enlazar el informe de la corrida junto a las changes
que archiva, y el enlace SHALL sobrevivir a reescribir la sección de su versión.
La versión de esa release SHALL publicarla un gesto del mantenedor —escribirla,
o mergear la petición de release que la propone—; un agente MUST NOT escribirla
en la rama principal salvo que el mantenedor se lo pida con palabras.

#### Scenario: Las notas enlazan el informe
- **WHEN** el mantenedor escribe la sección de una versión nombrando el informe de la corrida
- **THEN** la sección SHALL abrir con el enlace al informe y listar las changes que archiva
- **AND** reescribir la sección sin nombrarlo SHALL conservar el enlace

#### Scenario: Un informe que no existe no se enlaza
- **WHEN** el mantenedor nombra un informe que no está en el repositorio
- **THEN** la escritura SHALL rehusarse sin tocar el `CHANGELOG.md`
