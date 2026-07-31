@AGENTS.md

<!-- meltemi:context:begin sha256=eb95a41717bdbbcb77f413a00f7bcd6751e85f04addb8f6b6a582ba15257ee9f -->
# Meltemi — contexto proyectado

_Compilado desde `.meltemi/` por `meltemi project`. El contenido del bloque gestionado se regenera; no editarlo a mano._

## Constitución

# Constitución de Meltemi

> **Estado: RATIFICADA v1.0** — 11 de julio de 2026, por Guillmar Ortiz (`fase-0-fundacion` 1.2).
> Estos son los principios no negociables del proyecto. Se inyectan como contexto en toda propuesta de cambio y en toda sesión de agente que trabaje sobre este repositorio. Toda modificación de este documento requiere una propuesta de cambio aprobada.

## Principios

### 1. Spec-first, proporcional
Ninguna funcionalidad se implementa sin una propuesta de cambio aprobada (proposal → design → specs → tasks). Los cambios triviales usan la vía rápida (`fast-forward`: todos los artefactos de una vez), nunca la vía nula. Los escenarios de las specs son la definición de "terminado": cada escenario debe quedar cubierto por un test o una verificación documentada.

### 2. Juego limpio — innegociable
Meltemi ejecuta únicamente los binarios oficiales de los agentes, con la autenticación que cada agente gestiona. Prohibido: leer, almacenar o reutilizar credenciales de agentes; suplantar el tráfico o la identidad de otro cliente; empaquetar agentes de terceros sin permiso expreso de su licencia. Ante la duda, la respuesta es no.

### 3. Seguridad por defecto
El daemon escucha solo en socket local con permisos exclusivos del usuario; el acceso remoto es únicamente vía túnel SSH. Sin cliente conectado, toda petición de permiso se deniega. Los agentes operan en worktrees aislados. Las acciones con efectos externos irreversibles requieren aprobación explícita incluso en modo autónomo.

### 4. Paridad de núcleo
Toda capacidad nueva del daemon debe ser consumible desde la TUI y la GUI por igual. Está prohibido añadir al daemon funcionalidad accesible desde una sola superficie.

### 5. Agnosticismo de agente y de modelo
El núcleo no asume ningún proveedor. Ninguna dependencia del workspace puede requerir una cuenta o clave de un proveedor concreto para compilar o pasar los tests (los tests e2e usan el agente simulado).

### 6. Estándares abiertos primero
ACP para pilotar agentes, MCP para herramientas, LSP para inteligencia de código, JSON-RPC 2.0 para transporte. Antes de inventar un protocolo o formato propio, hay que demostrar por escrito que ningún estándar abierto lo cubre.

### 7. Calidad verificable
`cargo clippy -- -D warnings`, `cargo fmt --check` y la suite de tests deben pasar en las tres plataformas (Windows, macOS, Linux) antes de cualquier merge. Windows es plataforma de primera clase, no un puerto posterior.

### 8. Trazabilidad
Un commit atómico por tarea; el mensaje referencia la change y la tarea (`fase-0-fundacion 3.2`). Cada línea de código debe poder rastrearse hasta el requisito que la originó.

### 9. Sin telemetría oculta
Toda métrica se calcula en local. Cualquier telemetría futura será opt-in, desactivada por defecto, y su contenido exacto estará especificado públicamente antes de existir.

### 10. Dependencias mínimas y pineadas
Cada dependencia nueva se justifica en el design de la change que la introduce. Versiones pineadas; auditoría de licencias y vulnerabilidades en CI.

### 11. Idioma
Documentación de producto y artefactos del método: español neutro internacional. Código, identificadores, mensajes de commit y comentarios: inglés (el estándar de la comunidad global). Los textos de cara al usuario final se diseñan para internacionalización desde el inicio (español e inglés como primeros idiomas).

### 12. Apache 2.0, para siempre
El núcleo, los clientes y el SDK son Apache 2.0 y no cambiarán de licencia. Ninguna contribución se acepta bajo términos que comprometan esta promesa.

## Rumbo

### product

# Rumbo: Producto

> **Enmienda pendiente de ratificación** — 31 de julio de 2026
> (`lanzador-conversacional`, design D5). El párrafo «Qué es Meltemi» ya no
> presenta la especificación revisada como condición previa, sino el gobierno
> de toda sesión como piso y la disciplina spec-first como el camino más
> corto. La firma del mantenedor fundador es **gate de archivo** de esa change;
> mientras no llegue, ese párrafo está aplicado pero no ratificado. El resto
> del documento sigue ratificado el 11 de julio de 2026.

**Qué es Meltemi**: el plano de control spec-driven para el desarrollo agéntico. Open source (Apache 2.0), gratuito, de la comunidad. Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y proyección de contexto): toda sesión corre gobernada —proxy de permisos con deny-by-default, registro apend-only y punto de restauración declarado— y sobre ese piso la disciplina spec-first está siempre a un gesto: proponer, planificar y verificar viven en el mismo compositor donde se empieza a trabajar. La especificación revisada es el estándar que Meltemi hace fácil de sostener, no el peaje que impide empezar.

**Qué NO es**: ni un editor de propósito general (la superficie de código admite edición utilitaria al servicio del bucle agéntico; la autoría sostenida vive en el editor del usuario), ni otro agente de codificación (el motor propio de fase 2 es opcional), ni un servicio en la nube, ni CI/CD, ni un marketplace.

**Para quién (MVP, en orden)**: (1) el desarrollador individual que ya usa y paga agentes CLI y trabaja en terminal; (2) el tech lead que quiere disciplina de specs sin imponer herramientas; (3) mantenedores open source en bases de código maduras.

**El lema**: "Un rumbo, muchas velas." Una spec clara impulsa cualquier número de agentes, de cualquier fabricante, sin atarse a ninguno.

**Principio comercial**: no hay créditos, ni tarifas, ni lock-in. BYO-agent, BYOK, BYO-modelo.

**Referencia completa**: `meltemi.md` (documento fundacional, versión 0.2).

### structure

# Rumbo: Estructura y convenciones

**Monorepo** (destino; se materializa en `fase-0-fundacion`):

```
meltemi/
├── core/meltemid/     # binario del daemon (Rust)
├── core/mock-agent/   # agente ACP simulado para tests e2e
├── proto/             # JSON Schemas del contrato + crate meltemi-proto
├── tui/               # cliente de terminal `meltemi` (fase 1)
├── desktop/           # cliente GUI Tauri (fase 2)
├── sdk/               # SDK público (fase 2)
├── brand/             # identidad visual (V2 vigente; ver brand/README.md)
├── docs/              # documentación y research interno
├── .meltemi/          # constitución, rumbo y (a futuro) specs del propio proyecto
└── openspec/          # método SDD actual del proyecto (ver nota de migración)
```

**Método de trabajo (dogfooding en dos etapas)**: hasta que Meltemi pueda hospedar sus propias specs, el proyecto se desarrolla con OpenSpec (`openspec/changes/`, comandos `/opsx:*`). La constitución y el rumbo ya viven en `.meltemi/` (formato destino). Cuando el motor de specs de fase 1 esté operativo, se migrarán las specs vivas de `openspec/specs/` a `.meltemi/specs/` mediante una change dedicada.

**Convenciones**:
- Changes en kebab-case; un commit atómico por tarea con referencia `(<change> <tarea>)`.
- Código, identificadores y commits en inglés; artefactos del método en español neutro.
- Los escenarios de spec (`#### Scenario:`) son la fuente de los nombres de tests.
- Nada se implementa si no está en la change activa; lo que surja se anota como propuesta futura, no se cuela.

### tech

# Rumbo: Stack técnico y restricciones

**Lenguaje**: Rust estable (toolchain pineado en `rust-toolchain.toml`). Un solo lenguaje de sistemas en todo el producto.

**Arquitectura**: daemon headless `meltemid` (toda la lógica) + clientes finos (TUI `meltemi`, GUI Tauri) vía JSON-RPC 2.0 con delimitación por líneas sobre socket local (UDS 0700 en macOS/Linux; named pipe con ACL de usuario en Windows). Sin puertos de red, jamás.

**Dependencias clave (pineadas)**: `tokio` (runtime async), crate oficial del Agent Client Protocol (integración de agentes), `serde` (tipos del contrato `proto/`), `directories` (rutas por plataforma). Toda dependencia nueva se justifica en el design de su change.

**Contrato**: los JSON Schemas de `proto/` son la fuente de verdad del protocolo daemon↔clientes; los tipos Rust de `meltemi-proto` deben pasar el test de conformidad contra ellos.

**Persistencia**: logs de sesión JSONL apend-only en el directorio de datos del usuario; artefactos del método en `.meltemi/` dentro de cada repositorio.

**Plataformas soportadas**: Windows 10 1809+ / Windows 11, macOS 13+, Linux (glibc mainstream). CI obligatorio en las tres; Windows es primera clase.

**Calidad**: `cargo clippy -- -D warnings`, `cargo fmt --check`, tests por escenario de spec. Los e2e de CI usan `mock-agent` (nunca agentes reales ni red).

**Prohibiciones**: credenciales de agentes (ni leerlas ni tocarlas); transporte de red en el daemon; dependencias que exijan cuentas de proveedores para compilar o testear; features del daemon accesibles desde una sola superficie.

## Cambio activo: adaptadores-propios-acp

# adaptadores-propios-acp

## Why

El mantenedor lo decidió sin ambigüedad: «no quiero ACP (adapters) de
terceros. Revisa los que tenemos y construimos los propios». Esta change ya
no evalúa la opción — la ejecuta: los adaptadores propios **son** los puntos
de pilotaje que Meltemi distribuye, empaquetados en los mismos instaladores,
y reemplazan a los adaptadores de terceros como capa por defecto del
registro para las dos entradas de nivel 2 (Claude Code y Codex).

La decisión revierte una registrada: `niveles-integracion-conformidad` dejó
fuera de alcance «Mantener adaptadores propios: se consumen adaptadores
abiertos existentes». Aquella decisión fue correcta con el ecosistema de
entonces; cuatro hechos la vencieron. Primero, el suelo se movió: Zed
archivó su `codex-acp` en Rust el 2026-07-22 y los adaptadores canónicos
viven hoy en TypeScript bajo la org `agentclientprotocol` — consumirlos
significa un runtime Node en la distribución, contra el rumbo de un solo
lenguaje de sistemas, y la opción Rust upstream murió. Segundo, la zona
gris del adaptador de Claude es permanente y no depende de quién lo
mantiene: los términos de Anthropic (feb-2026) nombran al Agent SDK como no
autorizado para OAuth de suscripción, el adaptador canónico envuelve
exactamente ese SDK, y un fork heredaría la misma exposición — mientras que
el camino seguro que el research ya nombró (pilotar el binario oficial
`claude` donde el usuario ya hizo login) no lo toma ningún adaptador del
ecosistema. Tercero, los adaptadores propios viajan `bundled` en los
instaladores y matan el muro de onboarding «adaptador no detectado» que
`flota-deteccion-guia` diagnosticó: instalar Meltemi más el CLI oficial del
proveedor basta, sin `npm i -g` intermedio de un tercero. Cuarto, la
directiva del mantenedor, que convierte lo anterior en rumbo y no en
alternativa. La reversión queda argumentada por escrito en el design, con
fechas y fuentes; una decisión registrada solo se revierte con otra
decisión registrada.

## What Changes

- **Crate nuevo `core/meltemi-adapters`**: una librería de puente ACP
  compartida y **dos binarios**, `meltemi-claude-acp` y `meltemi-codex-acp`
  — nombres distintos de los binarios de terceros (`claude-agent-acp`,
  `codex-acp`) para que jamás colisionen en el PATH. **Cero dependencias
  nuevas en el workspace**: tokio, serde y el crate oficial
  `agent-client-protocol` ya son dependencias pineadas. Ninguno de los dos
  adaptadores enlaza pila HTTP/TLS alguna — a diferencia del motor propio,
  aquí no hay rustls que justificar: ambos lanzan el CLI oficial del
  proveedor como subproceso y hablan JSON delimitado por líneas por stdio;
  toda la red y toda la auth viven en el binario oficial, donde §2 las
  exige.
- **Adaptador de Claude** (`meltemi-claude-acp`): pilota el **binario
  oficial `claude` con la sesión que el usuario ya inició**, vía `-p
  --input-format stream-json --output-format stream-json
  --include-partial-messages` — deltas de tokens y transcripts de
  subagentes dan casi-paridad de streaming; `--resume` y `--fork-session`
  funcionan headless con ámbito de directorio de proyecto y sus worktrees,
  que calza con el modelo de Meltemi; `--mcp-config` recibe la proyección
  de perfiles MCP existente. Jamás el Agent SDK, jamás `--bare`: el flip
  anunciado de `--bare` como default de `-p` (mataría el OAuth en silencio)
  queda **pineado como riesgo en el design**, con detección de features vía
  el arreglo `capabilities` de `system/init` — que existe para exactamente
  esto — y rehúso diagnosticado si la superficie con sesión iniciada no
  está, nunca cambio silencioso a modo de clave de API.
- **Permisos de Claude con passthrough real y pérdidas declaradas**:
  `--permission-prompt-tool` apunta a un shim MCP mínimo por stdio que el
  propio adaptador hospeda (el mismo binario en modo shim, conectado al
  proceso padre por un canal privado); cada petición se releva a
  `session/request_permission` de la sesión ACP del adaptador, que meltemid
  proxya a la bandeja humana como toda petición de hoy — **el daemon no
  gana transporte alguno**. Hooks `PreToolUse` como compuerta dura, que
  deniega incluso en `bypassPermissions`. Lo que se pierde se escribe: el
  prompt-tool solo se consulta cuando ninguna regla estática decide; las
  herramientas `requiresUserInteraction` y `AskUserQuestion` se
  auto-deniegan en modo no interactivo y la sesión lo muestra con motivo,
  no lo esconde; el contrato del prompt-tool está infradocumentado upstream
  (issue #1175). El canal `canUseTool` del SDK existe en el mismo cable
  pero no está documentado: construir sobre él violaría el espíritu de §6,
  así que no.
- **Adaptador de Codex** (`meltemi-codex-acp`): lanza el CLI oficial
  `codex` en modo `app-server` — JSON-RPC 2.0 con delimitación por líneas
  sobre stdio, documentado, la misma interfaz que usa la extensión VS Code
  del propio proveedor. El esquema por versión se vuelca con `codex
  app-server generate-json-schema`, y eso es una historia de conformidad
  lista: los tipos del adaptador se prueban contra fixtures del esquema, la
  disciplina que `proto/` ya practica. Explícitamente **no** el patrón de
  los adaptadores Rust archivados (Zed) y comunitarios: embeber
  `codex-core` como librería hace que el adaptador mismo haga red y lea el
  almacén de auth de Codex — choca con §2 aunque toda la cadena sea
  Apache-2.0.
- **Flip del registro**: las filas `claude-code` y `codex-cli` cambian su
  capa adaptador a los binarios propios con `bundled = true`. La detección
  de capa empaquetada — sondear también el directorio hermano del meltemid
  en ejecución — la describe la propuesta de `motor-propio-byok` pero no
  está implementada (esa change tiene solo proposal): **esta change la
  implementa como mecanismo genérico del registro y el motor la hereda**.
  Las capas `cli-bin` no se tocan (los CLIs oficiales se siguen detectando
  aparte); mueren los `adapter-install` de terceros de esas filas, y el
  remedio de una capa empaquetada ausente remite a reinstalar o reparar
  Meltemi, no a un `npm i -g` ajeno. El estatus legal se reescribe **sin
  maquillaje** para describir la arquitectura nueva con verdad: Claude
  sigue en gris — la vía ofrecida es ahora exactamente el camino seguro que
  el research nombró, pero Anthropic no ha publicado bendición alguna de
  orquestadores terceros sobre `claude -p`, y la nota lo dice — jamás
  «sancionado»; Codex sigue tolerado y mejora, porque OpenAI publicó
  app-server precisamente para terceros y desaparece la dependencia de
  supply chain sobre un repo archivado.
- **La vía de terceros no se prohíbe, deja de recomendarse**: quien
  prefiera un adaptador de terceros lo declara por configuración (entrada
  `custom` o `command` literal) y el daemon lo pilota como cualquier otro,
  sin trato distinto. La guía documenta la receta con su nota legal; el
  registro ya no la recomienda.
- **La prueba que §6 exige, por escrito en el design**: del lado Meltemi el
  estándar es ACP, hablado con el crate oficial que ya es dependencia del
  workspace. Del lado proveedor no existe estándar que cubra el pilotaje
  programático de estos agentes: stream-json y app-server son la superficie
  programática oficial de cada proveedor, cada una documentada por su
  dueño. El adaptador es exactamente eso — traducción entre el estándar
  abierto y la superficie oficial del proveedor — y ninguna capacidad puede
  colgar de un canal no documentado.
- **`docs/agentes.md` se reescribe en lockstep** (el test de coherencia
  registro↔guía ya existe): la capa adaptador de estas entradas viaja con
  Meltemi, qué hacer si falta, y la receta de terceros por configuración.

## Capabilities

### New Capabilities
- `own-adapters`: los adaptadores ACP propios como puentes gobernados sobre
  el binario oficial de cada proveedor — dialecto de sesión headless de
  eventos JSON y dialecto de servidor JSON-RPC, permisos relevados al proxy
  vigente con compuerta dura y pérdidas visibles, conformidad por versión,
  sin pila de red y sin canal privilegiado. (Una sola capability y sin
  nombres de terceros en specs: los nombres de productos viven en el
  registro como datos factuales, nunca en la verdad viva — la regla que el
  propio registro declara.)

### Modified Capabilities
- `fleet-catalog`: + detección de capa empaquetada junto al daemon
  (genérica, keyed en `bundled`, el motor propio la hereda); + adaptador
  propio como punto de pilotaje por defecto de las entradas de nivel 2 con
  la vía de terceros preservada por configuración; el remedio por capa
  distingue capas empaquetadas (reinstalar Meltemi) de capas instalables.
- `integration-levels`: la suite de conformidad de nivel 2 se ejerce en CI
  a través de los adaptadores propios pilotando procesos proveedor
  simulados; agentes reales siguen siendo manuales por opt-in. La reversión
  del fuera-de-alcance registrado queda asentada en el design de esta
  change.
- `initial-docs`: la guía de agentes explica la capa empaquetada y
  documenta la vía de terceros por configuración, sin inventar bendiciones.

## Impact

- Workspace: un crate nuevo `core/meltemi-adapters` (lib + dos binarios) y
  un crate de fixtures `core/mock-provider` (dos binarios de cable
  simulado), ambos sin dependencias externas nuevas — tokio, serde y
  `agent-client-protocol` ya están pineados (§10). `core/meltemid` (filas
  de registro, detección `bundled` genérica, remedios), `proto/` (campos
  aditivos en las capas de `FleetAgent`: procedencia empaquetada del
  hallazgo), `tui/` y `desktop/ui` (render de la fuente empaquetada y del
  remedio nuevo), matriz de paridad, docs.
- Distribución: dos binarios hermanos viajan en los instaladores; el QA de
  presupuesto de tamaño re-mide sus gates (sin rustls el costo es
  moderado, pero se mide, no se supone). El skew de versión adaptador↔CLI
  se detecta — `system/init capabilities` en Claude, esquema volcado por
  versión en Codex — con remedio, no se asume; el skew adaptador↔daemon lo
  cubre el handshake ACP.
- Orden con `pulido-pre-anuncio`: esa change aterriza ahora y refresca los
  `adapter-install` de terceros a sus distribuciones vigentes — honestidad
  para el usuario de hoy. Esta change reemplaza después la capa adaptador
  de esas mismas filas; no hay conflicto: el requisito de vigencia de rutas
  que pulido añade sigue aplicando a los comandos de instalación que
  sobreviven (los `cli-install` de los CLIs oficiales), y esta change
  documenta su propia verificación al revisar la instantánea.
- Tests: CI sigue sin red externa ni agentes reales. Cada adaptador se
  prueba contra un proceso fixture que habla el cable documentado de su
  proveedor (stream-json guionado; app-server JSON-RPC guionado — por
  stdio, sólido en los 3 SO, patrón mock-agent); un e2e de workspace donde
  meltemid pilota el binario real del adaptador contra ese fixture; la
  conformidad contra CLIs reales es manual y por opt-in, documentada vía
  verify-mark, como `niveles-integracion-conformidad` estableció.
- Peaje asumido y nombrado: los flujos `AskUserQuestion` de Claude se
  auto-deniegan en modo `-p` — límite del proveedor, se muestra en sesión
  con motivo; en Windows, Claude Code no trae sandbox nativo (solo WSL2),
  la compuerta dura son los hooks más el worktree hasta que
  `sandbox-propio` exista; y el contrato del prompt-tool puede moverse bajo
  los pies (issue #1175) — riesgo pineado, con la detección de features
  como amortiguador.
- Honestidad legal: esta change no vuelve «sancionado» a Claude y la nota
  del registro no lo dirá jamás; si Anthropic publica una postura, la nota
  se actualiza con fuente, no con deseo.

## Fuera de alcance

- Forkear los adaptadores TypeScript: otro lenguaje, un runtime Node en la
  distribución, y en el caso Claude hereda intacta la zona gris del SDK. Su
  valor real es de material de referencia Apache-2.0 para la semántica del
  mapeo ACP↔eventos de sesión, y así se usa.
- Toda vía basada en el Agent SDK o en canales no documentados
  (`canUseTool`): §6, sin excepciones.
- Retirar la posibilidad de usar adaptadores de terceros: siguen pilotables
  por configuración del usuario, hoy y después; lo que muere es su lugar de
  capa recomendada en el registro.
- Adaptadores propios para otros agentes: cada uno exige su prueba §6 y su
  análisis legal propio, no se generaliza por analogía.
- Dialectos o modos adicionales de los CLIs pilotados (p. ej. `codex exec
  --json` como nivel 3 alternativo): la superficie elegida por dialecto es
  una, con su prueba escrita; otra superficie sería otra change.
- `sandbox-propio`: change propia, ya listada en el plan; aquí solo se
  nombra el hueco de Windows que agranda su urgencia.
- Auto-actualización o verbo de gestión de adaptadores: viajan con los
  instaladores y se actualizan con Meltemi; el daemon jamás descarga nada.

### Deltas

- `fleet-catalog`: 3 requisitos
- `initial-docs`: 1 requisito
- `integration-levels`: 1 requisito
- `own-adapters`: 6 requisitos

<!-- meltemi:context:end -->
