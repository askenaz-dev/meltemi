## Why

El propósito fundacional del proyecto — "un rumbo, muchas velas" — todavía no es
literalmente cierto: la orquestación por worktrees etiqueta carreras entre
agentes con nombres distintos, pero `resolve_launch` resuelve **un solo**
`agent.command`/`agent.id` de la configuración, así que dos competidores lanzan
el **mismo binario**. El dolor real del usuario multi-proveedor tiene dos capas:
(1) elegir el proveedor **por sesión** (no por proyecto), y (2) usar **múltiples
suscripciones** del mismo o distinto proveedor (cuenta personal vs trabajo) sin
que Meltemi toque jamás una credencial (constitución §2). Esta change cierra la
última milla: del catálogo de flota ya existente a la sesión que lanza el
binario de *ese* proveedor con *ese* contexto de autenticación.

## What Changes

- **Resolución de agente por sesión**: el nombre de agente que ya viaja en
  `worktree/assign`, `sdd/implement` y el despacho nuevo se resuelve contra la
  flota — perfil → id del catálogo (registro o `[[fleet.custom]]`) → agente
  configurado del proyecto como fallback — y la resolución (binario + fuente)
  queda registrada en el log de sesión: jamás ambiguo qué binario corrió.
- **Perfiles de lanzamiento** (`[[fleet.profile]]`): nombre + agente del
  catálogo + sobrecapa de entorno que selecciona el **contexto de
  autenticación** del binario (p. ej. `HOME`/`XDG_CONFIG_HOME` por cuenta). Los
  valores admiten referencias `${VAR}` resueltas al lanzar (patrón ya vigente en
  MCP); el lint de higiene rehúsa secretos en claro. Meltemi nunca lee, almacena
  ni reenvía el material secreto: BYOK hecho literal para multi-cuenta.
- **Despacho de competidor** (`worktree/dispatch`): correr el turno de un agente
  (o perfil) sobre el worktree de su asignación — checkpoint → turno bajo reglas
  → commit con trazabilidad — **sin marcar la tarea** (el competidor no la
  posee; la fusión asistida decide). N despachos en paralelo = la carrera
  multi-proveedor real.
- **Catálogo**: `fleet/list` lista los perfiles (fuente `profile`, agente
  subyacente, detección); CLI/TUI los muestran (paridad §4).

## Capabilities

### New Capabilities
- _Ninguna_ (la change extiende capacidades existentes con requisitos nuevos).

### Modified Capabilities
- `fleet-catalog`: + resolución por sesión, + perfiles de lanzamiento ciegos a
  credenciales, + perfiles en el listado.
- `worktree-orchestration`: + despacho de competidor sobre su worktree.

## Impact

- `core/meltemid` (resolución en `levels`/`fleet`, perfiles en `config`,
  `worktree/dispatch` en `server`), `proto/` (método + tipos aditivos), `tui/`
  (subcomando `dispatch`, render de perfiles en `fleet`).
- E2e: dos agentes de flota declarados (dos perfiles del mock con salida
  distinguible) corren la misma tarea en paralelo y cada worktree evidencia el
  binario/contexto que corrió.

## Fuera de alcance

- **Visibilidad de cuota/costo por suscripción**: leerla exige tocar la cuenta
  del proveedor (§2 juego limpio) y contarla sería telemetría (§9). Solo cabe
  contabilidad local de lo que Meltemi despachó (futuro, si se pide).
- **Failover reactivo** (reenrutar ante rate-limit/auth): fast-follow declarado;
  exige detectar clases de error ACP con honestidad (nunca predictivo — la
  cuota no es visible).
- **Credenciales**: leer/almacenar/reutilizar material secreto de agentes —
  jamás (§2). Los perfiles solo redirigen el contexto; el binario se autentica
  solo.
- `propose` sigue usando el agente configurado (no nombra agente hoy); darle
  selección es un delta menor futuro.
