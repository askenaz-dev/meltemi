# runner-nivel-3

> Vía completa (proposal → design → specs → tasks), **opcional**: solo se
> aborda si un CLI de nivel 3 lo justifica con demanda (hoy, Aider). Nace de
> la corrección que `guia-modelos-locales` hace al backlog —nivel 3 no se
> lanza— y del comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas LM-1 y AI-9).

## Why

El registro declara a Aider nivel 3 («modo headless estructurado») y
`docs/agentes.md` promete «ejecuciones programáticas dentro de guardarraíles
obligatorios; sin canal rico de permisos». El daemon conserva `headless` y
`headless_args` en las entradas (`core/meltemid/src/fleet.rs`) y **rehúsa
lanzar** todo nivel 3 con `level_not_pilotable` en los cinco verbos. Nivel
3 existe como categoría y no como camino.

Vale la pena si y solo si hay un agente que la gente quiere usar y que no
habla ACP: Aider es el candidato, y es además la vía más directa a modelos
locales por su propio soporte de ollama y OpenAI-compatible. Si Aider gana
ACP nativo antes de que esto se implemente, la change se archiva sin código.

## What Changes

- **Un runner headless** que lanza el CLI en su modo estructurado con la
  instrucción, **solo en un worktree gestionado** (jamás sobre el árbol del
  usuario), con checkpoint antes y commit por tarea después; sin canal de
  permisos, la sesión corre como autónoma con los guardarraíles que el
  worktree y el checkpoint dan, y lo dice en la flota y en el log
  («nivel 3: sin canal de permisos»).
- **Salida estructurada plegada al log**: lo que el CLI emita en su formato
  (JSON u otro) se registra verbatim y se resume como turno; sin
  streaming fino si el CLI no lo da.
- **Modelo verbatim por dato del registro**: la entrada declara
  `model-flag`; el usuario escribe el nombre completo que el CLI espera
  (`ollama_chat/…`); el daemon no traduce proveedores (§5).
- **Conformidad de nivel 3**: la suite por nivel gana sus criterios
  pasa/no-pasa y Aider se mide contra el binario real.

## Capabilities

### New Capabilities

- Ninguna: `integration-levels` ya define el nivel 3; esta change lo hace
  ejecutable.

### Modified Capabilities

- `integration-levels`: + requisitos del runner (worktree obligatorio,
  checkpoint, commit, salida verbatim, declaración de «sin canal de
  permisos»).
- `fleet-catalog`: + `model-flag` y `headless` como datos usados.
- `worktree-orchestration`: + escenario de despacho nivel 3.

## Impact

- Archivos: `core/meltemid/src/{levels.rs, worktrees.rs, free_session.rs,
  propose.rs, sdd_flow.rs, server.rs}`, `core/mock-headless` (ya existe
  para los e2e), `docs/agentes.md`, `docs/conformidad-manual.md`.
- Cero dependencias nuevas.
- Riesgo real: un CLI headless que edita fuera del worktree o ejecuta
  comandos sin preguntar; el worktree y el checkpoint acotan el daño, y la
  propuesta lo llama por su nombre: nivel 3 es menos gobierno, no más.

## Fuera de alcance

- Un canal de permisos sintético leyendo la salida del CLI: sería adivinar.
- Nivel 3 sobre el árbol del usuario: nunca.
