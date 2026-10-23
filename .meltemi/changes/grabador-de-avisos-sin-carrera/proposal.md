# grabador-de-avisos-sin-carrera

> Vía rápida: los artefactos de una vez y sin deltas, porque no cambia ningún
> requisito ni el comportamiento del producto, solo un fixture de test. Gate
> del mantenedor dado el 2026-10-23 («fix the intermittent notices test on
> Windows (the .cmd appender that loses a line) through the project's method,
> verify it, commit and push»). Nace del CI de `release-con-el-push`.

## Why

`notices::tests::the_test_notice_ignores_the_floor_and_does_not_spend_it`
falla a ratos en el CI de Windows con `left: 1, right: 2`: esperaba dos avisos
grabados y vio uno. Falló en el CI de `a6a67b5` (run 39993652459, intento 1) y
en el de `8037fbc` (run 39991046157, intento 2), dos commits que no tocaban
código; relanzado, pasa. Un rojo que se arregla relanzando enseña a relanzar,
y entonces también se relanza el rojo de verdad.

La causa no está en el producto. `send` lanza el comando del aviso y vuelve sin
esperarlo (design D4 de `aviso-de-espera-opt-in`), así que el test, que pide
dos avisos de prueba seguidos, deja dos `cmd.exe` corriendo a la vez el mismo
`append.cmd`. En `cmd`, `>>` abre el archivo sin compartirlo: el segundo falla
con «The process cannot access the file because it is being used by another
process» y su línea se pierde. El stderr del comando va a null, por eso el
error no se veía. Reproducido fuera de Rust con el mismo script: 56 de 200
rondas de dos lanzamientos simultáneos perdieron una línea.

En Unix no pasa: `echo ... >> archivo` es una sola escritura con `O_APPEND`, y
el sistema la añade entera.

## What Changes

- El `append.cmd` que el test escribe en Windows toma un candado antes de
  añadir su línea: crea un directorio con `mkdir`, que se crea o falla de forma
  atómica, y reintenta hasta conseguirlo; añade la línea y borra el
  directorio. Con el candado, 0 de 500 rondas perdieron una línea.
- Un test nuevo del fixture: dos lanzamientos simultáneos del grabador dejan
  dos líneas, veinte rondas seguidas. Sin el candado falla casi siempre: con
  una pérdida de una ronda de cada cuatro, veinte rondas sin pérdida son menos
  de una vez en cien.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- Ninguna: el comportamiento del aviso no cambia, y no se toca ningún
  requisito ni escenario.

## Impact

- `core/meltemi-client/src/notices.rs`, solo su módulo de tests.
- Cero dependencias, cero RPC, cero cambios fuera de los tests.

## Fuera de alcance

- `session_verbs::following_reading_and_cancelling_end_to_end` («the cancelled
  session is gone»), el otro test que falló en Windows el mismo día: tiene otra
  causa y merece otra change.
- La fragilidad bajo carga de `e2e_agente_por_sesion.rs` que anotó
  `aviso-de-espera-opt-in`.
