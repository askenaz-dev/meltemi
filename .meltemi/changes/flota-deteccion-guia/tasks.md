## 1. Registro y detección en dos capas

- [ ] 1.1 Claves aditivas del registro (`cli-bin`, `cli-candidate-paths`, `cli-install`, `adapter-install`, `legal-status`, `legal-note`), todas opcionales, con validación de que toda entrada con adaptador declara su CLI oficial _(Req: Detección en dos capas de las entradas con adaptador; design D1)_
- [ ] 1.2 Poblar la instantánea embebida: capas, comandos de instalación y estatus/nota legal por entrada, tomados del research interno de integración _(Req: Estatus legal de la ruta de integración sin maquillaje; design D1, D6)_
- [ ] 1.3 Detección por capa en `fleet.rs`: resolución independiente de cada capa y composición del estado único de cinco valores, dejando intacta la semántica de `detected` como punto de pilotaje _(Req: Detección en dos capas de las entradas con adaptador; design D2)_
- [ ] 1.4 Sondeo de Windows en dos conjuntos: lanzamiento (`.exe`/`.cmd`/`.bat`) y evidencia (añade `.ps1`), marcando el hallazgo solo-evidencia y sin devolverlo como objetivo de lanzamiento _(Req: Detección local pasiva de binarios; design D4)_
- [ ] 1.5 Composición del remedio por capa (frase + comando exacto del registro), sin ejecutar nunca el comando _(Req: Remedio por capa accionable en todas las superficies; design D5)_

## 2. Contrato

- [ ] 2.1 `proto/`: campos aditivos de `FleetAgent` (`layers[]` con `kind`/binario/`detected`/`binaryPath`/`evidenceOnly`/`install`, `installState`, `remedy`, `remedyCommand`, `legalStatus`, `legalNote`) y sus `$defs` en `proto/schemas/v1/fleet.schema.json` _(Req: Consulta fleet/list; design D3)_
- [ ] 2.2 Casos de conformidad: entrada de dos capas en sus cinco estados, entrada de una sola capa, omisión de los campos legales no declarados y rechazo de un `installState` desconocido _(Req: Consulta fleet/list; design D3)_

## 3. Superficies (paridad de render)

- [ ] 3.1 CLI `fleet`: render humano con estado compuesto, capas y remedio; `--json` sin cambios salvo los campos aditivos _(Req: Remedio por capa accionable en todas las superficies; design D8)_
- [ ] 3.2 TUI vista Flota: estado compuesto, capa faltante y remedio con glifo + palabra (jamás color solo), más la nota legal de la entrada _(Req: Remedio por capa accionable en todas las superficies; design D8)_
- [ ] 3.3 GUI: drawer de detalle de Flota con capas, comando copiable y nota legal según `design-system/` (nivel como pill, detección como dot + palabra, filas 32 px, celdas 8 px, radios 4/8, hairlines y un solo nivel de sombra), sin animar layout _(Req: Estatus legal de la ruta de integración sin maquillaje; design D8)_
- [ ] 3.4 Rehúso 2001: el diagnóstico y el remedio nombran la capa ausente y su comando de instalación _(Req: Remedio por capa accionable en todas las superficies; design D5)_
- [ ] 3.5 i18n: etiquetas de estado, de capa y de estatus legal en los catálogos ES/EN de TUI y GUI; el comando del remedio viaja como dato y no se traduce _(Req: Remedio por capa accionable en todas las superficies; design D8)_

## 4. Guía de agentes

- [ ] 4.1 `docs/agentes.md` (inglés): por entrada del registro, qué instala el usuario, cómo se detecta cada capa, nivel y su significado, perfiles para varias suscripciones con ejemplos completos, y solución de problemas por sistema operativo incluidos los shims de script en Windows _(Req: Guía de agentes verificada contra el registro; design D7)_
- [ ] 4.2 Enlace desde el README sin nombrar productos de terceros y alta de la guía en el lint de enlaces internos de la documentación _(Req: Guía de agentes verificada contra el registro; design D7)_
- [ ] 4.3 Test de coherencia registro↔guía en `tui/tests/docs.rs`: biyección de entradas, nivel y binarios por capa, y ejemplos de perfiles que parsean como configuración válida sin secretos en claro _(Req: Guía de agentes verificada contra el registro; design D7)_

## 5. Tests y calidad

- [ ] 5.1 Unit sobre fixtures por capa (solo CLI, solo adaptador, ambos, ninguno, solo evidencia): los cinco estados compuestos y el remedio de cada uno _(Req: Detección en dos capas de las entradas con adaptador; design D2, D5)_
- [ ] 5.2 Unit `cfg(windows)`: variantes de shim (`.cmd`, `.ps1`, ambas) distinguiendo evidencia de objetivo de lanzamiento _(Req: Detección local pasiva de binarios; design D4)_
- [ ] 5.3 E2e sobre repo fixture temporal con registro sustituido y binarios simulados por capa: `fleet --json` reporta capas, estado y remedio, y una sesión con el id sin capa de pilotaje rehúsa nombrando la capa; nunca contra la raíz del repo ni la red, con `mock-agent` _(Req: Remedio por capa accionable en todas las superficies; design D5, D8)_
- [ ] 5.4 `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las tres plataformas _(Req: Consulta fleet/list; constitución §7)_
