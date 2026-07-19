## Context

La flota (catálogo + detección) existe; los worktrees etiquetan carreras por
agente; `implement` compone el ciclo por tarea. Pero el lanzamiento resuelve un
único agente de configuración: la carrera multi-proveedor es hoy dos copias del
mismo binario. El contrato ACP ya nos hace agnósticos (§5/§6); falta que la
sesión elija su vela. Primera change tramitada íntegramente en
`.meltemi/changes/` (humo del dogfooding post-migración).

## Goals / Non-Goals

**Goals:** resolución de agente por sesión desde la flota, registrada en el log;
perfiles de lanzamiento multi-cuenta ciegos a credenciales; despacho paralelo de
competidores con su propio binario; perfiles visibles en el catálogo.
**Non-Goals:** cuota/costo por suscripción (§2/§9); failover (fast-follow);
tocar credenciales (jamás, §2); selección de agente en `propose`.

## Decisions

### D1 — Resolución por nombre con fallback visible, nunca ambigua
Orden por sesión: (1) perfil de lanzamiento, (2) id del catálogo (registro o
`[[fleet.custom]]`), (3) el agente configurado del proyecto como fallback. La
resolución (binario efectivo + fuente `profile|catalog|configured`) se registra
como evento en el JSONL de la sesión. Razón del fallback: compatibilidad — los
nombres de carrera actuales son etiquetas libres ("fast"/"thorough") y romperlas
sería un cambio de contrato; la honestidad la da el registro, no la rigidez. Un
id resuelto pero **no detectado** en el sistema rehúsa con 2001 (diagnóstico +
remedio), nunca degrada en silencio a otro proveedor.

### D2 — Perfiles como descriptores de lanzamiento, no de identidad
`[[fleet.profile]]`: `name`, `agent` (id del catálogo) y `env` (tabla). La
sobrecapa se aplica al subproceso del binario oficial; el patrón sancionado es
**redirigir el contexto de autenticación** (`HOME`, `XDG_CONFIG_HOME`,
directorios de config del proveedor), de modo que el binario se autentica solo
con la cuenta de ese contexto. Los valores admiten `${VAR}` resueltas del
entorno del daemon al lanzar (reuso exacto de `resolve_ref` de MCP) y el lint
`looks_like_plaintext_secret` (reuso de mcp-passthrough D1) **rehúsa** valores
que parezcan secretos en claro: la config de Meltemi jamás persiste material
secreto (§2). Nada de esto lee ni valida la credencial: si el contexto está sin
autenticar, el error es del binario y se muestra tal cual (honesto).

### D3 — `worktree/dispatch`: el primitivo componible de la carrera
`(change, task, agent|profile)` → resolver binario (D1) → crear/reutilizar el
worktree de la asignación + checkpoint pre-turno → turno del agente en el
worktree bajo las reglas vigentes → commit con trailers de trazabilidad —
**sin tick de `tasks.md`**: un competidor no posee la tarea; el humano elige con
la fusión asistida existente (`worktree/diff` + `worktree/merge-file`). N
despachos concurrentes = carrera; la base común ya queda fijada por la
asignación. `sdd/implement` (secuencial, con tick) queda intacto salvo que su
parámetro `agent` pasa por D1.

### D4 — Perfiles en el catálogo, paridad de superficies
`fleet/list` incorpora los perfiles con fuente `profile`, su agente subyacente y
la detección del binario subyacente (aditivo al schema). CLI `fleet` los
renderiza; el subcomando nuevo `dispatch <change> <task> <agent>` expone el
despacho (paridad §4: método consumible por TUI/GUI/CLI por igual).

## Risks / Trade-offs

- **Colisión de sobrecapa con el entorno que el daemon ya fija** → la sobrecapa
  del perfil se aplica al final y solo a las claves declaradas; documentado.
- **Falsos positivos del lint de higiene** → mismo trade-off ya aceptado en MCP:
  mejor rehusar de más con remedio (`${VAR}`) que persistir un secreto.
- **Etiqueta libre que casualmente coincide con un id del catálogo** → cambia el
  binario lanzado respecto de hoy; mitigado por el evento de resolución (visible)
  y por ser el comportamiento que el nombre ya sugería.
- **Windows**: claves de entorno case-insensitive; la sobrecapa usa las claves
  tal como se declaran y se prueba en las tres plataformas.

## Migration Plan

Aditivo por completo: sin perfiles declarados y con etiquetas libres, todo
comportamiento actual se conserva (fallback D1-3). Reversión: retirar método y
config; los worktrees/commits creados son artefactos git normales.

## Open Questions

- Failover reactivo (clases de error ACP → reintento con otro perfil): forma
  exacta del mapeo de errores; queda como change futura con evidencia de uso.
- ¿`propose` con agente nombrado? Delta menor cuando la selección demuestre
  demanda.
