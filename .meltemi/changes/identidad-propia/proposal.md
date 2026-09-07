# identidad-propia

> Vía completa (proposal → design → specs → tasks). Hay un **MODIFIED** sobre un
> requisito vivo de `remote-access` («El punto de encuentro en dos vías está
> documentado», que hoy manda que la identidad del usuario y el selector de
> máquinas queden como notas de fase 3), dos métodos nuevos del contrato con
> su deber de paridad §4, una tabla nueva de configuración y un binario
> simulado para los e2e. Ninguna dependencia nueva: la identidad la lee el
> binario oficial del cliente de malla del usuario, como el daemon lee `git`.

## Why

El mantenedor lo pidió así (2026-09-06): «Lo otro importante, inicio de
sesión. Sé que para esto hace falta un backend pero puede ser parte de la
propuesta». Y semanas antes había dicho lo que quiere ver desde el móvil:
«poder autenticarme en el cel con mi cuenta y ver las máquinas o instancias
que están conectadas», con su dominio (`askenaz.dev`), su Keycloak ya
desplegado y su casa como servidor.

La casa ya decidió lo esencial, y esta change no lo relitiga: **Meltemi no
gana cuentas**. `docs/acceso-remoto.md:153` lo escribe con todas las letras
—«Meltemi con login» choca con el rumbo y exigiría enmienda fundacional—, la
constitución §3 mantiene al daemon en un socket local sin puertos, §5 prohíbe
que compilar o testear exija cuenta o red, y `deny.toml` prohíbe con razón
escrita todo cliente HTTP y toda pila TLS en el workspace (`deny.toml:62-97`).
El webview tampoco alcanza ningún origen `https` (`desktop/tests/surface.rs:47`).
Un flujo OpenID Connect **dentro** de Meltemi rompería tres guardianes a la
vez; por eso la maqueta que lo dibujaba como código de dispositivo dentro de la
app no puede construirse tal cual, y esta propuesta la corrige.

Lo que sí cabe, y es exactamente lo que el mantenedor quiere: la identidad que
**su malla ya conoce**. En la variante de red privada documentada
(`docs/acceso-remoto.md:129-148`) la malla admite un IdP autohospedado, y el
mantenedor la tiene desplegada con Headscale y Keycloak; el cliente oficial de la malla (`tailscale`) hace el OIDC en el navegador
del usuario cuando se enlaza un equipo, y a partir de ahí **sabe quién es el
usuario y qué equipos suyos existen y están en línea** —`tailscale status
--json` lo imprime—. Nada de eso pasa por Meltemi. Lo que falta es que Meltemi
lo **lea** por el binario oficial (la misma regla con la que ejecuta `git` y
los agentes), lo muestre en sus tres superficies, y componga —sin ejecutar
jamás— el gesto que enlaza un equipo nuevo, como ya compone el gesto de login
de una suscripción (`fleet-catalog` «El login compuesto, jamás ejecutado»).

Y hay un requisito vivo que se lo prohíbe hoy: `remote-access` manda que
«identidad del usuario, selector de máquinas» queden anotadas como notas de
design para la change móvil, «no como capacidades presentes»
(`.meltemi/specs/remote-access/spec.md:115-118`). Esta change es la que las
convierte en capacidad, y por eso lleva un MODIFIED y no un ADDED disfrazado.

## What Changes

- **`identity/status`**: el daemon invoca el cliente oficial de la malla del
  usuario en modo de solo lectura y responde quién es el usuario (nombre de
  login y servidor de login), cómo se llama este equipo, y la lista de sus
  equipos con presencia (en línea o no), sistema y nombre. Sin cliente
  configurado o encontrado, rehúso honesto con remedio. La identidad **nunca
  es requisito**: todo lo local sigue funcionando sin ella, y el daemon no
  etiqueta ni distingue conexiones por identidad (la regla del túnel se
  conserva) (design D1, D2, D4).
- **`identity/link`**: compone el gesto que enlaza este equipo a la malla —el
  comando del cliente oficial con el servidor de login de la configuración y
  el nombre del equipo— en forma POSIX y PowerShell, como texto, con la pista
  de que abrirá el navegador del usuario para el OIDC de **su** IdP. El daemon
  jamás lo ejecuta y el gesto jamás lleva secretos. El CLI lo imprime y solo
  lo ejecuta con `--exec`, con la entrada y salida heredadas, como `tunnel`
  (design D3).
- **Configuración `[identity]`**: `mesh` (hoy solo `tailscale`; otro valor
  rehúsa con remedio), `login-server` (la URL de tu Headscale), `command`
  (ruta opcional al binario). Misma disciplina que el resto: un valor
  inválido conserva el default y deja diagnóstico (design D5).
- **Tres superficies antes que el móvil**: CLI (`identity`, `identity-link`),
  paleta de la TUI, y en la GUI una sección «Identidad» en Ajustes con el
  gesto copiable y una fila al pie de la barra —quién soy y cuántos equipos—
  o «Enlazar este equipo» cuando no hay identidad. La declaración de
  privacidad «sin cuentas» sigue siendo literalmente cierta y ahora dice de
  quién es la identidad que se muestra (design D6, D7).
- **Los e2e no tocan red ni cuenta**: un binario simulado `mock-mesh` imprime
  el JSON canónico del cliente de malla por escenario, como `mock-agent` hace
  con los agentes (design D8).
- **La documentación deja de anotar y empieza a describir**:
  `docs/acceso-remoto.md` gana «Identidad: la de tu malla», y el requisito
  vivo del punto de encuentro se restata para que la identidad y la lista de
  equipos sean capacidad presente en la variante de malla, mientras la
  identidad por certificados SSH de la variante de bastión y el aviso de
  espera siguen siendo notas.

## Capabilities

### Modified Capabilities

- `remote-access`: + tres requisitos ADDED (la identidad es la de la malla,
  el vínculo se compone y nunca se ejecuta, las máquinas se leen y no se
  registran) y **un MODIFIED** sobre «El punto de encuentro en dos vías está
  documentado» que conserva sus tres escenarios con su nombre; el tercero, «Lo
  de fase 3 está anotado y no prometido», cambia sus pasos para exigir que la
  identidad y la lista de equipos de la variante de malla se describan como
  capacidad presente y leída. Ninguna change activa tiene deltas sobre
  `remote-access`.
- `gui-shell`: + un requisito ADDED (la identidad de la malla se ve donde se
  ven los ajustes). «Superficie de Ajustes» no se modifica: su declaración
  «sin cuentas» sigue siendo verdad.
- `tui-shell`: + un requisito ADDED (identidad y máquinas desde la paleta).
- `cli-contract`: + un requisito ADDED (los verbos de identidad). «Mapeo
  comando↔método RPC» no se toca: lo modifica `lanzador-conversacional` sin
  archivar.

### New Capabilities

- Ninguna. La identidad es una faceta del acceso remoto, no una capability
  aparte: sin la malla no hay nada que identificar.

## Impact

- `proto/meltemi-proto` (dos métodos, params/results, `identity.schema.json`,
  conformidad, código de error `IDENTITY_UNAVAILABLE` en el dominio 6xxx),
  `core/meltemid` (`identity.rs`, `config.rs` con `[identity]`,
  `server.rs`), `core/mock-agent` (bin `mock-mesh`), `tui` (dos verbos, paleta,
  render), `desktop/ui` (`registry.ts` + `gen:forms`, `Settings.svelte`,
  `Sidebar.svelte`, `messages.ts`), `docs/paridad-nucleo.md` (dos filas),
  `docs/referencia-cli.md` (regenerada), `docs/acceso-remoto.md` y su test
  `tui/tests/docs.rs` —que hoy pinea la nota de fase 3 que esta change
  reescribe—, `docs/ux/design-system.md` (dos filas de vocabulario para la
  presencia).
- **Nace deber de paridad §4 y se paga aquí**: dos métodos → paleta TUI,
  registry GUI, filas de paridad; el gate `tui/tests/parity.rs` lo fuerza. Y
  se paga **antes que el móvil**, como exige la regla de subconjunto de
  `mobile-companion`.
- **Cero dependencias nuevas** y `deny.toml` intacto: ni HTTP, ni TLS, ni
  OIDC en el workspace. El cliente de malla es del usuario, se detecta como
  se detectan los agentes, y si no está, se dice.
- La constitución no se enmienda. El rumbo de producto podría ganar
  «BYO-identidad» junto a BYO-agent/BYOK/BYO-modelo; se propone como
  enmienda de una línea a firma del mantenedor, y la change no depende de
  ella.

## Fuera de alcance

- **Cualquier login dentro de Meltemi**: ni OIDC, ni código de dispositivo,
  ni tokens. Es del IdP del usuario y de su cliente de malla, y lo seguirá
  siendo.
- **Saber si un equipo corre `meltemid` antes de conectarse**: la presencia
  es de la malla; si hay daemon se sabe al conectar, como hoy con
  `MELTEMI_ENDPOINT`. Un «selector de máquinas» que conecte es concepto de la
  aplicación móvil (`companero-movil`) y de una change futura de la GUI de
  escritorio si el uso lo pide.
- **La variante de bastión SSH** (certificados SSH de vida corta emitidos por
  el IdP): sigue documentada como nota; esta change resuelve la identidad en
  la variante de malla, que es la que el mantenedor tiene desplegada.
- **Otros clientes de malla** (Netbird, ZeroTier): el campo `mesh` los
  rehúsa hoy con remedio; añadir uno es una change con su propia tabla de
  salida.
- **Control sin túnel**: la lista de equipos no abre nada; el transporte
  sigue siendo el SSH del usuario hacia el socket local, y `remote-access`
  sigue rechazando todo empuje o relevo fuera de la puerta.
