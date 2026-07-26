# Meltemi — contexto para agentes

La constitución y el rumbo se **proyectan automáticamente** al bloque gestionado al pie de este archivo (`meltemi project`, dogfooding de meltemi.md §2.8); no los copies a mano. Todo lo que está fuera del bloque es contexto operativo mantenido a mano.

## Qué es este proyecto

Meltemi: plano de control spec-driven open source (Apache 2.0) que orquesta agentes de codificación externos vía ACP. Daemon headless `meltemid` (Rust) + TUI `meltemi` + GUI Tauri (fase 2). Documento fundacional: `meltemi.md` (v1.3 enmendada; ratificación de v1.2/v1.3 pendiente del mantenedor fundador; base v1.0 y constitución/rumbo ratificados 2026-07-11). La edición de código es *utilitaria al servicio del bucle agéntico*, acotada por la spec de gobernanza `edit-surface`; el compañero móvil (fase 3) está acotado por `mobile-companion`. Backlog maestro: `docs/plan-de-cambios.md`.

Workspace Cargo en la raíz: `core/meltemid` (daemon), `core/meltemi-spec` (motor de specs), `core/mock-agent` (agente ACP simulado para e2e), `proto/meltemi-proto` (tipos del contrato), `tui/` (binario `meltemi`: CLI scriptable + TUI). Toolchain pineado en `rust-toolchain.toml` (1.97.0).

## Reglas no negociables (constitución — resumen operativo)

1. **Spec-first**: nada se implementa sin propuesta de cambio aprobada en `openspec/changes/` (método actual; ver bootstrap abajo). Los escenarios de las specs son la definición de "terminado".
2. **Juego limpio**: solo binarios oficiales de agentes con su propia auth. Prohibido leer/almacenar credenciales ajenas o suplantar clientes.
3. **Seguridad**: daemon solo en socket local; deny-by-default sin cliente; sin puertos de red, jamás.
4. **Paridad de núcleo**: ninguna feature del daemon accesible desde una sola superficie.
5. **Calidad**: `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las 3 plataformas antes de merge. Windows es primera clase.
6. **Sin telemetría**: métricas solo locales; cualquier telemetría futura es opt-in y especificada antes de existir.

## Convenciones

- **Idiomas**: artefactos del método en español neutro; código, identificadores, strings del contrato `proto/` y mensajes de commit en inglés.
- **Commits**: atómicos, uno por tarea, con referencia `(<change> <tarea>)`. **Sin trailers de co-autoría.**
- **Dependencias**: mínimas, pineadas, justificadas en el design de su change (auditoría con cargo-deny en CI).
- **Licencia**: Apache-2.0; todo archivo fuente lleva cabecera SPDX (`docs/politica-spdx.md`).
- **Tests e2e**: siempre contra repos fixture temporales, nunca contra la raíz de este repo. En CI se usa `mock-agent`, nunca agentes reales ni red.

## Bootstrap del método — etapa 1 cerrada

El motor de specs de fase 1 está operativo y hospeda las specs del propio proyecto: la **verdad viva vive en `.meltemi/specs/`** y el histórico en `.meltemi/changes/archive/` (migrados desde `openspec/` con verificación del motor, `migracion-openspec-a-meltemi`). El método del proyecto son los comandos de Meltemi (`propose`/`plan`/`review`/`verify`/`archive`/`implement`) sobre `.meltemi/`; toda enmienda fundacional entra como change en `.meltemi/changes/`. El árbol `openspec/` se conserva como histórico consultable hasta que el mantenedor confirme su retiro físico (design D3 de la migración; D9 de `fase-0-fundacion` cumplido).

## Referencias

- `meltemi.md` — visión y decisiones D1–D6
- `.meltemi/constitution.md` + `.meltemi/rumbo/{product,tech,structure}.md`
- `docs/plan-de-cambios.md` — backlog ordenado de changes
- `docs/research/integracion-agentes.md` — matriz de integración por agente (interno)

<!-- meltemi:context:begin sha256=177a9a71137dcc5127f84b8ffe446f1b9acaf82666c0c34dda62f2994e6f8b88 -->
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

**Qué es Meltemi**: el plano de control spec-driven para el desarrollo agéntico. Open source (Apache 2.0), gratuito, de la comunidad. Orquesta los agentes de codificación que el usuario ya tiene (vía ACP y proyección de contexto), bajo una disciplina donde ninguna línea de código se escribe sin una especificación revisada.

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

## Cambio activo: motor-propio-byok

## Why

El mantenedor pide tres cosas: una vía de referencia para modelos
autohospedados (ollama en `http://localhost:11434` o cualquier endpoint
OpenAI-compatible), de modo que "un agente Meltemi con el harness de
Meltemi" corra contra un modelo local, con BYOK para los hospedados; el
harness como concepto de primera clase con un default; y una decisión sobre
su proyecto Forge Harnesses. La forma ya estaba escrita: meltemi.md D6
promete que el motor propio "entra a la flota como un agente más… jamás un
canal privilegiado", y esa frase decide la arquitectura completa. El motor
es un binario ACP de nivel 1 que meltemid pilota por stdio exactamente como
pilota a gemini-cli — mismas specs, mismo proxy de permisos, mismos
worktrees y checkpoints — y todo su tráfico de red vive en el subproceso,
donde hoy vive el de toda la flota. Así el daemon no gana ni una línea de
HTTP (§3), la flota sigue siendo la única superficie de integración (§5) y
el motor queda genuinamente opcional (rumbo de producto). Vive en este
monorepo (`core/meltemi-engine`) porque es un hito de fase 2 de este
proyecto y merece su disciplina — dejarlo fuera arriesga la inanición del
hito D6 —, pero entra por la puerta pública del catálogo porque esa es la
prueba más fuerte de neutralidad: Meltemi trata a su propio motor
exactamente como trata a Codex.

## What Changes

- **Crate nuevo `core/meltemi-engine`** (binario `meltemi-engine`, modo ACP
  vía `meltemi-engine acp`): loader de harness, un solo dialecto de modelo
  en v1 — `openai-chat`, cliente HTTP mínimo sobre rustls, solo saliente,
  confinado al crate — y el bucle agéntico que mapea tool calls a
  operaciones ACP del lado cliente: lecturas, escrituras y permisos vuelven
  por el proxy existente de meltemid. El motor queda gobernado, no
  confiado, y jamás escucha en puerto alguno. El design deja por escrito la
  prueba que §6 exige (ningún estándar abierto cubre las APIs de
  inferencia; la superficie chat OpenAI-compatible es la interoperable de
  facto: ollama, llama.cpp, vLLM, LM Studio, OpenRouter; tampoco existe
  estándar de manifiestos de harness) y la lectura estricta de §3: meltemid
  no enlaza pila HTTP/TLS alguna — propiedad verificable por cargo-deny —
  para que este debate no reabra la puerta después.
- **Harness como manifiesto TOML v1**, el concepto de primera clase:
  `schema = 1`, nombre, `[model]` (dialect, base-url, model,
  `api-key = "${VAR}"` opcional), `[prompt]`, `[tools]` (allow/ask/deny que
  moldean lo que el motor pide; el proxy sigue decidiendo) y `[limits]`.
  Un modelo local y uno hospedado BYOK son el mismo esquema: solo cambian
  `base-url` y la presencia de la referencia de clave. Literales que
  parecen secretos se RECHAZAN con el remedio `${VAR}` (el lint de higiene
  existente de perfiles/MCP); ninguna clave se persiste, se loguea ni asoma
  en diagnósticos (§2).
- **Harness default embebido** (`include_str!`, como el registro de flota)
  apuntando a `http://localhost:11434/v1` — el único default que no
  privilegia a proveedor comercial alguno (§5). Sin modelo alcanzable, el
  motor rehúsa con diagnóstico y remedio ("nada escucha en
  localhost:11434 — inicia ollama o declara un harness"; o lista lo que el
  endpoint sí sirve); nunca degrada en silencio. Es un trade deliberado —
  onboarding más duro que un default hospedado — y esta propuesta lo
  defiende para que no se "arregle" a la ligera después.
- **Descubrimiento y listado sin RPC nuevos**: harnesses en
  `<config>/meltemi/harnesses/*.toml` y `.meltemi/harnesses/*.toml`
  (proyecto pisa usuario por nombre, misma precedencia que perfiles y MCP);
  `fleet/list` los anida bajo la entrada del motor con fuente
  embedded/user/project — campos aditivos, paridad ×3 heredada. El daemon
  valida forma e higiene de secretos; jamás interpreta semántica de
  endpoint o prompt (§5 literal). Una sesión que nombra el motor o un
  harness resuelve por el orden existente y lanza
  `meltemi-engine acp --harness <ruta>`; binario y harness efectivos quedan
  en el log de sesión como toda resolución de hoy.
- **Entrada de registro** `meltemi-engine` (nivel 1, `acp-args = ["acp"]`)
  más una extensión honesta: `bundled = true`, para que la detección sondee
  también el directorio hermano del meltemid en ejecución — el motor viaja
  en los mismos instaladores. Regla de gobernanza explícita en el design:
  toda capacidad motor↔daemon más allá de ACP base debe ser una extensión
  ACP abierta y documentada que cualquier agente tercero pueda implementar;
  nada puede colgar de `id == "meltemi-engine"`.
- **Forge Harnesses permanece como proyecto separado, conectado por
  contrato de importación.** Absorberlo tiene méritos reales (cambios
  atómicos, un solo CI), pero cada mérito choca con una regla: la
  constitución entera aplicaría a un laboratorio de iteración de prompts
  (spec-first por cada ajuste, clippy en 3 SO, Apache-2.0 + CLA desde el
  día uno), el CI de Meltemi jamás ejecuta agentes reales ni red — la
  absorción ni siquiera compra tests de integración — y arrastraría a
  Meltemi hacia el marketplace que su rumbo explícitamente no es; el
  registro comunitario ya tiene casa declarada en fase 3. El contrato: este
  repo publica el esquema del manifiesto versionado (JSON Schema + fixtures
  de conformidad) y Forge produce manifiestos que se prueban contra ellos;
  instalar uno es copiarlo al directorio de harnesses, donde el daemon lo
  valida y lo lista con su fuente. El daemon jamás descarga nada. Condición
  explícita: si algún día se absorbe, entra Apache-2.0 bajo CLA, sin
  excepciones.
- **Guía de modelos autohospedados** en docs (EN): las dos vías con
  honestidad — el motor propio con su harness, y la que ya funciona hoy sin
  código nuevo (OpenCode y Aider de la flota actual soportan proveedores
  ollama/OpenAI-compatible por su propia configuración), verificada contra
  versiones actuales antes de publicarse, no citada de memoria.

## Capabilities

### New Capabilities
- `own-engine`: el bucle agéntico del motor propio como agente ACP nivel 1
  — dialecto `openai-chat`, gobernado por el proxy, sin canal privilegiado.
- `harness-config`: manifiestos de harness v1, default embebido con rechazo
  diagnosticado, higiene `${VAR}`, descubrimiento y listado con fuente.

### Modified Capabilities
- `fleet-catalog`: + entrada `meltemi-engine` con detección `bundled`; +
  harnesses anidados bajo el motor en `fleet/list` (campos aditivos).
- `initial-docs`: + guía de modelos autohospedados verificada.

## Impact

- Workspace: crate nuevo `core/meltemi-engine` con la única dependencia
  nueva (cliente HTTP mínimo sobre rustls, confinada al crate y justificada
  en el design, §10 — el set de dependencias de meltemid no se mueve);
  `core/meltemid` (fila de registro, detección `bundled`, descubrimiento de
  harnesses), `proto/` (campos aditivos en `FleetAgent`), `tui/` y
  `desktop/ui` (render de harnesses y del rechazo del default), matriz de
  paridad, `rumbo/structure.md` (+ directorio), docs.
- Distribución: el binario hermano viaja en los instaladores; el QA de
  presupuesto de tamaño re-mide sus gates con el costo de rustls a la
  vista; el skew de versión meltemid↔motor se detecta en el handshake ACP
  con remedio, no se asume.
- Peaje asumido y nombrado: cada lectura, escritura y permiso del motor
  viaja por stdio ACP daemon↔motor, y el motor re-deriva contexto que el
  daemon ya tiene. Es el precio de "jamás un canal privilegiado"; el design
  lo declara, no se descubre después.
- Tests: el bucle del motor contra un `ModelTransport` fake en memoria; el
  dialecto HTTP contra un servidor de modelo fixture solo-loopback
  (127.0.0.1, puerto efímero, in-process, sólido en los 3 SO); un e2e de
  workspace donde meltemid pilota el binario real contra ese fixture. CI
  sigue sin red externa ni agentes reales; los e2e del daemon siguen contra
  mock-agent, intactos.
- Analítica: las sesiones del motor leen "no reportado por el protocolo"
  hasta que exista una extensión ACP abierta de usage — irónico para el
  motor propio, y deuda declarada, no escondida (frontera de honestidad de
  analitica-consumo-local).

## Fuera de alcance

- Cliente HTTP o motor dentro de meltemid — jamás: destruiría la propiedad
  auditable de que el daemon no enlaza red (§3) y haría al núcleo asumir
  proveedores (§5).
- `sandbox-propio`: change propia, ya listada en el plan.
- Dialectos adicionales de modelo (cada uno con su prueba §6 escrita) y
  cliente MCP nativo en el motor (meltemi.md lo asigna al motor, pero es
  separable).
- Cambio de modelo o harness in-sesión: los session modes de ACP no están
  cableados hoy; se promueve con evidencia de demanda y por la vía ACP
  (§6), no por RPC propio.
- Verbo `meltemi engine import` y toda historia de registro, firmado o
  descarga de Forge: el daemon jamás descarga nada; en v1 instalar un
  harness es copiar un archivo que el daemon valida y lista al
  descubrirlo. El verbo de conveniencia es fast-follow si se pide.
- Hot-reload de harness y hooks de evaluación: presión de extensión sobre
  la frontera ACP; futuro con evidencia, nunca canal privado.
- Extensión de usage para el panel de analítica: change futura como
  extensión ACP abierta y documentada.
- Absorber Forge Harnesses; si algún día ocurre, entra Apache-2.0 bajo CLA,
  sin excepciones (§12).

<!-- meltemi:context:end -->
