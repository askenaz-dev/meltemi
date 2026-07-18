## 1. Compilador

- [x] 1.1 `meltemi-spec`: función pura de proyección (constitución + rumbo con inclusión + change activa) con salida determinista _(Req: Compilación determinista)_
- [x] 1.2 Mapa de destinos como datos versionados (base AGENTS.md + variantes; nombres de terceros solo en datos) _(Req: Destinos y variantes)_

## 2. Escritura gestionada

- [x] 2.1 Bloques gestionados con huella: parsing de marcadores, preservación byte a byte fuera de ellos, anexado si faltan, escritura atómica _(Req: Bloques gestionados)_
- [x] 2.2 Idempotencia verificada (sin cambios de fuente → sin escritura efectiva) _(Req: Bloques — Idempotencia)_

## 3. Contrato y superficies

- [x] 3.1 `proto/`: `methods::CONTEXT_PROJECT` + params/result (destinos escritos + huella) _(Req: Proyección bajo demanda)_
- [x] 3.2 Handler en `meltemid` y subcomando CLI `project` (humano + `--json`; gramática y mapeo del delta acumulativo) _(Modified: cli-contract)_
- [x] 3.3 Registrar `project` en la paleta y acción en la vista Proyecto de la TUI

## 4. Dogfooding

- [x] 4.1 Proyectar este repositorio: `AGENTS.md` pasa a bloque gestionado; retirar la nota de proyección manual _(Req: Dogfooding)_

## 5. Tests y calidad

- [x] 5.1 Unit: determinismo, reglas de inclusión, preservación con documentos adversos (marcadores duplicados/ausentes), idempotencia
- [x] 5.2 E2e: `context/project` contra fixture temporal con destinos múltiples; CLI `project --json` (un objeto)
- [x] 5.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
