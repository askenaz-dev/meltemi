# Diseño: Fase 0 — Fundación de Meltemi

## Context

El repositorio no contiene código: solo el documento fundacional ([meltemi.md](../../../meltemi.md)), que ya fija las decisiones de arquitectura (núcleo headless en Rust + clientes finos, ACP como driver universal de agentes, juego limpio con binarios oficiales). Esta change materializa el esqueleto mínimo que valida esas decisiones de extremo a extremo.

Restricciones heredadas del documento fundacional y del rumbo del proyecto:
- **Juego limpio (§4.7)**: solo se ejecutan binarios oficiales de agentes, con la autenticación que cada agente gestiona; nada de suplantar tráfico ni tocar credenciales.
- **Seguridad del daemon (§8.1)**: socket local con permisos exclusivos del usuario; sin transporte de red.
- **Plataforma de desarrollo primaria: Windows** (constitución §7; `.meltemi/rumbo/tech.md`) — el soporte Windows no puede ser una ocurrencia tardía.

## Goals / Non-Goals

**Goals:**
- Probar la cadena completa: cliente JSON-RPC → `meltemid` → subproceso agente ACP → streaming de vuelta → petición de permiso → aprobación del cliente.
- Contrato daemon↔clientes versionado en `proto/` desde el primer día.
- `constitution.md` ratificada en `.meltemi/` (dogfooding).
- CI que compila, pasa lint y ejecuta el e2e con un agente simulado en las tres plataformas.

**Non-Goals:**
- Motor de specs (EARS, deltas, verdad viva) — fase 1.
- TUI real, proyección de contexto, worktrees, reglas persistentes de permisos, catálogo de flota — fase 1.
- Motor agéntico propio, sandbox propio, GUI — fase 2.

## Decisions

### D1 — Workspace Cargo con dos crates + esquemas contract-first
Workspace Rust en la raíz con `core/meltemid` (binario del daemon) y `proto/` conteniendo: (a) los **JSON Schemas** del protocolo daemon↔cliente como fuente de verdad neutral al lenguaje, y (b) el crate `meltemi-proto` con los tipos serde equivalentes. *Alternativa considerada*: generar los tipos desde los schemas — se difiere; en fase 0 la duplicación manual es pequeña y un test de conformidad valida que tipos y schemas coinciden.

### D2 — El transporte daemon↔cliente reutiliza el estilo de ACP
JSON-RPC 2.0 con delimitación por líneas sobre **socket local**: Unix domain socket (permisos `0700`) en macOS/Linux y **named pipe** con ACL restringida al usuario en Windows. *Alternativa considerada*: un framework JSON-RPC pesado — descartado; usar el mismo modelo mental (y utilidades) que ya exige ACP reduce dependencias y deja un solo patrón de mensajería en todo el sistema.

### D3 — Integración ACP con el crate oficial, meltemid como cliente ACP
`meltemid` usa el **crate oficial del Agent Client Protocol** (versión pineada) actuando como *cliente* ACP: lanza el binario del agente como subproceso stdio, ejecuta `initialize` → `session/new` → `session/prompt`, consume `session/update` en streaming y atiende `session/request_permission`. El agente a lanzar es **configurable por comando** (`agent.command` en la config); no se incluye ningún agente.

### D4 — Agente simulado para CI; agente real para la prueba manual
CI no puede depender de suscripciones ni de red (juego limpio + determinismo). Se añade `core/mock-agent`: un binario mínimo construido con el mismo crate ACP (lado agente) que responde con un guion fijo — incluida una petición de permiso — para el test e2e automatizado. El hito con **agente real** se ejecuta manualmente con cualquier agente ACP instalado por el desarrollador.

### D5 — Permisos: passthrough interactivo, sin reglas todavía
Cada `session/request_permission` del agente se reenvía tal cual al cliente conectado, que responde permitir/denegar. Sin persistencia de reglas (fase 1). Si no hay cliente conectado, la respuesta es **denegar** — el default seguro.

### D6 — Sesiones: registro JSONL apend-only
Cada sesión escribe un log de eventos JSONL en el directorio de datos del usuario (vía el crate `directories`), indexado por proyecto. Cumple "sesiones persistentes e inspeccionables" (§4.10) con el formato más simple posible de auditar y de re-leer.

### D7 — `/propose` mínimo = andamiaje + delegación al agente
El RPC `propose(idea)` hace dos cosas: (1) `meltemid` crea el esqueleto `.meltemi/changes/<kebab-name>/proposal.md` de forma determinista, y (2) envía al agente ACP un prompt corto para completar el `proposal.md` con la idea dada, con CWD en el repositorio. El agente escribe con sus propias herramientas y sus permisos fluyen por D5. La calidad del contenido no es objetivo de fase 0; la cadena completa, sí.

### D8 — Runtime y calidad
`tokio` como runtime async (es lo que asume el ecosistema ACP). CI en GitHub Actions con matriz {ubuntu, macos, windows}: `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` (incluye e2e con mock-agent).

### D9 — Bootstrap del método en dos etapas
El desarrollo de Meltemi se gobierna con OpenSpec (`openspec/changes/`) hasta que el motor de specs de fase 1 tenga `/archive` operativo; `.meltemi/` contiene desde ya la constitución y el rumbo (formato destino). La migración `openspec/ → .meltemi/` será una change dedicada (`migracion-openspec-a-meltemi`). La excepción al §9.3 de meltemi.md se ratifica vía la change `enmiendas-fundacionales-v1` (ver docs/plan-de-cambios.md).

### D10 — Versionado del contrato `proto/`
Los schemas de `proto/` llevan un `protocolVersion` entero. El cliente lo declara al conectar; el daemon acepta o responde error con ambas versiones. Cambios aditivos no incrementan la versión; cambios rompedores sí. La política completa vive en `proto/README.md` cuando se materialice la tarea 2.1.

### D11 — Taxonomía de errores
Errores de aplicación con códigos propios fuera del rango reservado JSON-RPC, agrupados por dominio (1xxx daemon, 2xxx sesión ACP, 3xxx propose). `error.data` estructurado: `{ kind, detail, remedy }`. Los mensajes del contrato van en inglés (el campo `remedy` permite a las superficies traducir/ampliar). El catálogo es parte de los schemas de la tarea 2.1.

### D12 — Logging del daemon y esquema de eventos de sesión
Crate `tracing` para el log operacional de `meltemid` (niveles configurables, destino en el directorio de datos del usuario, rotación) — imprescindible porque el daemon corre desacoplado y sin terminal. El evento JSONL de sesión tiene schema versionado en `proto/` (`{ v, ts, type, payload }`).

### D13 — Configuración (resuelve la Open Question)
Rutas por plataforma vía crate `directories` (Windows: `%APPDATA%\meltemi\config.toml`; macOS: `~/Library/Application Support/meltemi/`; Linux: `~/.config/meltemi/`). Precedencia: defaults < config de usuario < `.meltemi/config.toml` del proyecto < variables `MELTEMI_*` < flags de CLI. Clave de indexación de proyectos en el directorio de datos: hash de la ruta canónica de la raíz del repositorio.

### D14 — Pruebas contra repositorios fixture
Todo e2e — automatizado o manual — se ejecuta contra un repositorio fixture temporal, nunca contra la raíz del repo de Meltemi (evita que los andamiajes de prueba de `/propose` aterricen junto a la constitución real).

### D15 — La CLI de prueba es desechable
El cliente de la tarea 5.4 se llama `meltemi-devclient`, es tooling de desarrollo no distribuible, y será reemplazado por la CLI especificada en la change `cli-contrato` de fase 1.

### D16 — Dependencias concretas (registro de implementación)
Todas pineadas exactas (`=`) en `[workspace.dependencies]`; el Cargo.lock va commiteado. Justificación por crate: `agent-client-protocol` 1.2.0 (D3, crate oficial ACP); `tokio` 1.52.3 (D8, runtime); `serde`/`serde_json` (D1, tipos del contrato); `thiserror`/`anyhow` (errores de lib/bins); `tracing` + `tracing-subscriber` + `tracing-appender` (D12, log operacional con rotación); `directories` 6.0.0 (D13, rutas por plataforma); `toml` (D13, config); `sha2` (D13, hash de la ruta canónica del proyecto); `uuid` v4 (identificadores de sesión); `jsonschema` sin features por defecto (solo dev-dependency del test de conformidad D1; las features por defecto arrastran reqwest/rustls para resolución HTTP que no se usa); `libc` (permisos UDS en Unix); `windows-sys` (ACL del named pipe en Windows, D2).

## Risks / Trade-offs

- **[Churn del protocolo/crate ACP]** → versión pineada + los tipos que tocamos quedan encapsulados en un módulo `acp/` único; el test e2e con mock-agent actúa como suite de conformidad temprana (semilla de la "suite de conformidad" del roadmap).
- **[Semánticas distintas UDS vs named pipes]** → abstracción de transporte propia con tests por plataforma en CI; ninguna lógica fuera de ella toca el socket directamente.
- **[El e2e real depende de qué agente tenga instalado el dev]** → el contrato se prueba contra mock-agent en CI; el agente real es verificación manual documentada, no gate.
- **[Costo en tokens del e2e manual]** → prompts mínimos y documentados; el mock cubre el 95% de las iteraciones.
- **[Tentación de adelantar features de fase 1]** → el alcance lo fijan las specs de esta change; todo lo demás se rechaza en revisión.

## Migration Plan

Greenfield: no hay migración. Rollback = borrar `core/`, `proto/` y `.meltemi/` (la constitución se conserva vía git history).

## Open Questions

- ~~Ubicación final de la config~~ — resuelta en D13.
- Distribución del alias `mel` (symlink vs binario propio): irrelevante hasta fase 1; anotado para entonces.
