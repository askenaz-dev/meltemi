# release-distribution — delta

## ADDED Requirements

### Requirement: El estado de cada capacidad viaja con la release
El estado de cada capacidad que el producto declara SHALL vivir en un archivo
versionado del repositorio, con su nombre y su nota en inglés y en español, los
assets de release que la entregan y las changes que cierran lo que le falta, y
la verificación SHALL atarlo a los hechos. Una capacidad publicada MUST tener
cada uno de sus assets producido por el pipeline de release. Una capacidad en
desarrollo MUST nombrar al menos una change abierta. Una capacidad que todavía
no existe MUST NOT tener archivada ninguna de sus changes ni publicar un asset.
Cada release SHALL publicar ese estado como un asset de nombre estable, dentro
del `SHA256SUMS` que cubre su atestación, y SHALL declarar como firmado con
firma de plataforma solo lo que el pipeline verificó.

#### Scenario: Publicada exige su artefacto
- **IF** una capacidad dice estar publicada y el pipeline de release no produce uno de sus assets
- **THEN** la verificación SHALL fallar nombrando la capacidad y el asset

#### Scenario: En desarrollo nombra su change abierta
- **IF** una capacidad dice estar en desarrollo y ninguna de sus changes está abierta
- **THEN** la verificación SHALL fallar nombrando la capacidad

#### Scenario: Todavía no con su change archivada falla
- **IF** una capacidad dice que todavía no existe y una de sus changes está archivada o uno de sus assets se publica
- **THEN** la verificación SHALL fallar nombrando la capacidad y la change o el asset

#### Scenario: Un texto en un solo idioma falla
- **IF** una capacidad no tiene nombre y nota en inglés y en español
- **THEN** la verificación SHALL fallar nombrando la capacidad y el idioma que falta

#### Scenario: La release publica el estado atestado
- **WHEN** el pipeline publica una release
- **THEN** el estado de las capacidades SHALL constar como un asset de nombre estable en esa release
- **AND** su checksum SHALL estar en el `SHA256SUMS` que cubre la atestación

#### Scenario: Sin firma verificada no se declara firmado
- **WHEN** el pipeline genera el estado sin haber verificado una firma de plataforma
- **THEN** cada asset SHALL declararse sin firma de plataforma
