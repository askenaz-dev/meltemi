# motor-propio-byok

> Nota terminológica (2026-08-09, decisión del mantenedor): lo que esta
> propuesta llamaba «harness» pasa a llamarse **manifiesto del motor**
> (engine manifest) — archivos en `engines/*.toml`, capability
> `engine-manifest`, flag `--manifest`. La palabra «harness» nombra desde hoy
> el harness de agentes (rules, skills, agents, hooks) de la change
> `harness-global-y-por-agente`. Los dos conceptos son independientes: el
> manifiesto configura la conexión y conducta del motor propio (BYOK); el
> harness equipa a cualquier agente de la flota.

## Why

El mantenedor pide tres cosas: una vía de referencia para modelos
autohospedados (ollama en `http://localhost:11434` o cualquier endpoint
OpenAI-compatible), de modo que "un agente Meltemi con el manifiesto de
Meltemi" corra contra un modelo local, con BYOK para los hospedados; el
manifiesto del motor como concepto de primera clase con un default; y una
decisión sobre su proyecto Forge Harnesses. La forma ya estaba escrita:
meltemi.md D6 promete que el motor propio "entra a la flota como un agente
más… jamás un canal privilegiado", y esa frase decide la arquitectura
completa. El motor es un binario ACP de nivel 1 que meltemid pilota por stdio
exactamente como pilota a gemini-cli — mismas specs, mismo proxy de permisos,
mismos worktrees y checkpoints — y todo su tráfico de red vive en el
subproceso, donde hoy vive el de toda la flota. Así el daemon no gana ni una
línea de HTTP (§3), la flota sigue siendo la única superficie de integración
(§5) y el motor queda genuinamente opcional (rumbo de producto). Vive en este
monorepo (`core/meltemi-engine`) porque es un hito de fase 2 de este
proyecto y merece su disciplina — dejarlo fuera arriesga la inanición del
hito D6 —, pero entra por la puerta pública del catálogo porque esa es la
prueba más fuerte de neutralidad: Meltemi trata a su propio motor
exactamente como trata a Codex.

## What Changes

- **Crate nuevo `core/meltemi-engine`** (binario `meltemi-engine`, modo ACP
  vía `meltemi-engine acp`): loader del manifiesto, un solo dialecto de
  modelo en v1 — `openai-chat`, cliente HTTP mínimo sobre rustls, solo
  saliente, confinado al crate — y el bucle agéntico que mapea tool calls a
  operaciones ACP del lado cliente: lecturas, escrituras y permisos vuelven
  por el proxy existente de meltemid. El motor queda gobernado, no
  confiado, y jamás escucha en puerto alguno. El design deja por escrito la
  prueba que §6 exige (ningún estándar abierto cubre las APIs de
  inferencia; la superficie chat OpenAI-compatible es la interoperable de
  facto: ollama, llama.cpp, vLLM, LM Studio, OpenRouter; tampoco existe
  estándar de manifiestos de motor de inferencia) y la lectura estricta de
  §3: meltemid no enlaza pila HTTP/TLS alguna — propiedad verificable por
  cargo-deny — para que este debate no reabra la puerta después.
- **Manifiesto del motor como TOML v1**, el concepto de primera clase:
  `schema = 1`, nombre, `[model]` (dialect, base-url, model,
  `api-key = "${VAR}"` opcional), `[prompt]`, `[tools]` (allow/ask/deny que
  moldean lo que el motor pide; el proxy sigue decidiendo) y `[limits]`.
  Un modelo local y uno hospedado BYOK son el mismo esquema: solo cambian
  `base-url` y la presencia de la referencia de clave. Literales que
  parecen secretos se RECHAZAN con el remedio `${VAR}` (el lint de higiene
  existente de perfiles/MCP); ninguna clave se persiste, se loguea ni asoma
  en diagnósticos (§2).
- **Manifiesto default embebido** (`include_str!`, como el registro de
  flota) apuntando a `http://localhost:11434/v1` — el único default que no
  privilegia a proveedor comercial alguno (§5). Sin modelo alcanzable, el
  motor rehúsa con diagnóstico y remedio ("nada escucha en
  localhost:11434 — inicia ollama o declara un manifiesto"; o lista lo que
  el endpoint sí sirve); nunca degrada en silencio. Es un trade deliberado —
  onboarding más duro que un default hospedado — y esta propuesta lo
  defiende para que no se "arregle" a la ligera después.
- **Descubrimiento y listado sin RPC nuevos**: manifiestos en
  `<config>/meltemi/engines/*.toml` y `.meltemi/engines/*.toml`
  (proyecto pisa usuario por nombre, misma precedencia que perfiles y MCP);
  `fleet/list` los anida bajo la entrada del motor con fuente
  embedded/user/project — campos aditivos, paridad ×3 heredada. El daemon
  valida forma e higiene de secretos; jamás interpreta semántica de
  endpoint o prompt (§5 literal). Una sesión que nombra el motor o un
  manifiesto resuelve por el orden existente y lanza
  `meltemi-engine acp --manifest <ruta>`; binario y manifiesto efectivos
  quedan en el log de sesión como toda resolución de hoy.
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
  instalar uno es copiarlo al directorio de manifiestos, donde el daemon lo
  valida y lo lista con su fuente. El daemon jamás descarga nada. Condición
  explícita: si algún día se absorbe, entra Apache-2.0 bajo CLA, sin
  excepciones.
- **Guía de modelos autohospedados** en docs (EN): las dos vías con
  honestidad — el motor propio con su manifiesto, y la que ya funciona hoy
  sin código nuevo (OpenCode y Aider de la flota actual soportan
  proveedores ollama/OpenAI-compatible por su propia configuración),
  verificada contra versiones actuales antes de publicarse, no citada de
  memoria.

## Capabilities

### New Capabilities
- `own-engine`: el bucle agéntico del motor propio como agente ACP nivel 1
  — dialecto `openai-chat`, gobernado por el proxy, sin canal privilegiado.
- `engine-manifest`: manifiestos del motor v1, default embebido con rechazo
  diagnosticado, higiene `${VAR}`, descubrimiento y listado con fuente.

### Modified Capabilities
- `fleet-catalog`: + entrada `meltemi-engine` con detección `bundled`; +
  manifiestos anidados bajo el motor en `fleet/list` (campos aditivos).
- `initial-docs`: + guía de modelos autohospedados verificada.

## Impact

- Workspace: crate nuevo `core/meltemi-engine` con la única dependencia
  nueva (cliente HTTP mínimo sobre rustls, confinada al crate y justificada
  en el design, §10 — el set de dependencias de meltemid no se mueve);
  `core/meltemid` (fila de registro, detección `bundled`, descubrimiento de
  manifiestos), `proto/` (campos aditivos en `FleetAgent`), `tui/` y
  `desktop/ui` (render de manifiestos y del rechazo del default), matriz de
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
- Cambio de modelo o manifiesto in-sesión: los session modes de ACP no
  están cableados hoy; se promueve con evidencia de demanda y por la vía
  ACP (§6), no por RPC propio.
- Verbo `meltemi engine import` y toda historia de registro, firmado o
  descarga de Forge: el daemon jamás descarga nada; en v1 instalar un
  manifiesto es copiar un archivo que el daemon valida y lista al
  descubrirlo. El verbo de conveniencia es fast-follow si se pide.
- Hot-reload de manifiesto y hooks de evaluación: presión de extensión
  sobre la frontera ACP; futuro con evidencia, nunca canal privado.
- Extensión de usage para el panel de analítica: change futura como
  extensión ACP abierta y documentada.
- Absorber Forge Harnesses; si algún día ocurre, entra Apache-2.0 bajo CLA,
  sin excepciones (§12).
