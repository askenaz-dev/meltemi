# adaptadores-propios-acp — design

## Context

El nivel 2 de la flota pilota Claude Code y Codex a través de adaptadores
ACP de terceros declarados en el registro. Ese suelo se movió: Zed archivó
su `codex-acp` en Rust el 2026-07-22 («Development has moved to
agentclientprotocol/codex-acp»), los adaptadores canónicos viven hoy en
TypeScript bajo la org neutral `agentclientprotocol`, y el adaptador de
Claude envuelve el Agent SDK que los términos de Anthropic (feb-2026)
nombran como no autorizado para OAuth de suscripciones de consumo. El
research (scratchpad de adaptadores, verificado 2026-07-27 contra GitHub,
npm y la documentación de cada proveedor) estableció la arquitectura; el
mantenedor dio la directiva: «no quiero ACP (adapters) de terceros. Revisa
los que tenemos y construimos los propios». Los adaptadores propios dejan
de ser una opción a evaluar: son los puntos de pilotaje que Meltemi
distribuye.

Esta change revierte el fuera-de-alcance que `niveles-integracion-conformidad`
dejó registrado — proposal: «Mantener adaptadores propios: se consumen
adaptadores abiertos existentes»; design: «mantener adaptadores propios (se
consumen los abiertos existentes, declarados en datos)» — y lo hace por
escrito, con fechas y fuentes (D1).

Interacción con changes vecinas: `pulido-pre-anuncio` (pendiente,
fast-forward) refresca los `adapter-install` de terceros a las
distribuciones vigentes bajo `@agentclientprotocol` — debe aterrizar antes,
por honestidad con el usuario de hoy (D11). `motor-propio-byok` (proposal
redactada, sin implementar) describe la detección de directorio hermano
para binarios `bundled`; como no existe en el código, esta change la
implementa como mecanismo genérico y el motor la hereda (D8).

## Goals / Non-Goals

**Goals:** dos adaptadores ACP propios en Rust, en este monorepo, que
pilotan exclusivamente los binarios oficiales `claude` y `codex` con la
auth que cada CLI gestiona; los adaptadores viajan empaquetados en los
instaladores y son la capa de pilotaje por defecto del registro para las
dos entradas de nivel 2; detección genérica de capas `bundled`; permisos
relevados al proxy vigente con compuerta dura y pérdidas visibles;
conformidad por versión; notas legales veraces; guía reescrita en
lockstep; CI sin red ni agentes reales.

**Non-Goals:** forkear los adaptadores TypeScript; toda vía basada en el
Agent SDK o en canales no documentados; prohibir los adaptadores de
terceros (siguen pilotables por configuración); adaptadores para otros
agentes; sandbox propio (change aparte); auto-actualización o descarga de
adaptadores (el daemon jamás descarga nada); dialectos alternativos de los
CLIs pilotados.

## Decisions

### D1 — Revertir la decisión registrada, por escrito

`niveles-integracion-conformidad` excluyó mantener adaptadores propios
cuando el ecosistema ofrecía adaptadores abiertos vivos y en Rust. Los
hechos que la vencieron, con fecha: (1) 2026-07-22, Zed archiva
`codex-acp` en Rust; las implementaciones canónicas quedan en TypeScript
bajo `agentclientprotocol` — consumirlas exige un runtime Node en la
distribución, contra el rumbo de un solo lenguaje de sistemas, y la única
vía Rust del ecosistema es embeber el runtime del proveedor como librería
(patrón que choca con §2, ver D6). (2) feb-2026, los términos de Anthropic
nombran al Agent SDK como no autorizado para OAuth de suscripción; el
adaptador canónico de Claude envuelve ese SDK y ningún fork escapa, porque
la zona gris vive en la superficie de auth, no en el mantenedor. (3) El
camino seguro que `docs/research/integracion-agentes.md` nombró desde el
principio — pilotar el binario oficial con la sesión ya iniciada — no lo
toma ningún adaptador del ecosistema; solo un adaptador propio lo toma.
(4) 2026-07-27, la directiva del mantenedor. Una decisión registrada se
revierte con otra decisión registrada: esta.

### D2 — Un crate, dos binarios, cero dependencias nuevas

`core/meltemi-adapters`: una librería de puente compartida (lado ACP,
supervisión del subproceso proveedor, framing NDJSON, apagado limpio) y
dos binarios, `meltemi-claude-acp` y `meltemi-codex-acp`. Un solo crate
porque los dos adaptadores comparten más de la mitad de su esqueleto y sus
tests; dos binarios porque el registro declara un punto de pilotaje por
entrada y la detección resuelve binarios, no flags. Los nombres llevan el
prefijo `meltemi-` y difieren de los binarios de terceros
(`claude-agent-acp`, `codex-acp`): jamás una colisión de PATH ni una
detección ambigua. Dependencias: tokio, serde/serde_json y el crate
oficial `agent-client-protocol` — las tres ya pineadas en el workspace.
**Ningún adaptador enlaza pila HTTP/TLS**: a diferencia de
`meltemi-engine` (que necesita rustls para hablar con el modelo), aquí
toda la red vive en el CLI oficial. La propiedad es verificable por
cargo-deny igual que la de meltemid, y el shim MCP de permisos (D5) se
implementa a mano sobre serde: JSON-RPC por stdio no justifica una
dependencia nueva (§10).

### D3 — Una capability `own-adapters`, sin nombres de terceros en specs

El borrador de propuesta nombraba dos capabilities (`adapter-claude-code`,
`adapter-codex`). Se corrige: la verdad viva de `.meltemi/specs/` no
contiene un solo nombre de producto de terceros (verificado por grep), y
el propio registro declara la regla — «third-party names live here, never
in specs». Los slugs de capability son directorios de la verdad viva. Una
sola capability `own-adapters` (espejo de `own-engine` en
`motor-propio-byok`) con requisitos por dialecto, en términos neutrales:
«dialecto de sesión headless de eventos JSON» y «dialecto de servidor
JSON-RPC». Los nombres concretos (claude, codex, stream-json, app-server,
flags) viven en el registro, en este design y en la guía — datos factuales
de interoperabilidad, no verdad normativa.

### D4 — Dialecto de sesión headless: el binario oficial `claude`, jamás el SDK, jamás `--bare`

`meltemi-claude-acp` lanza el `claude` oficial con la sesión que el
usuario ya inició: `-p --input-format stream-json --output-format
stream-json --include-partial-messages`. Qué compra: deltas de tokens y
transcripts de subagentes (casi-paridad de streaming con nivel 1),
`--resume`/`--fork-session` headless con ámbito de directorio de proyecto
y worktrees (calza con el modelo de Meltemi), `--mcp-config` para la
proyección de perfiles MCP existente. Qué se prohíbe: el Agent SDK (zona
gris nombrada por los términos del proveedor) y `--bare` (salta el OAuth y
exige `ANTHROPIC_API_KEY` — exactamente lo que §2 y el principio BYOK no
quieren como default).

**Riesgo pineado**: la documentación del proveedor anuncia que `--bare`
será el default de `-p` en una versión futura. Si ese flip llega y el
adaptador no lo maneja, el OAuth muere en silencio. Mitigación: el evento
`system/init` del cable trae un arreglo `capabilities` que existe para
detección de features; el adaptador detecta la superficie efectiva en el
handshake y, si el modo con sesión iniciada no está disponible (o el CLI
indica modo de clave de API), **rehúsa con diagnóstico y remedio** — nunca
degrada a inyectar una clave. La marca «docs del proveedor llaman a
`claude -p` "the Agent SDK via the CLI"» es marketing sobre el mismo
binario oficial; la frontera real es: binario oficial sí, librería SDK no.

**Enmienda (2026-07-28, tarea 5.2 — la corrida manual contra el CLI real)**:
dos afirmaciones de arriba no sobrevivieron al binario. Fuente: `claude`
2.1.167 en Windows 11 (26200), lanzado con exactamente los argumentos de
sesión del adaptador; procedimiento y salidas en `docs/conformidad-manual.md`.

1. **No existe arreglo `capabilities` en el evento inicial.** El evento real
   trae `apiKeySource`, `claude_code_version`, `permissionMode`, `model`,
   `tools`, `slash_commands`, `agents`, `skills` y `mcp_servers`; ningún
   `capabilities`. La detección de features no puede colgar de él, y la
   mitigación del flip de `--bare` descansa entonces sobre `apiKeySource`
   —que **sí** existe y vale `"none"` bajo la sesión iniciada, con lo que el
   nombre provisional de la tarea 1.3 queda anclado— más el rehúso ante
   cualquier otra fuente anunciada. El código ya trataba `capabilities` como
   información y nunca como requisito («capabilities are read, never
   demanded»), así que la corrección es de esta decisión, no del adaptador.
2. **El CLI no anuncia la sesión hasta recibir su primera entrada.** No es
   lentitud: lanzado sin entrada, no emite nada en 60 segundos; escrita una
   línea de entrada, el evento inicial llega en el acto. El handshake del
   adaptador lo espera *antes* de enviar nada, de modo que contra el CLI real
   toda sesión agota el plazo y rehúsa con `provider_handshake_failed` —
   nivel verificado 0. El fixture emitía el evento inicial antes del primer
   `await-input` y por eso ninguna prueba lo veía: un fixture solo prueba lo
   que se le pidió parecerse. La corrección es la tarea 5.3, y con ella el
   fixture pasa a emitirlo donde el CLI lo emite.

**Enmienda (2026-07-28, tarea 5.3 — la corrección y lo que exigió decidir)**:
el handshake de este dialecto **ocurre en el primer turno**, después de
escribir el mensaje del usuario y antes de mapear un solo evento de ese turno;
la guarda de superficie corre ahí, con la misma conducta de siempre — rehúso
diagnosticado ante cualquier fuente de credenciales que no sea la sesión
iniciada, jamás degradación silenciosa. Lo que abrir la sesión ya no necesita
es que el CLI hable, y eso obligó a una decisión que este design no tenía:

3. **La identidad de la sesión se dicta, no se aprende.** El adaptador acuña un
   UUID y lo entrega en `--session-id` (existe en 2.1.167 y el CLI lo respeta
   al pie de la letra: el archivo de sesión que deja lleva ese nombre). Antes
   la identidad salía del evento inicial, que es precisamente lo que ataba el
   handshake al `session/new`; dictarla desata los dos y conserva intacta la
   reanudación de la tarea 3.5, porque el id que el daemon recuerda sigue
   siendo el del CLI — ahora por construcción y no por haberlo escuchado. Un
   `--resume` nombra la sesión que continúa y nunca viaja junto a
   `--session-id`: dos identidades en un lanzamiento son una pregunta que el
   CLI no debería tener que responder.

4. **`capabilities` deja de leerse.** El punto 1 de la enmienda anterior lo
   declaró inexistente y el código lo seguía leyendo «por si acaso»; un campo
   que ningún CLI emite no describe nada, así que sale del tipo del cable, de
   la superficie y de la procedencia. La mitigación del flip de `--bare`
   descansa entera sobre `apiKeySource`, que sí existe.

5. **La procedencia se parte en dos actualizaciones, porque se conoce en dos
   momentos.** Qué binario se lanzó, en qué versión y con qué id — hechos del
   `session/new`, registrados ahí. Qué credencial, qué modelo y qué modo de
   permisos anunció el CLI — hechos del primer turno, registrados ahí. Fingir
   que lo segundo se sabía al lanzar es exactamente el error que esta tarea
   deshizo, y una sola actualización lo habría reintroducido en el log.

Y una lección de método que la change se lleva puesta: **el orden es parte de
un cable**. El guión del proveedor simulado ahora anuncia la sesión detrás del
primer `await-input` —con un marcador `once`, porque el CLI lo dice una vez por
proceso y no una por turno— y una prueba corre ese cable sin entrada y exige
silencio. Un fixture cortés en el punto exacto donde el binario no lo es no es
media prueba: es una prueba de un proveedor que no existe.

### D5 — Permisos del dialecto headless: prompt-tool + hooks, pérdidas declaradas

Dos canales documentados, en capas:

1. **`--permission-prompt-tool`** apunta a una herramienta de un servidor
   MCP mínimo por stdio que el CLI lanza según el `--mcp-config`
   proyectado. El servidor es el mismo binario `meltemi-claude-acp` en un
   modo shim, conectado a su proceso padre por un canal privado; cada
   petición del CLI se convierte en `session/request_permission` de la
   sesión ACP del adaptador, que meltemid proxya a la bandeja humana por
   el flujo vigente. **El daemon no gana transporte alguno** y el shim no
   abre socket alguno.

   **Enmienda (2026-07-27, tarea 3.3)**: este design decía «canal privado
   heredado al lanzar (pipe anónimo)». No es implementable, y el hecho que
   lo vence es de arquitectura, no de plataforma: **el shim no es hijo del
   adaptador**. Lo lanza el CLI pilotado —un runtime que Meltemi no
   controla— como uno de sus servidores MCP, de modo que el shim es un
   nieto y ningún descriptor del adaptador sobrevive ese salto; en Windows
   no sobrevive de ninguna forma. Lo reemplaza un **directorio de
   rendezvous por sesión**, creado por el adaptador dentro del directorio
   temporal del usuario (`0700` donde la plataforma tiene modos; en Windows
   el temporal por usuario ya es exclusivo del usuario por ACL heredada):
   el shim escribe su pregunta como archivo y espera la respuesta, ambos
   con escritura atómica por renombrado dentro del mismo directorio. La
   sustitución conserva **todas** las propiedades por las que el pipe
   estaba ahí —privado al usuario, sin puerto, sin listener, sin
   transporte nuevo en el daemon— y su costo es un sondeo de 20 ms, pagado
   donde nadie puede notarlo: detrás de un humano decidiendo. Que el
   directorio desaparezca es además la señal de vida del adaptador: un shim
   que no lo encuentra deniega en el acto en vez de esperar su plazo. Se
   descartó por escrito la alternativa de un socket local (named pipe en
   Windows, UDS en el resto): cumpliría igual, pero el estándar de la casa
   para un endpoint local es ACL explícita de usuario —lo que el daemon
   hace con `windows-sys`— y eso significaba código inseguro específico de
   plataforma en un crate cuyo design (D2) declara cero dependencias
   nuevas; el rendezvous alcanza la misma garantía con la biblioteca
   estándar.
2. **Hooks `PreToolUse`** (inyectados vía `--settings`) como compuerta
   dura: deniegan toda llamada no aprobada incluso si el CLI corre en un
   modo permisivo (`bypassPermissions`). El orden de evaluación del
   proveedor está documentado (hooks → deny → ask → mode → allow → prompt
   tool), y los hooks son el eslabón que ninguna configuración del CLI
   puede saltar.

Pérdidas, por escrito y visibles en sesión: el prompt-tool solo se
consulta cuando ninguna regla estática decide; `AskUserQuestion` y las
herramientas `requiresUserInteraction` se auto-deniegan en modo no
interactivo — el adaptador muestra la denegación con su motivo, no la
esconde ni la aprueba por su cuenta; el contrato del prompt-tool está
infradocumentado upstream (anthropics/claude-code#1175) y puede moverse —
la detección de features de D4 es el amortiguador, y los fixtures de cable
(D10) congelan el contrato observado por versión. El canal `canUseTool`
del SDK existe en el mismo cable pero no está documentado: §6 lo excluye.
En Windows no hay sandbox nativo del CLI (solo WSL2): la compuerta son los
hooks más el worktree aislado, y `sandbox-propio` sigue siendo la change
que cierra ese hueco.

### D6 — Dialecto de servidor JSON-RPC: `codex app-server`, jamás el embed de librería

`meltemi-codex-acp` lanza el `codex` oficial en modo `app-server`:
JSON-RPC 2.0 bidireccional con delimitación por líneas sobre stdio,
documentado por el proveedor como «the interface Codex uses to power rich
interfaces such as the Codex VS Code extension» — publicado explícitamente
para terceros. Las primitivas de conversación del servidor
(hilo/turno/ítem) se mapean a la sesión ACP; las aprobaciones que el
servidor solicita se relevan a `session/request_permission`.

Se rechaza el patrón de las dos implementaciones Rust existentes (la de
Zed, archivada; la comunitaria `cola-io`): ambas embeben `codex-core` y
crates hermanos como dependencias de librería, con lo que el adaptador
mismo hace red y lee el almacén de auth de Codex. Apache-2.0 de punta a
punta y aún así incompatible con §2 («solo binarios oficiales, con la
autenticación que cada agente gestiona»). Spawn del binario oficial, sin
excepción.

**Conformidad por versión**: `codex app-server generate-json-schema`
vuelca el esquema exacto del binario instalado. Los tipos del adaptador se
validan contra fixtures de ese esquema en CI (la disciplina que `proto/`
ya practica) y el handshake detecta el desfase adaptador↔CLI y rehúsa con
remedio — nunca se asume compatibilidad.

### D7 — La prueba que §6 exige

Del lado Meltemi el estándar es ACP, hablado con el crate oficial ya
pineado en el workspace. Del lado proveedor **no existe estándar abierto
que cubra el pilotaje programático de estos agentes**: ACP no lo cubre
(los CLIs no lo hablan — ese es el problema entero), MCP es herramientas,
LSP es inteligencia de código. Las superficies elegidas son la superficie
programática oficial y documentada de cada proveedor: stream-json
(documentación de headless/CLI del proveedor) y app-server (README y
generadores de esquema del proveedor). El adaptador es exactamente la
pieza que §6 bendice: traducción entre el estándar abierto y la superficie
oficial del dueño. Regla de gobernanza heredada de `motor-propio-byok`:
ninguna capacidad puede colgar de un canal no documentado del proveedor ni
de un canal privado adaptador↔daemon — todo lo que exceda ACP base sería
una extensión ACP abierta y documentada.

### D8 — Flip del registro y detección `bundled` genérica

Las filas `claude-code` y `codex-cli` cambian su capa adaptador:
`bin = "meltemi-claude-acp"` / `bin = "meltemi-codex-acp"`, con
`bundled = true` y sin `adapter-install` (la capa viaja en los
instaladores de Meltemi). Las capas `cli-bin`, `cli-candidate-paths` y
`cli-install` no se tocan: el CLI oficial se sigue detectando e instalando
aparte, y la semántica de dos capas de `flota-deteccion-guia` queda
intacta — solo cambia qué binario es la capa adaptador y de dónde sale.

La detección `bundled` — sondear el directorio del ejecutable del daemon
en ejecución — la describe la proposal de `motor-propio-byok`, pero esa
change no tiene design ni código: **se implementa aquí como mecanismo
genérico del registro** (cualquier capa con `bundled = true` lo usa; nada
cuelga de un id concreto) y el motor lo hereda cuando llegue.
Precedencia: PATH, luego `candidate-paths`, luego el directorio hermano
del daemon — coherente con la filosofía vigente de que la intención del
usuario pisa el default (override de entorno > `command` > id), y no es
degradación silenciosa: el catálogo reporta la ruta absoluta y la fuente
del hallazgo, el log de sesión registra el binario efectivo, y el skew
adaptador↔daemon lo detecta el handshake ACP. `proto/` gana campos
aditivos en la capa (procedencia empaquetada del hallazgo); paridad ×3
heredada por `fleet/list`.

Los adaptadores de terceros siguen pilotables por configuración (`custom`
o `command` literal), sin trato distinto. El registro deja de
recomendarlos; no los prohíbe.

### D9 — Estatus legal: gris sigue gris, con la nota reescrita con verdad

La nota de la entrada de Claude se reescribe para describir la
arquitectura nueva: la capa de pilotaje ya no envuelve el Agent SDK — es
el binario oficial con la sesión que el usuario inició, exactamente el
camino seguro que la nota vigente señala. Pero el estatus **se queda en
`grey`**: «tolerado» afirmaría que el proveedor tolera la orquestación de
`claude -p` por terceros, y no existe evidencia publicada de eso — el
research lo dice sin rodeos («Anthropic has published no OpenAI-style
blessing»). Subir el estatus con la evidencia actual sería maquillaje al
servicio propio, justo lo que el requisito «sin maquillaje» prohíbe. La
nota nueva dice las dos cosas: esta vía elimina la exposición del SDK y no
cuenta con postura publicada del proveedor. Si esa postura llega, la nota
se actualiza con fuente. Codex permanece `tolerated` con nota actualizada:
app-server fue publicado para terceros y la dependencia de supply chain
sobre un repo archivado desaparece. (Si el mantenedor prefiere
`tolerated`-con-nota para Claude — defendible según el research — es un
cambio de un literal y de esta decisión, no de la arquitectura.)

### D10 — Tests: fixtures de cable de proveedor, e2e real, conformidad manual

CI jamás corre agentes reales ni toca red; entonces el cable del proveedor
se simula, no el adaptador. Crate nuevo `core/mock-provider` (patrón
mock-agent, cero dependencias nuevas) con dos binarios:

- `mock-claude-wire`: emite stream-json guionado — `system/init` con
  `capabilities`, deltas parciales, tool calls que disparan el prompt-tool
  y los hooks, resultado final — y acepta entrada stream-json; guiones por
  archivo/variable de entorno, como mock-agent.
- `mock-codex-wire`: servidor JSON-RPC NDJSON guionado — handshake,
  conversación hilo/turno/ítem, petición de aprobación — más un volcado de
  esquema fixture para el test de conformidad.

Tres anillos: (1) unit/integración del puente y de cada mapeo contra los
mocks en memoria o por stdio; (2) e2e de workspace donde **meltemid pilota
el binario real del adaptador** y este pilota el mock wire — el mismo
anillo que hoy corre contra mock-agent; (3) conformidad contra CLIs reales:
manual, por opt-in, con resultado persistido con fecha y versión, y los
escenarios que solo un CLI real ejerce se marcan vía `sdd/verify-mark` con
nota — la disciplina que `niveles-integracion-conformidad` estableció y
`procedencia-de-release` ya practica.

**Enmienda (2026-07-28, tarea 5.2)**: el tercer anillo se materializa en
`core/meltemid/tests/conformance_real.rs`, `#[ignore]` y además silencioso
sin `MELTEMI_CONFORMANCE_REAL=1` — dos cerrojos, porque uno solo es el que
se olvida. Persiste en el almacén real del usuario, que es lo que hace que
`fleet/list` reporte el nivel verificado de una corrida de verdad. Regla que
la corrida trajo consigo: **un criterio que no se pudo ejercer no se
reporta**, ni aprobado ni fallido; `conformance::verified_level` se niega a
otorgar un nivel cuyos criterios declarados no estén todos presentes, de
modo que una corrida incompleta produce un resultado incompleto en lugar de
uno halagador. Y `verifiedLevel: 0` es un resultado, no un fallo de la
corrida: es lo que la primera corrida devolvió para el dialecto headless, y
es la razón por la que existe la tarea 5.3.

### D11 — Orden con `pulido-pre-anuncio`

`pulido-pre-anuncio` aterriza primero: refresca los `adapter-install` de
terceros a las distribuciones vigentes bajo `@agentclientprotocol`, porque
el usuario de hoy merece comandos que instalen proyectos vivos. Esta
change reemplaza después la capa adaptador de esas mismas filas y retira
esos `adapter-install`. No hay conflicto: el requisito «Vigencia de las
rutas de instalación de la instantánea» que pulido añade sigue aplicando a
todo comando que sobreviva (los `cli-install`), y la revisión de la
instantánea que esta change hace documenta su propia verificación con
fuente y fecha, como ese requisito exige. Si por calendario esta change
llegara antes, absorbe el refresco de datos trivialmente; el orden
declarado es pulido → esta.

**Verificación de la instantánea de esta change (tarea 4.2, 2026-07-28)**:
`pulido-pre-anuncio` se archivó el 2026-07-27 dejando la instantánea en
`version = "2026-07-27"`. Esta revisión sube a `2026-07-28` y **retira** los
dos `adapter-install` de terceros en vez de re-verificarlos: la capa de
pilotaje de ambas entradas de nivel 2 pasa a ser un binario que Meltemi
construye y empaqueta, de modo que ninguna ruta de terceros sobrevive a la
revisión — y una ruta que no se declara no puede envejecer. Los únicos
comandos de instalación que quedan son los dos `cli-install` de los CLIs
oficiales, **idénticos** a los de la instantánea del 2026-07-27, cuya
verificación contra la fuente de distribución (registro npm, 2026-07-27)
consta en `.meltemi/changes/archive/2026-07-27-pulido-pre-anuncio/.verify.jsonl`.
Ningún comando nuevo se introduce, así que no hay nada que citar de memoria.
La misma nota vive en la cabecera del propio archivo de registro, donde la
lee quien edite la instantánea la próxima vez.
