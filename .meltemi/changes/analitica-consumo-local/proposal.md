## Why

El mantenedor pide un panel de analítica de consumo. La constitución marca la
cancha con precisión: §2 prohíbe tocar las cuentas de los proveedores (la
cuota real no es visible y no se promete), §9 exige que toda métrica sea
local. flota-multiproveedor ya lo dejó declarado como futuro condicionado a
demanda: "solo cabe contabilidad local de lo que Meltemi despachó (futuro, si
se pide)". Se pidió. Y hay más disponible de lo que parece, sin romper nada:
los logs JSONL de sesión ya registran turnos, permisos, ediciones, commits y
resoluciones de agente/perfil; y los modos headless oficiales (nivel 3) sí
emiten contadores de tokens en su salida estructurada (`claude -p
--output-format stream-json`, `codex exec --json`), que puede capturarse
honestamente porque ES la interfaz oficial. ACP v1.2 no transporta usage: se
declara "no disponible" para esas sesiones, nunca se estima ni se inventa.

## What Changes

- **Contabilidad local de actividad**: agregación sobre los JSONL existentes
  — sesiones, turnos, duración, permisos (aprobados/denegados/vencidos),
  ediciones humanas y commits — por proyecto × agente × perfil × período.
- **Captura de tokens donde la interfaz oficial los emite**: en ejecuciones
  headless (nivel 3), los contadores de uso del stream oficial se persisten
  como evento local del log de sesión; en sesiones ACP se muestra
  "no reportado por el protocolo" — el panel jamás mezcla medido con
  estimado.
- **RPC de agregación** (`analytics/usage`, aditivo) que computa en el daemon
  sobre los logs locales; **panel de analítica** en la GUI (vista bajo el
  sidebar) y salida `--json`/tabla en CLI + casa en la paleta TUI (paridad
  §4).
- **Declaración de honestidad en el propio panel**: qué se mide, de dónde
  sale, qué no es visible (cuota del proveedor) y que nada sale de la
  máquina (§9), visible junto a los números.

## Capabilities

### New Capabilities
- `local-analytics`: la contabilidad local agregada y su superficie, con la
  frontera de honestidad como requisito de primera clase.

### Modified Capabilities
- `session-history`: + evento local de uso para ejecuciones headless.

## Impact

- `core/meltemid` (agregador sobre JSONL, captura headless), `proto/`
  (método + tipos aditivos), `tui/`, `desktop/ui` (panel con el design
  system), matriz de paridad (+1 método en las tres superficies).
- E2e: fixtures con logs sintéticos multi-proyecto/perfil; verificación de
  que una sesión ACP reporta "sin datos de tokens" y una headless simulada
  sí los agrega.

## Fuera de alcance

- Leer cuota, saldo o facturación de cuentas de proveedores — jamás (§2).
- Estimación de tokens por conteo propio de texto: números inventados no
  (honestidad); si un día se ofrece, será opt-in y etiquetado como estimado.
- Telemetría o envío de métricas fuera de la máquina — jamás (§9).
- Presupuestos/alertas de gasto: fast-follow si la contabilidad demuestra
  demanda.
