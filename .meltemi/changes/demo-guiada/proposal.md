# demo-guiada

> Vía completa (proposal → design → specs → tasks): un verbo nuevo, el agente
> simulado viajando en los instaladores bajo presupuesto, y una experiencia
> de primer uso que no toca ningún proyecto del usuario. Nace del comparativo
> con Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brecha P3). Después de `antes-de-anunciar` y de `senales-de-vida`.

## Why

El tiempo desde instalar hasta la primera sesión gobernada es la métrica
que decide si alguien se queda. Hoy exige tener un agente instalado y
autenticado, un proyecto y una idea. Orca lo resuelve con su landing en
movimiento; Meltemi puede resolverlo con lo que ya tiene y ningún
competidor tiene: un agente simulado propio (`core/mock-agent`), el mismo
que corre en CI y el que produce las capturas del sitio.

`meltemi demo` arranca una sesión gobernada de verdad —permisos que llegan
a la bandeja, un checkpoint, un diff, una carrera de dos— sobre un fixture
temporal y con el agente simulado, etiquetada «simulado» en cada superficie
para que nadie confunda la demostración con un proveedor.

## What Changes

- **`meltemi demo`** y «Ver un ejemplo» en el onboarding de la GUI: crean
  un fixture en el directorio temporal, arrancan el agente simulado con un
  guion (propuesta, permiso, edición, carrera de dos calles) y abren la
  sesión; al terminar, un enlace a «ahora con tu agente».
- **El agente simulado viaja en los instaladores** como binario aparte,
  bajo `MELTEMI_TUI_BUDGET_BYTES` y el presupuesto del instalador; si no
  cabe, la demo se descarga como paquete aparte desde la release y lo dice.
- **Etiqueta «simulado»** en la flota, en la sesión, en la barra de estado y
  en el log de la sesión demo; el registro de proyectos no guarda el
  fixture.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `cli-contract`: + `demo`.
- `gui-shell` y `tui-shell`: + «Ver un ejemplo» y la etiqueta.
- `release-distribution`: + el agente simulado como artefacto, con su
  presupuesto.
- `fleet-catalog`: + la entrada «simulado» visible solo durante la demo.

## Impact

- Archivos: `tui/src/`, `core/mock-agent` (guion), `desktop/ui/src/lib/
  components/Onboarding.svelte`, `.github/workflows/{release,build}.yml`,
  `docs/quickstart.md`.
- Cero dependencias nuevas; el tamaño se mide antes de prometer.
- Riesgo: un usuario que crea que la demo es un agente real; la etiqueta es
  requisito, no adorno.

## Fuera de alcance

- Un tutorial interactivo paso a paso: el guion demuestra; la FAQ enseña.
