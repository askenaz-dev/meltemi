## Why

El mantenedor tiene `claude` y `codex` instalados y la Flota los muestra "no
detectados". El diagnóstico es preciso y doble: (1) para los agentes de nivel
2 la detección sondea **el adaptador ACP** (`claude-agent-acp`, `codex-acp`)
— el punto de entrada que meltemid puede pilotar — y no el CLI oficial que el
usuario sí tiene; el "no" es técnicamente honesto pero no explica **qué**
falta ni **cómo** resolverlo. (2) En Windows los shims de npm/nvm existen en
variantes que hoy no se sondean todas (`codex.ps1` además de `codex.cmd`).
Y no existe ninguna guía de configuración por agente: ni en el repo para
GitHub, ni superficie en la app que diga el comando exacto. Para un producto
BYO-agent, la primera experiencia de flota ES el onboarding del producto.

## What Changes

- **Detección en dos capas para nivel 2**: el registro gana `cli-bin` (el
  binario oficial del proveedor) junto al `bin` del adaptador; `fleet/list`
  reporta ambos — CLI detectado sí/no, adaptador detectado sí/no — y el
  estado compuesto honesto: "CLI instalado; falta el adaptador ACP", con el
  comando de instalación como remedy.
- **Sondeo Windows completo**: además de `.exe/.cmd/.bat`, detectar `.ps1`
  como evidencia de instalación (para *detección*; el lanzamiento sigue
  prefiriendo `.exe/.cmd` ejecutables directamente).
- **Remedios accionables en las superficies**: el drawer de Flota (GUI) y el
  detalle (TUI/CLI `--json`) muestran qué capa falta y el comando exacto;
  para Claude Code, la nota de ToS del research (adaptador SDK en zona gris
  con suscripciones; camino seguro: binario oficial) se muestra tal cual —
  honestidad antes que conveniencia.
- **Guía de agentes** `docs/agentes.md` (EN, enlazada desde el README para
  GitHub): por agente soportado — qué instala el usuario, cómo se detecta,
  nivel de integración y qué significa, configuración de perfiles
  (suscripciones múltiples) con ejemplos completos, y solución de problemas
  de detección por SO. Fuente única: se genera/verifica contra
  `fleet-registry.toml` para que no mienta.

## Capabilities

### New Capabilities
- _Ninguna._

### Modified Capabilities
- `fleet-catalog`: + detección en dos capas para nivel 2 (CLI + adaptador)
  con remedy por capa; + sondeo `.ps1` como evidencia; + campos aditivos en
  `fleet/list`.
- `initial-docs`: + guía de agentes verificada contra el registro.

## Impact

- `core/meltemid` (`fleet.rs` detección, registro con `cli-bin`), `proto/`
  (campos aditivos en `FleetAgent`), `tui/` y `desktop/ui` (render del estado
  compuesto y remedios), `docs/agentes.md` + test de coherencia
  registro↔guía, README.
- E2e: fixtures con binarios simulados por capa (solo CLI, solo adaptador,
  ambos, ninguno) y variantes de shim Windows.

## Fuera de alcance

- Instalar adaptadores por el usuario (ejecutar npm/instaladores): Meltemi
  muestra el comando, no lo corre — sin efectos externos silenciosos.
- Cambiar niveles declarados o la política de niveles (conformance suite).
- El sitio web (change `sitio-web-producto` reutiliza la guía).
