# release-distribution — delta

## ADDED Requirements

### Requirement: Manifiestos de gestores publicados tras la release
Después de publicar una release y nunca antes, el pipeline SHALL actualizar los
manifiestos de los gestores de paquetes del proyecto con la versión publicada y
con los hashes tomados del `SHA256SUMS` publicado, verificado antes contra la
atestación de la release. Un manifiesto MUST NOT apuntar a un artefacto que la
release no publicó. IF la actualización de un gestor falla, THEN la release
SHALL permanecer publicada e intacta, los demás gestores SHALL intentarse
igual, y la actualización SHALL poder repetirse sola. WHERE el instalador de
escritorio de macOS no lleve firma de plataforma con notarización, su manifiesto
de Homebrew MUST NOT publicarse.

#### Scenario: Los manifiestos salen de lo publicado
- **WHEN** la publicación de una release termina
- **THEN** el pipeline SHALL actualizar cada manifiesto con la versión y con los hashes del `SHA256SUMS` publicado
- **AND** SHALL haber verificado ese `SHA256SUMS` contra la atestación antes de leerlo

#### Scenario: Sin publicación no hay manifiesto
- **IF** el pipeline no publica una release
- **THEN** SHALL NOT actualizar ningún manifiesto

#### Scenario: Un gestor que falla no deshace nada
- **IF** la actualización del manifiesto de un gestor falla
- **THEN** la release SHALL seguir publicada e intacta
- **AND** los demás gestores SHALL intentarse, y la actualización SHALL poder repetirse sin repetir la publicación

#### Scenario: Sin notarización no hay cask
- **IF** el DMG de la release no lleva firma de plataforma con notarización
- **THEN** el manifiesto de Homebrew del escritorio SHALL NOT publicarse
- **AND** el paso SHALL declarar por qué

### Requirement: La actualización la hace el gestor del usuario
Meltemi MUST NOT consultar por red si existe una versión nueva, desde el daemon
ni desde ningún cliente. La actualización SHALL quedar en manos del gestor por
el que el usuario instaló, y la documentación de releases SHALL decir el
comando de cada gestor junto a los instaladores directos.

#### Scenario: Ningún cliente HTTP en el árbol
- **WHEN** la auditoría de dependencias corre en la integración continua
- **THEN** SHALL rechazar cualquier pila HTTP o TLS en el workspace

#### Scenario: Los comandos de cada gestor están escritos
- **WHEN** se lee la documentación de releases
- **THEN** SHALL nombrar el comando de instalación y de actualización de cada gestor publicado
