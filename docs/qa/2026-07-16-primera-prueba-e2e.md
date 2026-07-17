# QA — Primera prueba e2e del producto (2026-07-16)

Primera ejecución de punta a punta de los binarios reales (`meltemi`, `meltemid`,
`mock-agent`, `meltemi-devclient`) en Windows, contra un repo fixture temporal y
directorios de datos aislados (`MELTEMI_DATA_DIR`/`MELTEMI_CONFIG_DIR`) — nunca
este repo (constitución, "tests e2e"). Endpoint por defecto del producto.

## Resultado global: PASA

| Paso | Resultado | Evidencia |
|---|---|---|
| `status` en frío (arranque bajo demanda) | ✅ exit 0 | **189 ms** (presupuesto §12: < 1 s, en build debug) |
| `status --json` | ✅ exit 0 | exactamente un objeto JSON en stdout; stderr vacío |
| `propose` vía CLI (deny-by-default) | ✅ exit 0 | andamio creado; permiso del agente denegado con aviso por stderr; 454 ms |
| `propose` vía devclient (permiso aprobado) | ✅ exit 0 | **el agente escribió la propuesta vía ACP** (passthrough → grant → write); 151 ms |
| Log de sesión JSONL | ✅ | 7 eventos: session_started, prompt_sent, agent_update, permission_requested, permission_decided, turn_completed, session_ended |
| `stop` | ✅ exit 0 | proceso `meltemid` terminado, sin huérfanos |
| `status` tras `stop` | ✅ exit 0 | re-arranque bajo demanda en 176 ms |
| Daemon inalcanzable | ✅ exit 10 | humano por stderr; `--json` → un objeto `{code:10, kind:"daemon_unreachable"}` |
| Subcomando desconocido `--json` | ✅ exit 2 | un objeto `{code:2, kind:"usage"}`; stderr vacío |

La TUI interactiva no es ejecutable en este harness (sin TTY); su contrato está
cubierto por los 65 tests de render/reductores (`TestBackend`). Prueba manual:
abrir `target\debug\meltemi.exe` en una terminal real.

## Hallazgos (para tramitar por el método, no parchear al margen)

| # | Sev. | Hallazgo | Destino sugerido |
|---|---|---|---|
| H1 | 🟠 media | **Honestidad del `propose` denegado**: con el permiso de escritura denegado, la propuesta queda como andamio vacío pero stdout reporta `Completed` sin advertirlo en el resultado (el aviso solo pasa por stderr). Un usuario puede creer que la propuesta se generó completa. | #9 `proxy-permisos` (raíz) o delta menor a `propose-flow`/`cli-contract` (señalizar "permiso denegado durante el turno" en el resultado) |
| H2 | 🟠 media | **Errores de conexión sin endpoint**: "daemon did not start accepting within 5s" no dice qué pipe/socket intentó; el equivalente TUI sí exige socket path + remedy (spec `tui-shell`). | delta menor a `cli-contract` (incluir endpoint en el diagnóstico) |
| H3 | 🟠 media | **`meltemid` muere sin contexto al fallar el bind**: `Error: os error 123` sin nombrar el endpoint, y el fallo no queda en el log. | delta menor a `daemon-lifecycle` (bind con contexto + registro) |
| H4 | 🟡 baja | `Completed` en salida humana es el `Debug` de Rust (variante capitalizada); debería ser una palabra estable/minúscula. | junto con H1 |
| H5 | 🟡 baja | Rutas con separadores mixtos en stdout (`.../demo-repo\.meltemi\...`). | pulido en el mismo delta que H1/H4 |
| H6 | 🟡 baja | **Gotcha de git-bash (MSYS)**: exportar `MELTEMI_ENDPOINT='\\.\pipe\...'` colapsa `\\`→`\` al pasar a procesos nativos → error 123. No es defecto del producto; merece nota para contribuidores Windows. | #22 `documentacion-inicial` |

## Deuda técnica ya conocida, confirmada por la prueba

- El **contador de permisos pendientes es por conexión** en la TUI: no existe RPC
  para enumerar pendientes; una reconexión pierde el conteo. #9 debe resolverlo a
  nivel de contrato (spec visible).
- Las cadenas humanas de la **CLI scriptable no pasan por la tabla ES/EN** (el
  shell sí); hueco de alcance entre `cli-contract` y `tui-shell`.
- Extracción futura de un crate `meltemi-client` (hoy la TUI enlaza `meltemid`
  como librería; anotado en el workspace).
