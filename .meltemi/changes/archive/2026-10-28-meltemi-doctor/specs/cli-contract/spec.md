# cli-contract — delta

## ADDED Requirements

### Requirement: Diagnóstico local con doctor
El subcomando `doctor` SHALL imprimir un informe compuesto en el cliente sin RPC
nuevo: la versión del cliente y la del daemon y si coinciden, el endpoint y si
responde, las carpetas de datos y de configuración, la configuración de
usuario, cada agente listo o con una capa a medias con su remedio, los no
detectados en una fila, `git` y `ssh`, y las últimas líneas de error del log del
daemon. Cada fila SHALL decir su estado con un glifo con gemelo ASCII y una
palabra, y su remedio cuando lo hay. `doctor` NO SHALL arrancar un daemon ni
ejecutar el binario de ningún agente, y ningún paso SHALL esperar sin tope.
Con `--json` el informe SHALL salir como JSON. El comando SHALL salir con `0`
si ninguna fila falla y con `14` si alguna falla.

#### Scenario: El diagnóstico funciona sin daemon y no arranca ninguno
- **WHEN** se corre `doctor` contra un endpoint donde nada responde
- **THEN** el informe SHALL salir entero, con la fila del daemon en falla y su remedio
- **AND** SHALL NOT haber arrancado un daemon
- **AND** el comando SHALL salir con `14`

#### Scenario: Cada problema dice su remedio
- **WHEN** la flota tiene un agente con una capa a medias
- **THEN** su fila SHALL decir el estado y el remedio que la flota declara

#### Scenario: El diagnóstico se lee como JSON
- **WHEN** se corre `doctor --json`
- **THEN** stdout SHALL ser un objeto JSON con cada fila, su id, su estado y sus hechos
