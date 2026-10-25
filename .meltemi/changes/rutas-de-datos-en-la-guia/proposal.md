# rutas-de-datos-en-la-guia

> Vía rápida (fast-forward): corrige una frase de `docs/plataformas.md` y
> pone un test que la ata al código. Sin código de producto. La halló la
> redacción de la página de privacidad del sitio (meltemi-web,
> `ide-privacy-statements`), que nombra las carpetas reales desde el
> 2026-10-24.

## Why

La guía de plataformas dice que en Windows el daemon guarda sus datos en
`%LOCALAPPDATA%\meltemi\`. No es así: el daemon resuelve sus carpetas con
`directories::ProjectDirs::from("", "", "meltemi")`
(`core/meltemi-client/src/paths.rs`), que en Windows es el perfil móvil,
`%APPDATA%\meltemi\data` para los datos y `%APPDATA%\meltemi\config` para la
configuración. En la máquina de desarrollo, donde el IDE corre a diario,
`%LOCALAPPDATA%\meltemi` no existe y `%APPDATA%\meltemi\data` sí.

La guía tampoco dice dónde queda la configuración (`config.toml`,
`permissions.toml`): quien quiera respaldar o borrar lo que Meltemi guarda
busca en el sitio equivocado y no encuentra la mitad. Y desde que el aviso de
privacidad del sitio nombra las carpetas reales, la guía y el aviso se
contradicen en lo único que un lector puede comprobar abriendo el explorador
de archivos.

Que se pudiera escribir mal sin que nada fallara es el defecto de fondo: la
guía nombra carpetas y ningún test las compara con lo que el código resuelve.

## What Changes

- **`docs/plataformas.md`, «Data paths»**: la carpeta de datos y la de
  configuración de cada plataforma, con la de Windows corregida.
- **Un test en `tui/tests/docs.rs`** que lee la guía, comprueba que nombra las
  seis carpetas y que las que el código resuelve en la plataforma donde corre
  el test son las que la guía nombra para ella. Corre en las tres plataformas
  del CI, así que cada una comprueba su propia fila.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `initial-docs`: + requisito «La guía de plataformas dice dónde guarda el
  producto».

## Impact

- Archivos: `docs/plataformas.md`, `tui/tests/docs.rs`.
- Cero dependencias; cero código de producto.
- Riesgo: el test depende de las variables `XDG_*` y de las de Meltemi en el
  entorno donde corre; el design dice cómo se acota.

## Fuera de alcance

- Mover las carpetas del producto: no se cambia nada de dónde guarda.
- La página de privacidad del sitio: ya dice las carpetas reales (meltemi-web).
