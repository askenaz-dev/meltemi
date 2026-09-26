# integration-levels — delta

## ADDED Requirements

### Requirement: Corrida integrada reproducible contra los CLIs reales
El repositorio SHALL ofrecer una corrida integrada contra los CLIs reales
instalados, manual y por opt-in, que levante un daemon aislado —su endpoint,
sus datos y su configuración propios, con el arranque pedido expresamente en
ese endpoint— sin leer ni tocar jamás los del usuario; que ejerza sobre un
repositorio fixture temporal la misma matriz de verbos por cada agente listo
(sesión de lectura, escritura sin regla, escritura con regla, propuesta,
despacho en worktree, dirección y cancelación); que mida lo que el producto
registra, sin estimar nada; y que escriba su informe con fecha y versiones en
`docs/qa/`. Los agentes del catálogo que no estén instalados SHALL declararse
como no medidos. La corrida MUST NOT ejecutarse en CI.

#### Scenario: La corrida no toca el daemon del usuario
- **WHEN** se corre la corrida integrada
- **THEN** SHALL usar un endpoint, un directorio de datos y uno de configuración propios, y pedir expresamente el arranque en ese endpoint
- **AND** NO SHALL leer ni escribir los del usuario

#### Scenario: Cada agente listo pasa por la misma matriz
- **WHEN** la corrida encuentra agentes listos
- **THEN** cada uno SHALL ejercer los siete verbos de la matriz sobre el fixture
- **AND** el informe SHALL registrar por verbo su resultado, su duración y los permisos decididos

#### Scenario: Lo no instalado se declara
- **WHEN** un agente del catálogo no está instalado
- **THEN** el informe de la corrida SHALL listarlo como no medido

#### Scenario: La corrida no corre en CI
- **WHEN** se revisan los workflows del repositorio
- **THEN** ninguno SHALL ejecutar la corrida integrada
