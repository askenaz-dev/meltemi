<!-- SPDX-License-Identifier: Apache-2.0 -->
# Corrida integrada — 2026-09-26

Escrita por `scripts/smoke-agents.ps1` el 2026-09-26 00:08 UTC, sobre `meltemi 0.1.1` (commit `a9e7bb7`), en Microsoft Windows 11 Pro. Daemon, datos y configuración aislados; un repositorio fixture temporal.

## Agentes

| Agente | Versión |
| --- | --- |
| `opencode` | 1.18.32 |
| `claude-code` | 2.1.280 (Claude Code) |
| `codex-cli` | codex-cli 0.156.1 |
| `gemini-cli` | **no medido**: no instalado |
| `copilot-cli` | **no medido**: no instalado |
| `cursor-cli` | **no medido**: no instalado |
| `kiro-cli` | **no medido**: no instalado |
| `kilo-code` | **no medido**: no instalado |
| `aider` | **no medido**: no instalado |
| `antigravity` | **no medido**: no instalado |

## La matriz

Permisos: pedidos · decididos por regla · por el cliente · denegados. Uso: el que el agente reportó por el protocolo (costo, contexto usado/tamaño), vacío si no reportó.

### `opencode`

| Verbo | Resultado | Segundos | Permisos | Turnos vacíos | Uso | Nota |
| --- | --- | --- | --- | --- | --- | --- |
| session (read) | completed | 20.2 | 0 · 0 · 0 · 0 | 0 | 0 USD · 11733/200000 |  |
| session (write, no rule) | **WRITTEN without asking** ¹ | 17.8 | 0 · 0 · 0 · 0 | 0 | 0 USD · 11673/200000 |  |
| session (write, rule) | written without asking ¹ | 18.7 | 0 · 0 · 0 · 0 | 0 | 0 USD · 11671/200000 |  |
| propose | completed | 36.8 | 0 · 0 · 0 · 0 | 0 | 0 USD · 18913/200000 | add-a-farewell-function-next-to-greet-drafted-by-opencode |
| dispatch | completed | 38.3 | 0 · 0 · 0 · 0 ³ | 0 | 13753/200000 ³ | commit aea9f6ba46e23b6eee458c3810eb35d358a8df37 |
| direct | resumed | 9.4 | 0 · 0 · 0 · 0 | 0 | 0 USD · 11756/200000 |  |
| cancel | cancelled (cancelled) | 9.3 | 0 · 0 · 0 · 0 | 0 |  |  |

### `claude-code`

| Verbo | Resultado | Segundos | Permisos | Turnos vacíos | Uso | Nota |
| --- | --- | --- | --- | --- | --- | --- |
| session (read) | completed | 34.5 | 5 · 0 · 5 · 5 | 0 |  |  |
| session (write, no rule) | not written (denied) | 18.3 | 1 · 0 · 1 · 1 | 0 |  |  |
| session (write, rule) | not written ² | 17.3 | 1 · 0 · 1 · 1 | 0 |  |  |
| propose | completed | 78.8 | 7 · 0 · 7 · 7 | 0 |  | add-a-farewell-function-next-to-greet-drafted-by-claude-code |
| dispatch | completed | 36 | 7 · 0 · 7 · 7 ³ | 0 |  | no commit: sus ediciones se denegaron ² |
| direct | resumed | 16.6 | 4 · 0 · 4 · 4 | 0 |  |  |
| cancel | cancelled (cancelled) | 5.9 | 0 · 0 · 0 · 0 | 0 |  |  |

### `codex-cli`

| Verbo | Resultado | Segundos | Permisos | Turnos vacíos | Uso | Nota |
| --- | --- | --- | --- | --- | --- | --- |
| session (read) | completed | 39.7 | 1 · 0 · 1 · 1 | 0 |  |  |
| session (write, no rule) | not written (denied) | 37.6 | 2 · 0 · 2 · 2 | 0 |  |  |
| session (write, rule) | not written ² | 29.2 | 3 · 0 · 3 · 3 | 0 |  |  |
| propose | completed | 49.1 | 4 · 0 · 4 · 4 | 0 |  | add-a-farewell-function-next-to-greet-drafted-by-codex-cli |
| dispatch | completed | 34.9 | 2 · 0 · 2 · 2 ³ | 0 |  | no commit: sus ediciones se denegaron ² |
| direct | exit 11 | 0.1 | 0 · 0 · 0 · 0 | 0 |  | contract error: [2004] session not directable: session `8815f327-73ab-4258-b3b9-44931f768fe1` has ended and its agent does not support resuming — List sessions … |
| cancel | cancelled (cancelled) | 6.6 | 0 · 0 · 0 · 0 | 0 |  |  |

Las marcas ¹ ² ³ remiten a los hallazgos. Tres celdas de esta tabla se
corrigieron a mano después de la corrida, y se dice cuáles: los permisos de los
tres `dispatch` (³), leídos de los archivos del registro porque `meltemi log` no
los encontró, y los dos veredictos de escritura de OpenCode (¹), que el script
de esta corrida llamó «sin regla» y «por regla» cuando OpenCode no pidió
permiso ninguna de las dos veces. El script ya distingue ese caso.

## Hallazgos

1. **OpenCode 1.18.32 edita sin pedir permiso.** En toda la matriz no hizo una
   sola petición: escribió `NOTES-opencode-unruled.md` sin regla y sin humano,
   y su despacho hizo commit. El registro muestra la llamada (`tool_call`,
   `kind: edit`, título `write`) y ningún `permission_requested`: con su
   configuración por omisión, OpenCode no pregunta por ACP y el proxy de
   permisos no tiene nada que decidir. La sesión queda registrada, pero **no
   gobernada**. Es la primera cosa que arreglar: una change aparte que lance
   OpenCode de modo que pregunte.
2. **Una regla acotada por ruta no concede nada a los agentes de nivel 2.**
   Las peticiones de Claude Code y de Codex llegan por sus adaptadores sin
   `rawInput` ni `locations`: Claude pone la ruta solo en el título y en el
   contenido, Codex ni eso. El daemon no conoce la ruta, una regla con
   `path-prefix` no puede casar, y la escritura con regla se denegó igual que
   sin ella; por eso sus despachos no hicieron commit. Lo aborda
   `adaptadores-con-rawinput`, ya propuesta. Codex, además, pide `execute`
   para leer archivos, y sin regla para ejecutar se le deniega: es lo diseñado.
3. **`meltemi log` no encuentra el registro de una sesión de worktree.** Una
   sesión de `dispatch` queda indexada con el worktree como raíz del proyecto,
   pero su registro vive bajo la clave del proyecto principal: `session/log`
   busca donde no está y devuelve una página vacía, con código `0`. Afecta al
   verbo `log` de `verbos-del-agent-boss` y probablemente al detalle de sesión
   de la GUI. Change aparte.
4. **Codex no se reanuda.** `direct` sobre su sesión terminada responde
   `session not directable` (2004): lo aborda `codex-reanudar-y-mcp`. OpenCode
   y Claude Code se reanudaron.
5. **Solo OpenCode reporta uso por el protocolo**: contexto usado y un costo de
   0 USD (su modelo gratuito). Los adaptadores de Claude Code y de Codex no
   envían `usage_update`; lo que ellos sepan de su consumo no llega a la
   contabilidad. Encaja en `extension-acp-usage`, ya propuesta.
6. **Lo que funcionó en los tres**: la lectura, la denegación sin regla (salvo
   OpenCode, que no pregunta), `propose`, `dispatch` hasta completar el turno,
   y `cancel` sobre una sesión viva, que llegó en 6–10 segundos.

## Lo no medido

- Los siete agentes del catálogo que no están instalados (tabla de arriba).
- `scripts/smoke-agents.sh`: no puede ejercerse en esta máquina —el named pipe
  no se alcanza desde git-bash— y queda para una corrida en macOS o Linux.
- El costo real de Claude Code y de Codex: sus adaptadores no lo reportan
  (hallazgo 5).
