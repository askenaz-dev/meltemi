# modelo-y-esfuerzo-por-sesion

> Vía completa (proposal → design → specs → tasks). Cruza contrato, resolver
> de flota, los dos adaptadores propios y las tres superficies, y §5 gobierna
> el diseño entero: **el núcleo transporta strings opacos; solo los
> adaptadores traducen**. Referencias del mantenedor (2026-08-09): el chip
> «modelo · esfuerzo» en el compositor, el picker con búsqueda y ficha por
> modelo, y el aviso de costo al cambiar a mitad de sesión. El motivo es
> administrar cuotas.

## Why

El mantenedor lo pidió con sus capturas y su razón: elegir el modelo del
agente y cuánto esfuerzo de razonamiento gasta, por sesión — «esto es
importante porque ayuda a manejar las cuotas». Una corrección de docs no
merece el modelo grande en high; una migración sí. Hoy Meltemi no ofrece la
palanca: ni el contrato ni el lanzador ni los adaptadores la conocen.

Lo notable es cuánto está a un paso, verificado:

- El protocolo app-server de **Codex ya expone modelo y esfuerzo por turno**
  — `TurnStartParams.model/effort` en el esquema vendorizado
  (`core/mock-provider/schemas/codex-app-server/`) — y el adaptador propio
  manda deliberadamente solo `cwd`, con la decisión §5 citada en el código
  (`core/meltemi-adapters/src/codex/wire.rs:129-141, 170-178`). La palanca
  existe; está apagada a propósito, esperando exactamente esta change.
- El adaptador de **Claude no pasa `--model` hoy**, pero el punto de
  inserción es limpio (`session_args()`,
  `core/meltemi-adapters/src/claude/wire.rs:75-88`) y el adaptador ya **lee**
  el modelo que el CLI anuncia y lo reexporta como meta `providerModel`
  (`claude/wire.rs:289-291`, `claude/surface.rs:54-55`).
- Los perfiles de flota — la unidad con la que el usuario ya administra
  suscripciones — solo declaran `name/agent/env`
  (`core/meltemid/src/config.rs:54-63`): les falta el campo.
- El evento de resolución no registra modelo: `agent_resolved` lleva
  binario, fuente, perfil y nivel y nada más
  (`proto/meltemi-proto/src/lib.rs:1742-1757`). El modelo solo asoma al log
  por vías laterales — el meta `providerModel` que el adaptador de Claude ya
  persiste en sus updates (e2e que lo pinea:
  `e2e_adaptadores_claude.rs:298-309`) y el `usage_reported` de un nivel 3 —
  nunca como dato de resolución consultable.
- Y el hallazgo que endereza la prueba §6: **ACP sí trae el vehículo**. El
  crate pineado arrastra el esquema 1.4.0 con *session config options* —
  `NewSessionResponse.config_options`, request `session/set_config_option` —
  y categorías estándar `Model`, `ModelConfig` y `ThoughtLevel`. Son opciones
  que **el agente anuncia** y el cliente fija: encajan exactamente con
  adaptadores propios que conocen su CLI. La vía estándar existe y esta
  change la cablea (§6); el transporte del contrato propio queda para lo que
  las opciones no cubren — el default de perfil aplicado al lanzar — con la
  frontera exacta escrita en el design.

## What Changes

- **Contrato**: `model` y `effort` opcionales — **strings opacos** — en
  `session/start` (cobertura de `propose`/`worktree/dispatch` en el design),
  transportados por el resolver sin interpretarse jamás (§5: el patrón de
  `acp-args` y `auth-context-var`, datos que el núcleo lleva y no lee).
  `agent_resolved` gana `model`/`effort` efectivos (el meta `providerModel`
  que el adaptador de Claude ya deja en sus updates se promueve a dato de
  resolución): lo que corrió queda escrito donde se consulta.
- **La vía ACP estándar se cablea donde aplica**: los adaptadores propios —
  que son el lado agente de ACP — anuncian sus opciones (`Model`,
  `ThoughtLevel`) como *session config options* y el daemon las fija con
  `session/set_config_option`; el mock-agent gana el escenario para
  ejercitarlo sin proveedor alguno.
- **Cada adaptador traduce lo que su CLI oficialmente soporta**: Claude →
  `--model` en `session_args()` (y esfuerzo solo si el CLI pineado lo expone
  — el design lo verifica contra esa versión, no se cita de memoria); Codex →
  `model` al arrancar el thread y `effort` **por turno** (su esquema define
  `effort` solo en `TurnStartParams`, no en el arranque del thread). Lo no
  soportado se **rehúsa con diagnóstico** («este agente no acepta esfuerzo
  por sesión»), jamás se ignora en silencio.
- **Los perfiles ganan `model` y `effort` opcionales**: «perfil = agente +
  cuenta + modelo» — la unidad de administración de cuotas que el mantenedor
  pidió. El chip de perfil del lanzador ya existe y lo consume sin superficie
  nueva; con esto, «docs con el modelo barato» es un perfil, no un ritual.
- **GUI, con la referencia a la vista**: chip «modelo · esfuerzo» en el
  compositor/lanzador que abre un picker con búsqueda. La ficha por modelo
  muestra **solo lo que Meltemi sabe de verdad**: lo anunciado por el CLI,
  lo declarado en perfiles, y el consumo medido por la analítica local.
  **Sin precios ni créditos**: Meltemi no tiene ni lo uno ni lo otro
  (BYO-suscripción), y una tabla de precios embebida sería asumir proveedores
  (§5) y pudrirse en silencio. El picker lista lo declarado + lo anunciado, y
  admite entrada libre — el string es opaco también para la UI.
- **Cambio a mitad de sesión, con el aviso honesto de la referencia**: donde
  el protocolo del proveedor lo soporta por turno (Codex), se ofrece con el
  aviso «cambiar de modelo a mitad de sesión reinicia la caché del proveedor
  y puede aumentar el costo» — verdad técnica, no retórica. Donde exige
  relanzar (Claude), la vía es resume con el modelo nuevo y la UX lo dice;
  el design fija ambas formas. Compone con `sesion-que-espera` sin esperarla.
- **TUI y CLI a la par** (§4): flags `--model`/`--effort` en los verbos de
  arranque, selector en el flujo de dirección de la TUI, y el valor efectivo
  visible en el detalle de sesión de ambas superficies.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + transporte opaco de `model`/`effort` y su registro en
  `agent_resolved`.
- `fleet-catalog`: + campos opcionales de perfil y su resolución
  (perfil < sesión: lo explícito de la sesión pisa el default del perfil).
- `own-adapters`: + la traducción por adaptador con rehúso diagnosticado de
  lo no soportado.
- `cli-contract`: + flags en los verbos de arranque.
- `gui-shell` / `tui-shell`: + chip, picker y valor efectivo visible.

## Impact

- Archivos: `proto/` (schemas de arranque y `agent_resolved`),
  `core/meltemid/src/{server.rs, fleet.rs, config.rs}`,
  `core/meltemi-adapters/src/{claude,codex}/`, `tui/`, `desktop/ui`
  (lanzador, sesión), matriz de paridad, docs de flota.
- Cero dependencias nuevas. El mock-agent ignora los campos (son opacos
  también para él) — los tests no exigen proveedor alguno, §5 intacto.
- Riesgo nombrado: un string de modelo inválido lo rechaza el **CLI del
  proveedor**, no Meltemi; el requisito es que ese rechazo llegue legible a
  la superficie (el patrón de rehúso con remedio existente), no que Meltemi
  valide nombres que no le pertenecen.
- La verificación contra CLIs reales es manual y se documenta con la versión
  probada (CI jamás corre agentes reales) — el patrón de conformidad de
  `own-adapters` ya cubre el desfase de versión.

## Fuera de alcance

- **Ruteo automático «el mejor modelo para la tarea»** (el «Auto» de la
  referencia): sin precios, sin telemetría y sin asumir proveedores no hay
  «mejor» honesto que el núcleo pueda calcular. Primero la palanca manual y
  los perfiles (esta change); una política declarativa tarea→perfil es
  change futura (`ruteo-declarativo-de-perfiles`, nombrada aquí) y cualquier
  heurística llega con evidencia, jamás embebida en el core.
- **Precios, créditos y descuentos**: modelo comercial de terceros; Meltemi
  mide consumo local (analítica existente) y no fabrica costos.
- **Catálogo de modelos embebido en el núcleo**: §5 literal; los modelos
  visibles nacen de datos del usuario y de lo que el CLI anuncia.
- **Extender ACP por fuera de las session config options**: si algo no cabe
  en las opciones que el estándar ya define, viaja por el contrato propio
  daemon↔clientes — jamás por una extensión ACP privada sin su prueba §6
  escrita.
