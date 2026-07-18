## 1. Lanzadores por nivel

- [x] 1.1 Interfaz de lanzamiento por nivel en `meltemid` (L1 actual tras la interfaz; sesión declara nivel) _(Req: Semántica operativa)_
- [x] 1.2 L2: lanzamiento vía adaptador declarado (detección pasiva del adaptador; error 2001 si falta) _(Req: Lanzamiento por adaptador)_
- [x] 1.3 L3: ejecución headless con guardarraíles obligatorios (dir acotado + controles nativos desde datos + denegaciones del motor aplicadas) y mapeador de salida estructurada _(Req: Guardarraíles del nivel 3)_
- [x] 1.4 L4: sesión de tipo externo (sin proceso) ligada a la proyección _(Req: Integración por artefactos)_

## 2. Datos y catálogo

- [x] 2.1 Extender el registro de datos: adaptador (binario+args), invocación headless, mapeo de salida, controles nativos configurables — poblado desde el research interno
- [x] 2.2 `verifiedLevel` + fecha en `fleet/list` desde resultados persistidos; distinción declarado/verificado en la vista Flota y CLI _(Req: Nivel verificado en el catálogo)_

## 3. Conformidad

- [x] 3.1 Mocks por nivel: mock-adapter (puentea a mock-agent) y mock-headless (emite JSONL guionado)
- [x] 3.2 Suite `conformance` con criterios pasa/no-pasa por nivel; nombres de tests desde los escenarios; persistencia del resultado _(Req: Suite de conformidad)_
- [x] 3.3 Opt-in manual para agentes reales (`MELTEMI_CONFORMANCE_REAL=1`), documentado y excluido de CI

## 4. Tests y calidad

- [x] 4.1 E2e por nivel contra daemon efímero: L2 sesión completa vía mock-adapter; L3 rehúsa sin guardarraíles y mapea salida; L4 sin subprocesos
- [x] 4.2 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes en el workspace
