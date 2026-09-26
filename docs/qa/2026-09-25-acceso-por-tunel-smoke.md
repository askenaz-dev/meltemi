<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `acceso-remoto-afinado` (acceso por túnel)

**Fecha**: 2026-09-25 · **Plataforma**: Windows 11 Pro (10.0.26200), la única
máquina disponible · **Binarios**: `meltemi` 0.1.1 del taller de la change
(debug, sobre `c078fbc`) y, para el «antes», el release de `main` del
2026-09-24 · Directorios de datos y configuración aislados, y un endpoint
**ajeno** —`\\.\pipe\meltemid-smoke-foreign`, que no es el propio de este
usuario, como no lo es un socket reenviado por un túnel—. El daemon del
mantenedor no se tocó.

## Qué se midió

### El arranque en un endpoint ajeno

| Paso | Binario | Resultado |
| --- | --- | --- |
| `meltemi status` sin bandera | release de `main` (antes) | `exit 0` en 91 ms: **arrancó un `meltemid` en el endpoint ajeno** y le respondió como si fuera el daemon buscado |
| `meltemi status` sin bandera | taller (después) | `exit 10` en 34 ms: «`\\.\pipe\meltemid-smoke-foreign` is not accepting connections, and a daemon is started only at this user's own endpoint: if it is a tunnel, check the tunnel; …, or set MELTEMI_START_HERE=1 to start one at this endpoint»; **ningún `meltemid` quedó corriendo** |
| `meltemi status` con `MELTEMI_START_HERE=1` | taller | `exit 0` en 112 ms: arrancó el daemon del taller en ese endpoint y respondió |
| `meltemi stop` | taller | el daemon se apagó; ningún proceso huérfano |

La primera fila es el defecto que la change existe para cerrar, medido: con
`MELTEMI_ENDPOINT` apuntando a lo que podría ser un túnel caído, el cliente de
antes levantaba un daemon local vacío con ese nombre. El de después rehúsa sin
esperar y dice las dos salidas.

Las mismas tres conductas están fijadas por pruebas con el binario real de
`meltemid` (`core/meltemid/tests/e2e_arranque.rs`, `tui/tests/on_demand.rs`):
el rehúso aunque el binario esté a mano, el arranque pedido, y una segunda
conexión que reutiliza la instancia sin intentar otro arranque.

### El helper de túnel en esta máquina

`meltemi tunnel me@server` → `exit 2`, el rehúso de siempre: un named pipe no
se reenvía, y el remedio nombra `ssh <this-host> meltemi bridge`. En Windows,
por tanto, `--exec` no llega a lanzar `ssh`. La composición con latido
(`ServerAliveInterval=30`, `ServerAliveCountMax=3`, `ExitOnForwardFailure=yes`)
y el reporte de un túnel que muere están fijados por pruebas unitarias en todas
las plataformas.

### Heredado de pruebas anteriores

- El puente contra un named pipe con su entrada y salida en tuberías
  (`tui/tests/bridge.rs`; smoke del 2026-08-15).
- Los verbos sin terminal —`permissions`, `approve`, `watch`— contra el daemon,
  con el binario real (`tui/tests/permission_verbs.rs`,
  `tui/tests/session_verbs.rs`; `verbos-del-agent-boss`).

## Lo que queda pendiente, y qué falta para medirlo

| Tramo | Estado | Qué falta |
| --- | --- | --- |
| GUI y TUI contra un daemon macOS/Linux reenviado con `ssh -R` | **pendiente** | un host macOS/Linux con el daemon y `sshd`; el cliente de Windows habla con named pipes y no puede usar el socket reenviado |
| `--exec` con un `ssh` real que muere (latido, reenvío fallido) | **pendiente** | el mismo host: en Windows el helper rehúsa antes de lanzar `ssh` |
| `ssh <pc> meltemi bridge` a través de un `sshd` real de Windows | **pendiente** | instalar OpenSSH Server en el PC, que cambia un servicio del sistema: decisión del mantenedor |
| Qué pasa si lo primero que arranca el daemon es una sesión SSH de Windows | **pendiente** | lo mismo |

Mientras estos tramos no se midan, la documentación los presenta como lo
esperado, no como lo verificado (`docs/acceso-remoto.md`).
