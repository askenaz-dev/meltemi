# corrida-integrada

> Vía rápida (fast-forward): un script reproducible, dos informes en
> `docs/qa/` y las piernas que faltan en la conformidad manual. Casi todo es
> trabajo del mantenedor con sus binarios y sus suscripciones; nada corre en
> CI. Nace del pedido literal del mantenedor («prueba integrado usando los
> agentes locales») y de la prueba del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas AI-2, AI-V1 y P9).

## Why

Lo que el repositorio sabe de los agentes reales cabe en tres líneas de
`docs/conformidad-manual.md`: OpenCode abrió sesión una vez sin prompt;
Claude Code alcanzó nivel 2 contra `claude 2.1.167` (la máquina tiene
2.1.280); Codex tiene nivel verificado 0. Gemini, Copilot, Cursor, Kiro y
Kilo nunca se ejercieron. Y el informe de aceptación que la spec
`v01-acceptance` exige —una funcionalidad de idea a código en terminal, con
dos agentes de proveedores distintos en paralelo— no existe:
`docs/hito-v01.md` es una plantilla.

Una tarde de corrida ad hoc el 2026-09-24 encontró tres defectos que ningún
test con el agente simulado podía ver: el adaptador de Codex no lee el user
agent actual, un turno vacío se reporta como éxito, y `--model` se rehúsa a
un agente que anuncia modelos. La lección no es «probar más», es **probar
igual cada vez**: mismo aislamiento, misma matriz, mismo informe con fecha,
para que la próxima versión de un CLI se mida en una hora y no se descubra
en un push.

Orca compra credibilidad con uso diario y 77k estrellas. Meltemi tiene que
comprarla con evidencia publicada: qué agente, qué versión, qué verbo, qué
pasó.

## What Changes

- **`scripts/smoke-agents.ps1` y `scripts/smoke-agents.sh`**: levantan un
  daemon **aislado** (las cinco variables `MELTEMI_ENDPOINT`,
  `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR`, `MELTEMI_DAEMON_BIN` y, desde
  `acceso-remoto-afinado`, `MELTEMI_START_HERE=1`; jamás el daemon ni los datos
  del usuario), crean un repo fixture temporal, detectan la flota instalada y
  corren la matriz sobre cada agente `ready`: sesión de solo lectura, sesión de
  escritura sin regla (debe denegar), sesión de escritura con regla, `propose`
  con la regla de lectura y edición acotada al fixture, `dispatch` en
  worktree, `direct`, `cancel`. Miden duración, permisos
  pedidos/aprobados/denegados, turnos vacíos y uso reportado, y escriben el
  esqueleto de `docs/qa/<fecha>-corrida-integrada.md` con los números. Todo
  con las trampas ya aprendidas escritas en el script (pipe retenido,
  `Start-Process` con redirección, endpoint explícito).
- **El informe de aceptación v0.1**: el escenario de `v01-acceptance`
  ejecutado con dos agentes de proveedores distintos (Claude Code y OpenCode
  hoy; Codex cuando `codex-al-dia` lo permita), publicado como
  `docs/qa/<fecha>-hito-v01-aceptacion.md` con veredicto por criterio y
  `docs/hito-v01.md` dejando de ser plantilla.
- **Las piernas que faltan en `docs/conformidad-manual.md`**: Claude Code
  2.1.280 (`AskUserQuestion` con la forma real de `updatedInput`, `--model`
  de punta a punta, prompt-tool), OpenCode 1.18 con prompt, y los nivel 1 no
  instalados **declarados como no medidos** en vez de omitidos.
- **Regla escrita**: la corrida se repite ante cualquier cambio de versión de
  un CLI pineado o de un adaptador propio, y su fecha es parte del estatus
  que la flota muestra (`docs/agentes.md`).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `v01-acceptance`: + requisito «El informe de la corrida manual está
  publicado», con fecha, versiones y veredicto por criterio, y
  `docs/hito-v01.md` enlazándolo (absorbe lo que la propuesta ponía en
  `initial-docs`).
- `integration-levels`: + requisito «Corrida integrada reproducible contra los
  CLIs reales»: matriz agente × verbo, aislada del daemon del usuario,
  publicada en `docs/qa/`.

## Impact

- Archivos: `scripts/smoke-agents.{ps1,sh}` (nuevos), `docs/qa/` (dos
  informes nuevos), `docs/conformidad-manual.md`, `docs/hito-v01.md`,
  `docs/agentes.md`.
- Cero dependencias nuevas; cero RPC; nada en CI (constitución §5: los e2e de
  CI usan el agente simulado).
- Costo real: turnos de las suscripciones del mantenedor y su tiempo. Se
  acota con instrucciones cortas y un fixture mínimo.
- Depende de `codex-al-dia` para la pierna de Codex y de ollama instalado
  para la pierna local (`guia-modelos-locales`); las demás piernas no esperan.

## Fuera de alcance

- Automatizar la corrida en CI o con agentes reales sin humano: prohibido
  por §5 y por el costo.
- Corregir lo que la corrida encuentre: cada hallazgo es su change, como los
  tres del 2026-09-24.
