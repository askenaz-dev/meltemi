# meltemi-doctor

> Vía rápida (fast-forward): un verbo del CLI compuesto sobre RPC y
> comprobaciones locales que ya existen, con su panel en la GUI y su entrada
> en la TUI. Ningún RPC nuevo. Nace del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, hallazgo P-V1 del
> crítico).

## Why

Lo que costó una tarde diagnosticar el 2026-09-24 cabe en una pantalla:
qué versión de cliente y de daemon hablan, por qué endpoint, si el pipe
existe, qué agentes están instalados y en qué versión frente al rango que
soporta cada adaptador, si `ssh` está en el PATH, si en Windows el servidor
OpenSSH está presente, si un shim es solo `.ps1`, dónde están los datos y
los logs. Hoy esa información está repartida entre `meltemi status`,
`meltemi fleet`, el log del daemon y la cabeza del usuario; lo más cercano
es «Copiar diagnóstico» en el banner de la GUI
(`desktop/ui/src/lib/messages.ts`), que solo aparece cuando algo ya falló.

Cuando exista comunidad, la primera pregunta de cada issue será «¿qué
tienes instalado?». Un `doctor` es la respuesta más barata y la que evita
la mitad de los issues.

## What Changes

- **`meltemi doctor`** (`--json` incluido): un informe compuesto en el
  cliente a partir de `initialize`, `status`, `fleet/list` y comprobaciones
  locales — versiones de cliente y daemon y si coinciden; endpoint, su tipo
  y si responde; directorios de datos y configuración, permisos y espacio;
  configuración de usuario válida; flota con estado compuesto, versión del
  CLI instalado frente al rango del adaptador (`codex-al-dia`) y remedio;
  `git` y `ssh` presentes; en Windows, estado del servidor OpenSSH y shims
  solo `.ps1`; últimas líneas de error del log del daemon. Cada fila con
  glifo y palabra, y remedio cuando hay algo que hacer.
- **Panel «Diagnóstico» en Ajustes de la GUI** con el mismo informe y
  «Copiar», y entrada en la paleta de la TUI. Compuesto en el cliente sobre
  los mismos RPC: el daemon no gana capacidad, así que no nace deber de
  paridad; se ofrece en las tres superficies por utilidad.
- **Sin red, sin envío**: el informe se copia al portapapeles o se imprime;
  nunca se manda a ningún sitio (constitución §9). No incluye rutas ni
  nombres que el usuario no vea ya en su pantalla.
- **`docs/faq.md`** enlaza `doctor` como primer paso de cualquier problema.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `cli-contract`: + `doctor` y su salida.
- `gui-shell` y `tui-shell`: + el panel y la entrada.
- `initial-docs`: + su lugar en la FAQ.

## Impact

- Archivos: `tui/src/cli.rs`, `tui/src/run.rs`, un módulo de diagnóstico en
  `core/meltemi-client` (compartido por las tres superficies),
  `desktop/ui/src/lib/views/Settings.svelte`, `tui/src/shell/palette.rs`,
  `docs/referencia-cli.md`, `docs/faq.md`.
- Cero dependencias nuevas; cero RPC.
- Riesgo: comprobaciones que tardan (un `ssh -V`, leer un log grande); tope
  de tiempo por fila y «no comprobado» como resultado válido.

## Fuera de alcance

- Reparar nada automáticamente: `doctor` diagnostica y dice el remedio;
  ejecutarlo es del usuario (constitución §3: sin efectos externos
  silenciosos).
- Enviar el diagnóstico a un issue o a un servidor: nunca sin el usuario, y
  nunca desde Meltemi.
