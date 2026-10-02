# Tareas — identidad-propia

Vía completa. Un commit atómico por tarea, con referencia
`(identidad-propia N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny check`
y la suite del crate tocado. **Esta change se desarrolla en su propio taller
(`meltemi workspace identidad-propia`) y aterriza en `main` con `meltemi
land` al cerrar.** Ninguna tarea añade una dependencia ni toca `deny.toml`:
si alguna lo necesitara, la change se detiene y vuelve a design.

## 1. El contrato

- [x] 1.1 `proto/meltemi-proto`: métodos `identity/status` e `identity/link`
  con sus params/results (`IdentityStatusResult` con `available`, `user`,
  `provider`, `this` y `machines[]` de nombre, presencia, sistema y última
  vez visto; `IdentityLinkParams` con `hostname` opcional;
  `IdentityLinkResult` con `command`, `posix`, `powershell`, `opens_browser` y
  `login_server`; `reason: not_configured | client_missing | not_linked` en el
  resultado de estado), `identity.schema.json`, conformidad por campo opcional,
  y el código `IDENTITY_UNAVAILABLE = 6000` en el enum cerrado y en el
  `x-catalog` de `error.schema.json` con `kind: identity_unavailable` —en el
  daemon el `kind` es un literal en el sitio del rehúso, no una tabla— (design
  D5, D9) — gates: `cargo test -p meltemi-proto`
  <!-- 2026-09-26: hecha **sin el cliente de malla**, porque el contrato no
  parsea su salida: lo que 2.2 necesita medir es la forma del JSON del cliente,
  no la del contrato. El taller trajo `main` otra vez antes de empezar, para
  partir de `estado-en-vivo`, `via-ssh-del-cliente` y `aviso-de-espera-opt-in`
  ya aterrizadas.
  Dos guardianes del repositorio saltaron y los dos tenían razón. El primero es
  una **lista a mano** de los códigos de error en el test de conformidad, con un
  comentario que dice que existe porque es lo que hizo visible la deriva del
  2005; añadir 6000 al enum y al catálogo no basta, hay que añadirlo también ahí.
  El segundo es el gate de paridad: estos dos métodos **sí** son invocables por
  el cliente, así que van a la paleta de la TUI, al registro de la GUI y a la
  matriz, al contrario que `session/state` de la tanda anterior.
  Un campo que la tarea no listaba y que el contrato necesita: `thisMachine` por
  máquina. Sin él, marcar el equipo actual sería adivinarlo comparando nombres, y
  el test rechaza una máquina que lo omita — porque omitirlo la marcaría como
  «no es esta» por descuido. -->

## 2. El daemon

- [x] 2.1 `core/meltemid/src/config.rs`: tabla `[identity]` (`mesh`,
  `login-server`, `command`) **de ámbito de usuario** —una tabla en el
  `.meltemi/config.toml` del proyecto se ignora con diagnóstico— con la
  disciplina de la casa: default ante valor inválido, diagnóstico con remedio,
  `login-server` como URL `https`, lint de secretos (design D5) — gates: suite
  del crate
  <!-- 2026-09-26: hecha **sin el cliente de malla**, como la 1.1: leer la tabla
  no es parsear la salida de nadie.
  El ámbito no cabía en `apply`, que es uniforme para usuario y proyecto, así que
  `[identity]` se lee **en `load`**, del archivo de usuario y solo de ese, y un
  archivo de proyecto que la declare deja un diagnóstico. Dicho y no descartado en
  silencio: alguien la escribió ahí a propósito y merece saber por qué no pasó
  nada. El test comprueba además que un proyecto **no pisa** una tabla de usuario,
  aunque la precedencia general vaya en ese sentido.
  `KNOWN_MESHES` es una lista de uno, y rechazar lo demás es el punto: aceptar un
  nombre desconocido sería ejecutar un binario desconocido y adivinar lo que
  imprima, que es justo el fallo que esta change existe para evitar. Cada rechazo
  deja identidad apagada y nombra su remedio, como manda la casa. -->
- [x] 2.2 `core/meltemid/src/identity.rs` + `core/mock-agent` bin
  `mock-mesh`: **primero** capturar una salida real redactada de `status
  --json` como fixture del simulado (la forma no es verificable desde el
  repositorio, design Context); luego `identity/status` invoca el cliente
  configurado con tiempo límite, parsea solo los campos del modelo tolerando
  los desconocidos, marca este equipo, y rehúsa con la razón que distingue sin
  tabla / sin binario / sin enlazar (design D2, D4, D5, D8) — escenarios «Quién
  soy, según la malla», «Sin cliente de malla, rehúso con remedio», «Cliente
  presente, equipo sin enlazar», «La identidad nunca es requisito», «Ningún
  token entra ni sale del daemon», «Las máquinas son los pares de la malla» y
  «Presencia no es control» — gates: suite del crate
  <!-- **BLOQUEADA 2026-09-26, DESBLOQUEADA 2026-10-01.** Historia, porque la
  razón del bloqueo sigue siendo buena: la tarea empieza por «capturar una salida
  real redactada de `status --json`», y en este equipo no hay ningún cliente de
  malla instalado (`tailscale`, `netbird`, `zerotier-cli` ni `headscale`, ni en el
  PATH ni en Program Files, ni en `winget list`). Inventar el fixture era el fallo
  que el design nombró para evitarlo, así que la change se detuvo en vez de
  fingir, y se hizo todo lo que no necesitaba el cliente (1.1, 2.1, 2.3).
  **Cómo se desbloqueó sin el cliente**: verificando la forma por la vía más
  fuerte, las **definiciones de tipo publicadas del propio cliente** fijadas en
  `v1.102.5` (`ipn/ipnstate/ipnstate.go` y `tailcfg/tailcfg.go`), que es de donde
  el CLI serializa su JSON. No es un atajo: una captura muestra solo los campos
  que **una** máquina pobló en **un** estado, mientras que el tipo enumera todos
  con su etiqueta y su regla de presencia. La enmienda está escrita en el design
  (Context, «cómo se verificó»).
  La hipótesis del design quedó confirmada campo por campo, y el tipo delató
  **cuatro trampas** que una captura en un equipo sano y enlazado habría
  escondido, cada una con su test: `Self` y `CurrentTailnet` son **nulos** sin
  conexión (justo el escenario «sin enlazar»); `LastSeen` no lleva `omitempty`, de
  modo que un par en línea trae el **tiempo cero** y un parser ingenuo diría
  «última vez visto: año 1»; las claves del mapa `User` son el id en **decimal
  como cadena** mientras el par lo trae como **número**, así que buscarlo sin
  convertir devuelve un `user` vacío contra el cliente de verdad; y `ShareeNode`
  marca un equipo que **otra persona** compartió hacia dentro — el propio upstream
  dice que su `status` los oculta por defecto, así que listarlo habría puesto la
  máquina de un colega en la lista del usuario.
  **El rigor de los tests se midió neuterando el código**: seis roturas
  deliberadas, cinco pusieron en rojo el test previsto y **una no**, que es lo
  que valía el ejercicio. Nada cubría «`BackendState` dice `Running` pero `Self`
  es nulo» —los dos señales pueden discrepar en un cliente que arranca— y sin la
  guarda el handler **panickeaba** en vez de rehusar. Se añadió el test que
  faltaba y ahora sí muerde, con el panic exacto que predijo.
  **Lo que la tarea sumó sin estar en su línea**: el enrutado de los dos métodos
  en `server.rs`. La 2.3 compuso el gesto pero ningún método estaba cableado, así
  que `identity/status` e `identity/link` eran inalcanzables; `identity/link`
  rehúsa con `IDENTITY_UNAVAILABLE` y el `kind` literal en el sitio del rehúso.
  Config de usuario sin raíz de proyecto, que es la frontera de D5 hecha código.
  Y una **corrección del contrato de la 1.1**: la descripción de `provider` decía
  «el valor de `[identity] mesh`» y el design D2 y el escenario «Quién soy, según
  la malla» piden «el servidor o nombre de la malla». Derivó al escribir el
  contrato antes del parser; se alineó la descripción con D2, no al revés.
  El simulado `mock-mesh` trae cinco escenarios (`linked`, `unlinked`, `alone`,
  `fail`, `garbage`, `slow`) y **rehúsa cualquier argv que no sea `status
  --json`**, que es lo que convierte «Ningún token entra ni sale del daemon» en
  una aserción: si el daemon alguna vez corriera `up` o `login`, el test se cae
  en vez de firmar a alguien en silencio.
  **Queda una comprobación de campo**, y pendiente es lo honesto: correr
  `identity` una vez contra un cliente instalado y enlazado. No bloquea nada
  —los nombres salen del tipo, no de una conjetura— y se anota en la 4.1.
  Gates: fmt, `clippy --workspace --all-targets -D warnings` en 0, 46 tests de
  `meltemi-proto` y la suite entera de `meltemid` verde (311 unitarios y todos
  los binarios de integración). -->
- [x] 2.3 `identity/link`: el gesto compuesto (POSIX y PowerShell) con el
  servidor de login y el nombre del equipo, la pista del navegador, sin
  secretos y sin lanzar nada (design D3) — escenarios «El gesto de vínculo se
  imprime, no se ejecuta» y «El gesto no lleva secretos» — gates: suite del
  crate
  <!-- 2026-09-26: hecha **sin el cliente de malla**, como 1.1 y 2.1: componer el
  gesto es leer la configuración, no la salida de nadie. Queda el módulo con su
  mitad escrita y su mitad ausente declarada en la cabecera, para que el próximo
  que lo abra sepa por qué falta `status`.
  Las dos formas **no son la misma cadena mostrada dos veces**, y ese es el punto
  de tener dos: un nombre con espacio pegado de POSIX a PowerShell se parte en dos
  argumentos. Cada una cita como su shell quiere, y una comilla dentro del nombre
  se escapa de forma distinta en cada una. El criterio de qué va sin comillas es
  estrecho a propósito: unas comillas de más cuestan dos caracteres; unas de menos
  cuestan un comando que hace otra cosa.
  El test de secretos assertA **por ausencia** —ni `--authkey`, ni `tskey-`, ni
  token ni contraseña—, que es la única forma de que siga siendo cierto cuando
  alguien añada una opción mañana. -->

## 3. Las superficies

- [x] 3.1 CLI: verbos `identity` e `identity-link [--hostname] [--exec]` con
  su render humano y `--json`/`--yaml`; `--exec` corre el binario del usuario
  con stdio heredado; `registry.ts` GUI + `npm run gen:forms`; dos filas en
  `docs/paridad-nucleo.md`; `docs/referencia-cli.md` regenerada (gate
  bloqueante `tui/tests/parity.rs`) — escenarios «Ejecutar es explícito y
  visible», «identity imprime quién y qué equipos» e «identity-link imprime el
  gesto y solo ejecuta a pedido» — gates: suite de `tui`, parity, `check:forms`
  <!-- 2026-10-01: la tarea pedía ejecutar «el binario del propio usuario» y ahí
  apareció un hueco del contrato: `IdentityLinkResult` traía `command`, `posix` y
  `powershell`, tres **cadenas**, y para correr algo hace falta un **argv**.
  Volver a partir `command` por espacios habría roto una ruta de cliente con
  espacio —exactamente el fallo que las dos formas citadas existen para evitar—, y
  pasarlo por un shell está fuera de la casa. Así que el contrato gana `args`, un
  vector con el programa primero, con su `minItems: 2` y dos rechazos nuevos en
  conformidad: sin vector, y con un vector demasiado corto para ser programa más
  verbo. El CLI corre **ese** vector, nunca una cadena reconstruida.
  El render humano imprime la presencia **en palabra** y no en glifo, porque esta
  superficie se canaliza hacia otros programas y `online` sobrevive a una tubería
  donde un símbolo no; los glifos son de la 3.2, que es una pantalla. Y la forma
  que se imprime es la **de la plataforma** (`powershell` en Windows, `posix`
  fuera), no la lectura plana, que es justo la que se rompe al pegarla.
  `--hostname` entra a `FLAGS`, a la lista de paso y a la lista pineada del test
  `no_flag_grants_permissions`, con su razón escrita: nombra, no concede. Un
  `--hostname` sin valor es un error de uso y no «pídele a la malla el nombre que
  quiera».
  Un rehúso del cliente del usuario bajo `--exec` sale como `identity_link_failed`
  reutilizando el código 10, el mismo que `tunnel_down` ya estiró a «el camino no
  está arriba»; un número nuevo habría sido cambiar la taxonomía de salida para un
  fallo que no es nuevo.
  `identity` rehusa argumentos apuntando a `identity-link` en vez de decir
  «sobran argumentos», que es lo que alguien necesita leer cuando escribió el
  verbo de al lado.
  Gates: fmt, clippy del workspace en 0, 23 binarios de test de `meltemi` y
  `meltemi-proto` verdes, parity verde (5), `check:forms` fresco tras regenerar,
  `npm run check` 0 errores, suite de `meltemi-desktop` verde y
  `docs/referencia-cli.md` regenerada. -->
- [ ] 3.2 TUI: `Effect::IdentityStatus` e `Effect::IdentityLink` despachadas
  desde el `match` de la paleta en `state.rs`, su `Command` en `conn.rs`, su
  estado en `live.rs`, su render en `render.rs` con la presencia en símbolo con
  gemelo ASCII (`glyphs.rs`) y sus cadenas ES/EN en `messages.rs` —no existe
  runner genérico de métodos (design D6)— escenarios «Identidad desde la
  paleta» y «Sin cliente de malla, el terminal dice el remedio» — gates: suite
  de `tui`
- [ ] 3.3 GUI: sección «Identidad» en `Settings.svelte` (quién, proveedor,
  este equipo, lista con presencia en símbolo y palabra, gesto copiable, la
  frase de quién es la identidad) y la fila de `Sidebar.svelte` por encima de
  Ajustes, solo con `[identity]` configurada, con marca propia (no el avatar
  de agentes) y fuera del reparto; ES/EN; la declaración de privacidad intacta
  (design D6, D7) — escenarios «Ajustes muestra quién soy y mis equipos», «Sin
  enlazar, la barra ofrece enlazar y no exige nada», «Sin configurar, el cromo
  no pide nada», «Plegada, la identidad sigue alcanzable» y «La promesa de
  privacidad sigue siendo cierta» — gates: suite de cableado, `npm run check`,
  lint de i18n

## 4. Cierre

- [ ] 4.1 `docs/acceso-remoto.md`: sección «Identidad: la de tu malla» y la
  nota de fase 3 acotada a lo que sigue siendo nota (certificados SSH del
  bastión, el selector que conecta, el aviso de espera); **reescribir
  `the_rendezvous_pattern_is_documented_with_its_boundary` en
  `tui/tests/docs.rs`** conservando sus tres marcadores `// Scenario:` —hoy
  pinea la semántica vieja y exige el literal «BYO-identity» dentro de las
  notas—; dos filas de presencia en `docs/ux/design-system.md`; e2e en
  `core/meltemid/tests/e2e_identidad.rs` contra el simulado (enlazado con tres
  equipos, cliente presente sin enlazar, sin tabla, binario que falla);
  `meltemi validate identidad-propia` limpio y `meltemi verify` con los
  veintidós escenarios enlazados (meta: cero marcas manuales, incluidos los
  tres del bloque MODIFIED); suite completa, clippy, fmt y `cargo deny`
  verdes; smoke con la app viva de la fila del pie y de Ajustes; entrada en
  `docs/plan-de-cambios.md`; corrección del tablero «Inicio de sesión» del
  lienzo y maqueta de la fila en `design-system/` (design D10); y la rama
  aterriza en `main` con `meltemi land identidad-propia confirm`
