## Context

El daemon lanza hoy un único agente resuelto por `config.rs` (`agent_command`,
precedencia defaults < config usuario < config proyecto < `MELTEMI_AGENT_COMMAND`;
error 2000 si falta). La vista Flota de la TUI es una casa reservada con estado
vacío, y no existe noción de "qué agentes hay en esta máquina". El research
interno (`docs/research/integracion-agentes.md`, julio 2026) cataloga los agentes
del mercado con su invocación ACP, su modo headless y su nivel Meltemi — es la
fuente de datos de esta change. Restricciones heredadas: sin puertos de red,
dependencias mínimas, CI sin red ni agentes reales, Windows de primera clase, y
**sin nombres de productos de terceros en artefactos del método** (los nombres
viven en datos y research, no en specs).

## Goals / Non-Goals

**Goals:**
- Un catálogo consultable: registro conocido × detección local, con nivel
  declarado y estatus por agente.
- Selección de agente por **id** en la config del proyecto, retrocompatible.
- Superficies: RPC `fleet/list`, subcomando CLI `fleet`, vista Flota poblada.
- Todo testeable sin red y sin agentes reales (registro sustituible).

**Non-Goals:**
- Verificar niveles (suite de conformidad: #10). Aquí el nivel es *declarado*.
- Reglas de permisos por agente (#9), passthrough MCP (#13).
- Instalar/actualizar agentes (BYO-agent, siempre).
- Refresco del registro por red (v0.1 sin red; ver D1).

## Decisions

### D1 — Instantánea empaquetada del registro, sin red
El catálogo se puebla desde una **instantánea versionada del registro público
ACP** embebida en el binario (`include_str!` de un TOML de datos), curada en cada
release desde el registro público + el research interno. **Ningún acceso a red**
en v0.1: ni al arrancar, ni al listar. Un refresco manual explícito (comando que
consulta el registro público bajo demanda) queda como delta futuro.
- **Por qué**: local-first honesto; evita un cliente HTTP completo (árbol de
  dependencias desproporcionado frente a la constitución §10); CI hermética.
- **Sustituible**: `MELTEMI_FLEET_REGISTRY=<ruta>` (y clave equivalente en config)
  reemplaza la instantánea por un archivo local — la palanca de tests e2e (un
  registro fixture que apunta a `mock-agent`) y de usuarios avanzados.

### D2 — Detección local pasiva: presencia, jamás ejecución
Detectar = resolver el binario de cada entrada en `PATH` (más rutas candidatas
declaradas en la entrada), devolviendo ruta absoluta. En Windows se honran las
extensiones ejecutables (`.exe`, `.cmd`, `.bat` — muchos CLI de agentes son shims
de npm). La detección **nunca ejecuta** el binario (ni `--version`): sin efectos
laterales, sin latencia, sin sorpresas de seguridad. La versión real del agente
llega por el handshake ACP cuando se usa (ya existe en `initialize` de ACP).
- **Alternativa rechazada**: sondear `--version` por agente — lanza N procesos
  ajenos por un listado; costo y riesgo sin necesidad para v0.1.

### D3 — Contrato mínimo: `fleet/list`
Un único método nuevo (aditivo): request `fleet/list` con `projectRoot` opcional
→ `{ registryVersion, agents: [FleetAgent] }` donde `FleetAgent` incluye: `id`,
`displayName`, `source` (`registry` | `custom`), `integrationLevel` (1–4,
declarado), `detected` (bool), `binaryPath?` (si detectado), `configured` (bool,
si `projectRoot` se pasó y la config del proyecto lo selecciona). Cada llamada
re-escanea (la detección es barata, D2): el resultado refleja el presente.

### D4 — Selección por id, retrocompatible
`[agent] id = "<id-del-catalogo>"` en config (proyecto o usuario) como
alternativa a `command`. Resolución al abrir sesión: el id se busca en el
catálogo → si está detectado, argv = binario detectado + args ACP de la entrada.
**Precedencia** (de mayor a menor): `MELTEMI_AGENT_COMMAND` > `command` literal >
`id`. Un id no detectado o inexistente produce el error de aplicación **2001
`agent_not_detected`** (familia 2000 de config) con `remedy` accionable; no se
lanza nada.

### D5 — TUI: la Flota se puebla vía el actor de conexión
Nuevo `Command::FleetList` del shell al actor y `Update::Fleet(rows)` de vuelta;
se solicita al entrar a la vista 4 (y bajo demanda). Render con la línea base de
accesibilidad: estado de detección como glifo+palabra, nivel como etiqueta
(`L1`–`L4` + palabra), marcador del agente configurado; con cero detectados, la
tabla muestra el registro igualmente (qué se puede orquestar) conservando la
pista BYO-agent — el estado vacío de `tui-shell` se satisface por contenido, no
por pantalla muda. La paleta registra `fleet` (obligación ya viva en
`tui-shell`: todo método nuevo se registra).

### D6 — Neutralidad de marca en artefactos; datos con nombres
Specs y design no nombran productos de terceros (política del proyecto). Los
nombres reales viven en (a) la instantánea de datos (interoperabilidad factual,
como cualquier registro) y (b) el research interno. Las tasks pueblan la
instantánea **desde** el research sin copiar nombres a los artefactos del método.

## Risks / Trade-offs

- **La instantánea envejece** entre releases → `registryVersion` visible en
  `fleet/list` y en la vista; refresco manual como delta futuro; entradas
  `custom` cubren lo que falte hoy.
- **Falsos no-detectados** (instalaciones exóticas fuera de PATH) → rutas
  candidatas por entrada + agentes `custom` + `command` literal siempre
  disponible.
- **PATH en Windows** (shims `.cmd`) → detección con PATHEXT acotado y tests
  específicos de plataforma.
- **Nombres de terceros en datos públicos** → posición documentada (D6):
  factual, revisable por el mantenedor antes del repo público.

## Migration Plan

Aditivo: método nuevo, campos nuevos de config (opcionales), vista poblada.
`command` literal y `MELTEMI_AGENT_COMMAND` siguen funcionando sin cambios.
Reversión: retirar el módulo de catálogo; la config antigua no se toca.

## Open Questions

- Ubicación exacta y licencia de la instantánea de datos en el repo público
  (revisión del mantenedor antes de #21/#22).
- Si `fleet/list` debe paginarse — improbable a esta escala (≤ decenas); se
  decide al medir.
