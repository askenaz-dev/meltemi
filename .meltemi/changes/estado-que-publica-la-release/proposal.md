# estado-que-publica-la-release

> Vía completa (proposal → design → specs → tasks). Es la 3º de la tanda
> `entrega-sin-huecos`. El estado de cada capacidad del producto (núcleo,
> escritorio, compañero móvil, SDK) pasa a vivir en un archivo versionado de
> este repositorio, que un test ata a los hechos. Cada release lo publica como
> un asset atestado, `capabilities.json`, y el sitio deja de escribir su tabla
> a mano: la pinta desde ahí (`ide-state-from-release`, en meltemi-web).

## Why

La tabla «Capability status» de `/ide/` y `/es/ide/` decía el 2026-10-29 que
el compañero móvil estaba «especificado para la fase 3, nada publicado». Sin
embargo, `companero-movil` se había archivado el día anterior con 8/8 tareas y
7/7 en verify. Nadie mintió: la tabla es HTML en otro repositorio, sin nada que
la ate al código, y el código avanzó. `ide-state-as-it-is` la corrige a mano,
y ese arreglo va a envejecer como el anterior.

El sitio ya resolvió este problema una vez, para la versión: `ide-release.mjs`
lee la última release del IDE en cada despliegue y escribe la versión y la
fecha entre marcadores. Lo que no lee es qué contiene la release, porque la
release no lo dice en ningún formato que se pueda leer. Esta change hace que
lo diga.

## What Changes

- **`docs/capacidades.toml`**: una entrada por capacidad con
  - su id (`core`, `desktop`, `mobile`, `sdk`);
  - su estado (`published`, `in-development`, `not-yet`);
  - su nombre y su nota en inglés y en español;
  - los assets de release que la entregan, por su nombre estable;
  - las changes que cierran lo que le falta.
- **El archivo se ata a los hechos**, en un test de la suite que corre en CI:
  - `published` exige que `release.yml` produzca cada uno de sus assets;
  - `in-development` exige al menos una change abierta que cierre lo que falta;
  - `not-yet` prohíbe que alguna de sus changes esté archivada y que algún
    asset suyo se publique.
- **`meltemi-release capabilities`** valida el archivo y escribe
  `capabilities.json`: la versión, y por capacidad su estado, sus textos y sus
  assets, cada uno con `platformSigned` (hoy siempre `false`; lo enciende
  `firma-de-plataforma` cuando el pipeline verificó la firma).
- **La release lo publica**: el job `version` lo genera y lo sube como
  artefacto `meltemi-capabilities`. El job `publish` lo recoge con los demás,
  así que queda en el `SHA256SUMS` y bajo la misma atestación.
- **El estado de hoy**, escrito y verificado:
  - el núcleo, publicado;
  - el escritorio, en desarrollo (`herramientas-a-la-vista`,
    `firma-de-plataforma`, `distribucion-y-actualizacion`);
  - el compañero móvil, en desarrollo (`apk-en-la-release`,
    `firma-de-plataforma`);
  - el SDK, todavía no.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisito «El estado de cada capacidad viaja con
  la release».

## Impact

- Archivos: `docs/capacidades.toml` (nuevo), `core/meltemi-release/src/{lib.rs,
  main.rs}` y un test nuevo, `.github/workflows/release.yml` (un paso y un
  artefacto en `version`), `docs/release.md` (el asset en la tabla y qué
  significa).
- Cero dependencias nuevas: `toml` y `serde_json` ya están en el workspace.
  Hay que confirmar que `meltemi-release` puede usarlas (design D1).
- En meltemi-web, `ide-state-from-release` lee el asset. Mientras ninguna
  release lo traiga, el sitio conserva sus filas escritas.
- Riesgo: que el archivo diga «publicado» de algo que no lo está. El test lo
  impide para los assets; la nota la escribe una persona y se revisa en la PR,
  como cualquier texto.

## Fuera de alcance

- Que el sitio pinte otra cosa que esta tabla: la página de descargas y la
  portada siguen con sus propios textos.
- Métricas de uso, descargas o instalaciones: §9.
- Estados intermedios más finos («beta», «preview»): tres estados bastan para
  la tabla que existe, y un cuarto se propone cuando haga falta.
