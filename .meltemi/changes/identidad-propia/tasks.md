# Tareas — identidad-propia

Vía completa. Un commit atómico por tarea, con referencia
`(identidad-propia N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny check`
y la suite del crate tocado. **Esta change se desarrolla en su propio taller
(`meltemi workspace identidad-propia`) y aterriza en `main` con `meltemi
land` al cerrar.** Ninguna tarea añade una dependencia ni toca `deny.toml`:
si alguna lo necesitara, la change se detiene y vuelve a design.

## 1. El contrato

- [ ] 1.1 `proto/meltemi-proto`: métodos `identity/status` e `identity/link`
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

## 2. El daemon

- [ ] 2.1 `core/meltemid/src/config.rs`: tabla `[identity]` (`mesh`,
  `login-server`, `command`) **de ámbito de usuario** —una tabla en el
  `.meltemi/config.toml` del proyecto se ignora con diagnóstico— con la
  disciplina de la casa: default ante valor inválido, diagnóstico con remedio,
  `login-server` como URL `https`, lint de secretos (design D5) — gates: suite
  del crate
- [ ] 2.2 `core/meltemid/src/identity.rs` + `core/mock-agent` bin
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
- [ ] 2.3 `identity/link`: el gesto compuesto (POSIX y PowerShell) con el
  servidor de login y el nombre del equipo, la pista del navegador, sin
  secretos y sin lanzar nada (design D3) — escenarios «El gesto de vínculo se
  imprime, no se ejecuta» y «El gesto no lleva secretos» — gates: suite del
  crate

## 3. Las superficies

- [ ] 3.1 CLI: verbos `identity` e `identity-link [--hostname] [--exec]` con
  su render humano y `--json`/`--yaml`; `--exec` corre el binario del usuario
  con stdio heredado; `registry.ts` GUI + `npm run gen:forms`; dos filas en
  `docs/paridad-nucleo.md`; `docs/referencia-cli.md` regenerada (gate
  bloqueante `tui/tests/parity.rs`) — escenarios «Ejecutar es explícito y
  visible», «identity imprime quién y qué equipos» e «identity-link imprime el
  gesto y solo ejecuta a pedido» — gates: suite de `tui`, parity, `check:forms`
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
