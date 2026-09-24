# comandos-del-agente

> Vía rápida (fast-forward): un evento aditivo en el log, un campo aditivo en
> el contrato y su popup en las dos superficies. Nace del comparativo con
> Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brechas D2 y AI-7).

## Why

Los agentes anuncian sus comandos. OpenCode lo hizo el 2026-09-24 en la
primera sesión: `available_commands_update` con `init` («guided AGENTS.md
setup»), `review` y los demás, y el daemon lo escribió en el log tal cual
llegó. Pero nadie lo lee: `core/meltemid/src/acp.rs:731-736` deja el caso
`SessionUpdate::AvailableCommandsUpdate` visto y **deliberadamente no
tomado** — «commands are not options… Giving them a place of their own is
another change». Esta es esa change.

En la GUI, escribir `/` en el compositor no hace nada; en la TUI tampoco. El
usuario que sabe que Claude Code tiene `/compact` o que OpenCode tiene
`/review` los escribe a ciegas o no los usa. Orca los muestra y los deja al
agente («agent-handled slash commands»); es lo mismo que la constitución §5
pide: el comando es del agente, Meltemi lo enseña y lo entrega como texto,
jamás lo interpreta.

## What Changes

- **El daemon guarda el último anuncio por sesión**: `AvailableCommandsUpdate`
  se registra como evento propio `commands_announced` (como
  `config_options_announced` hoy) y la lista viaja aditiva en `session/list`
  y en el stream de `session/watch`, para que un cliente tardío también la
  tenga.
- **Popup de `/` en el compositor de la GUI** (Home y sesión abierta): al
  escribir `/` al inicio de una línea se listan los comandos anunciados con
  su descripción, rotulados «del agente» para que no se confundan con los
  modos de Meltemi (Proponer, Explorar); elegir uno inserta `/nombre ` como
  texto. El prompt se envía verbatim: el agente decide qué hace con él.
- **Lo mismo en la TUI**: el compositor conversacional ofrece la lista con la
  misma tecla y el mismo rótulo, con glifo y palabra.
- **Sin comandos anunciados, nada aparece**: ni lista vacía ni comandos
  inventados por Meltemi.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «Los comandos anunciados por el agente quedan
  registrados y disponibles para todo cliente de la sesión».
- `gui-shell` y `tui-shell`: + el popup de comandos del agente, rotulado y
  verbatim.

## Impact

- Archivos: `core/meltemid/src/{acp.rs, session.rs}`,
  `proto/schemas/v1/{session-event, session-list}.schema.json` (aditivos),
  `desktop/ui/src/lib/{conversation.ts, views/Home.svelte,
  views/SessionDetail.svelte}` (el evento nuevo necesita glifo y tono en
  `EVENT_STYLE`, como exige el guardián), `tui/src/shell/`,
  `docs/paridad-nucleo.md`.
- Cero dependencias nuevas.
- Riesgo: un `/` que el usuario quería literal al inicio del prompt; `Esc`
  cierra el popup y deja el carácter.

## Fuera de alcance

- Ejecutar comandos de Meltemi desde el compositor con `/`: los verbos del
  método viven en los chips y la paleta, no compiten con los del agente.
- Comandos con argumentos tipados o autocompletado del agente: si algún ACP
  los anuncia, se estudia con evidencia.
