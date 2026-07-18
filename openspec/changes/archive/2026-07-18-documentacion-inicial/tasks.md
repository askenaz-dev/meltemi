## 1. Documentos raíz

- [x] 1.1 README (secciones mínimas, estado honesto, sin terceros) + LEEME.md espejo _(Req: README)_ — el lint verifica secciones, ausencia de nombres de terceros y el enlace al espejo.
- [x] 1.2 Quickstart por plataforma de cero al primer propose revisado _(Req: Quickstart verificado)_ — `docs/quickstart.md`, con nota de plataforma para el paso Windows/git-bash.

## 2. Generación y verificación

- [x] 2.1 Generador de referencia CLI desde gramática+taxonomía; verificación de frescura en pipeline _(Req: Referencia generada)_ — `cli::reference()` genera desde `USAGE` + `exit::EXIT_CODES` (fuente única); `docs/referencia-cli.md` generado (example `gen_cli_ref`) y un test de frescura falla si la gramática cambia sin regenerar. El keymap-como-dato de la TUI se genera con el shell interactivo (en curso).
- [x] 2.2 Verificación del quickstart en CI contra binarios (pasos scriptables) _(Req: Quickstart — CI)_ — el test corre el binario `meltemi` real (`version`, `help`) y falla ante salida divergente.

## 3. Esqueleto docs/

- [x] 3.1 Estructura navegable: arquitectura, método SDD, accesibilidad, plataformas (incluye H6 git-bash, rutas por SO, túnel SSH) _(Req: Notas de plataforma)_

## 4. Calidad

- [x] 4.1 Lint de docs (presencia + secciones + enlaces internos válidos + frescura de la referencia + quickstart contra binario) y verificación completa en verde
