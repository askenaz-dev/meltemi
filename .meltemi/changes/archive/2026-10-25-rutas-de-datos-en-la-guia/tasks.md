# Tasks — rutas-de-datos-en-la-guia

## 1. La guía y su test

- [x] 1.1 `docs/plataformas.md`, «Data paths»: la carpeta de datos y la de
  configuración de las tres plataformas (design D1, D3) — gate: la guía las
  nombra y ninguna es `%LOCALAPPDATA%`
  <!-- 2026-10-24: datos y configuración de las tres plataformas, con la clase
  de lo que guarda cada carpeta. -->
- [x] 1.2 `tui/tests/docs.rs`: el test de los dos escenarios, con la segunda
  comprobación acotada por el entorno (design D2) — gates: verde en esta
  máquina, y rojo cuando se rompe cada carpeta de la guía en una copia
  <!-- 2026-10-24: rojo contra la guía vieja («the guide does not name
  %APPDATA%\meltemi\data»); verde con la corrección; cinco roturas de la guía
  (las dos de Windows, la de macOS y las dos de Linux), todas detectadas y la
  guía restaurada tras cada una. La segunda comprobación, la del código, pasó
  en Windows; macOS y Linux las comprueba el CI. -->

## 2. Cierre

- [x] 2.1 `meltemi validate rutas-de-datos-en-la-guia` y `verify` limpios, con
  daemon aislado; `cargo fmt --check`, clippy `-D warnings` y el test; la rama
  llega a `main` con `git merge --no-ff`, se empuja, y el CI de `main` queda
  verde en las tres plataformas
  <!-- 2026-10-24: `validate` limpio y `verify` 2/2 con daemon aislado; fmt limpio;
  clippy `-D warnings` de `meltemi` con sus tests en Windows y con `--target
  x86_64-linux-android`; los 15 tests de `docs` verdes. Merge e3075f9,
  empujado; el CI de `main` (CI, Build y Release) verde en las tres
  plataformas. -->
