# Design — rutas-de-datos-en-la-guia

## Context

`paths::data_dir()` y `paths::config_dir()` devuelven, salvo que
`MELTEMI_DATA_DIR` o `MELTEMI_CONFIG_DIR` las muevan, lo que
`directories::ProjectDirs::from("", "", "meltemi")` da en cada plataforma:

| | datos | configuración |
|---|---|---|
| Windows | `%APPDATA%\meltemi\data` | `%APPDATA%\meltemi\config` |
| macOS | `~/Library/Application Support/meltemi` | la misma |
| Linux | `$XDG_DATA_HOME/meltemi` o `~/.local/share/meltemi` | `$XDG_CONFIG_HOME/meltemi` o `~/.config/meltemi` |

La de Windows se leyó el 2026-10-24 en la máquina de desarrollo. Las de macOS
y Linux salen de lo que documenta el crate para esa llamada y las comprueba el
CI de cada plataforma con este test.

## Decisions

### D1. El test compara la guía con el código, en la plataforma donde corre

Dos comprobaciones. La primera, independiente del anfitrión: la guía contiene
las seis carpetas tal como se escriben para un lector. La segunda, del
anfitrión: las carpetas que `paths` resuelve aquí terminan como la guía dice
para esta plataforma. Una guía que miente sobre la plataforma que no es la
del CI se cae en el CI de esa plataforma, no en el local.

### D2. Se omite la segunda comprobación si el entorno mueve las carpetas

Si `MELTEMI_DATA_DIR` o `MELTEMI_CONFIG_DIR` están definidas, `paths` no
resuelve los valores por defecto y comparar no dice nada de la guía; el test
lo anota y sigue con la primera. En Linux, si `XDG_DATA_HOME` o
`XDG_CONFIG_HOME` están definidas, el valor esperado es el suyo más
`meltemi`.

### D3. La guía dice qué hay en cada carpeta por clase, no por archivo

Datos: registros de sesión y métricas locales, el índice de proyectos y los
registros del daemon. Configuración: ajustes y reglas de permisos. Los nombres
de archivo son del producto y pueden cambiar; las clases, no.

## Risks

- **Un entorno con rutas inusuales** (perfil redirigido en Windows, `HOME`
  distinto en un contenedor) hace fallar la segunda comprobación sin que la
  guía esté mal. Se acota comparando el final de la ruta y la presencia del
  perfil móvil, no la ruta entera.
