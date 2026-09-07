# Design — identidad-propia

## Context

Verificado el 2026-09-06:

- El daemon identifica a un cliente solo por `InitializeParams { protocol_version,
  client: PeerInfo { name, version } }` (`proto lib.rs:309`); no retiene nada
  de él (`server.rs:283`) y la única marca por conexión es un contador que se
  documenta como «no es una identidad» (`meltemi-client/src/rpc.rs:249`). El
  registro persistido de sesión no tiene campo de usuario
  (`session_index.rs:24`) y el autor de los commits es el git del usuario
  (`commit.rs:11`). `status` responde versión, uptime y sesiones — ni nombre
  de equipo, ni endpoint, ni usuario (`status.schema.json:39`).
- No existe noción de máquina ni de host en el daemon ni en el contrato:
  `NothingLeavesThisMachine` (`analytics.rs:41`) es una declaración de
  privacidad, no un modelo de equipo, y el resto de las apariciones de
  «machine» son prosa (`proto lib.rs:911`); el
  único `host` es el destino `user@host` del helper de túnel (`tunnel.rs:79`).
  Un daemon remoto se alcanza apuntando `MELTEMI_ENDPOINT` al socket
  reenviado (`tunnel.rs:93`); la GUI y la CLI resuelven **un** endpoint al
  arrancar (`desktop/src/lib.rs:214`, `tui/src/main.rs:19`).
- `deny.toml` prohíbe `reqwest` (único wrapper permitido: `tauri`), `hyper`,
  `rustls`, `native-tls`, `openssl`, `ureq`, `curl` e `isahc`, y escribe la
  razón: «No HTTP client and no TLS stack in the workspace» y «every byte that
  reaches a network leaves from the official binary» (`deny.toml:62-97`). El
  crate de escritorio no tiene cliente HTTP ni plugin `opener`/`shell`
  (`desktop/Cargo.toml:31`, `desktop/src/lib.rs:207`); su CSP no alcanza
  ningún origen `https` y el test lo pinea (`desktop/tests/surface.rs:47`).
- El precedente de «autenticación sin que Meltemi toque credenciales» es
  `subscription/link`: el daemon crea un directorio de contexto vacío y
  compone un `LoginGesture { var, value, hint, posix, powershell }` que **nunca
  ejecuta** (`proto lib.rs:993`; `fleet-catalog` «El login compuesto, jamás
  ejecutado»). El helper `tunnel` imprime el comando del usuario y solo lo
  ejecuta con `--exec` (`tui/src/cli.rs:127`).
- La configuración se parsea en `RawConfig` con cinco tablas (`agent`,
  `fleet`, `mcp`, `permissions`, `sessions`), cada una `#[serde(default)]`; un
  valor inválido conserva el default y deja un diagnóstico con remedio
  (`config.rs:186`, `:231`); existe un lint de secretos en claro reutilizable
  (`:603`).
- La spec viva `remote-access` «El punto de encuentro en dos vías está
  documentado» exige que identidad del usuario y selector de máquinas «SHALL
  quedar anotadas como notas de design» (`spec.md:115`); su escenario «La
  malla del usuario no es una dependencia de Meltemi» rechaza toda change que
  meta al workspace una dependencia de malla, bastión, plano de control o IdP
  (`:128`). `mobile-companion` exige que cualquier capacidad del daemon
  motivada por el móvil llegue con consumo TUI y GUI (`:34`) y que el daemon
  no distinga una conexión tunelizada de una local (`:39`). Ninguna change
  activa tiene deltas sobre `remote-access` ni `mobile-companion`.
- `docs/acceso-remoto.md` documenta la variante de malla (WireGuard, cliente
  Tailscale, Headscale, Keycloak, con licencias verificadas: `:129-148`) y
  deja la identidad como nota: certificados SSH de vida corta emitidos por
  el IdP del usuario (`:153-157`); «Meltemi con login» exigiría enmienda
  fundacional.
- Ajustes declara «Sin cuentas: Meltemi no tiene login ni servidor»
  (`messages.ts:541`) bajo el requisito vivo «Superficie de Ajustes», que
  manda esa declaración (`gui-shell/spec.md:313`); el onboarding de ambas
  superficies MUST NOT exigir cuenta (`gui-shell:226`, `tui-shell:287`).
- **No verificable desde el repositorio** (la única mención de Tailscale son
  las licencias, `docs/acceso-remoto.md:139`): según la estructura publicada
  del cliente, `tailscale status --json` trae `BackendState`, el perfil propio
  (`Self.HostName`, `Self.Online`, `Self.OS`, `Self.UserID`), el nombre de
  login en el mapa `User[<UserID>].LoginName`, la malla en
  `CurrentTailnet.Name` y los pares en el mapa `Peer` indexado por clave, con
  `HostName`, `Online`, `OS` y `LastSeen`. Esa forma se **captura de una salida
  real redactada antes de escribir el parser** (tarea 2.2): un campo mal
  nombrado daría un parser verde en CI y un rehúso contra el cliente de verdad.

## Goals / Non-Goals

**Goals**: que Meltemi diga quién eres y qué equipos tuyos hay, con la
identidad que tu malla ya conoce; que enlazar un equipo sea un gesto compuesto
y visible; que las tres superficies lo consuman antes que el móvil; y que
todo eso entre sin tocar la frontera de red del daemon ni la de la GUI.

**Non-Goals**: hacer login en Meltemi; guardar tokens; conectar a un equipo
desde la lista; certificados SSH; otros clientes de malla; cambiar `status`.

## Decisions

### D1 — La identidad es la de la malla, no la de Meltemi

Meltemi no tiene cuentas y no las tendrá: la constitución (§2, §3), el rumbo
(«ni un servicio en la nube»), `remote-access` y `docs/acceso-remoto.md`
apuntan en la misma dirección. Lo que sí tiene el usuario es una identidad
en su malla —Headscale la obtiene de su Keycloak por OIDC cuando enlaza cada
equipo— y una lista de equipos con presencia. Esa identidad es la buena por
tres razones: es **suya** (su dominio, su IdP), ya está **verificada** por
quien debe (el IdP, en el navegador del usuario), y ya está **en cada
equipo** (el cliente de malla la conoce localmente). Meltemi la lee; no la
crea.

### D2 — Se lee por el binario oficial, en modo de solo lectura

El daemon invoca `tailscale status --json` (o el `command` configurado) con
tiempo límite y parsea su salida; nunca invoca `up`, `login`, `logout` ni
nada que cambie estado o abra el navegador. Es la misma regla con la que
ejecuta `git` y los binarios de los agentes: el proceso es del usuario, la
red la hace su daemon de malla (`tailscaled`) en su propio socket, y ni un
byte sale del código de Meltemi. `deny.toml` queda intacto por construcción.

La salida se normaliza a un modelo pequeño: `user` (login name), `provider`
(el servidor de login configurado, o el nombre de la malla), `this` (nombre,
en línea, SO) y `machines[]` (nombre, en línea, SO, última vez visto). La
salida del cliente **contiene** claves públicas —el mapa de pares se indexa
por ellas—, así que la regla no es «no leerlas» sino que **nada fuera del
modelo sale del parser**: ni direcciones, ni claves, ni se conserva la salida
cruda. La lista es para reconocer equipos, no para conectar.

### D3 — El vínculo se compone y jamás se ejecuta

`identity/link` devuelve un gesto puro —`tailscale up --login-server
<login-server> --hostname <equipo>`— en forma POSIX y PowerShell, con un
campo enumerado `opens_browser: true` que cada superficie traduce por su
catálogo: los strings del contrato son en inglés y la localización vive en
las superficies (§11), así que el daemon no compone prosa en español. El daemon
no lo ejecuta; el CLI lo imprime y, solo con `--exec`, corre el binario del
usuario con la entrada y salida heredadas, para que el usuario vea el URL
que su cliente imprime y decida en su navegador. La GUI ofrece el gesto con
un botón de copiar: no tiene plugin para abrir URLs ni lo gana aquí. El gesto
no lleva secretos (una clave de preautorización sería un secreto; no se
compone).

### D4 — Las máquinas se leen, no se registran; presencia no es control

Meltemi no mantiene registro propio de equipos: la lista es lo que la malla
dice ahora. La presencia es de solo lectura y no abre nada —el control sigue
siendo únicamente el túnel SSH del usuario hacia el socket local— y saber si
en un par corre `meltemid` se aprende al conectar, no se supone. El daemon
sigue sin distinguir una conexión tunelizada de una local: la identidad
nunca viaja en `initialize` ni etiqueta conexiones.

### D5 — Configuración `[identity]`

```toml
[identity]
mesh = "tailscale"                          # único valor hoy; otro rehúsa con remedio
login-server = "https://headscale.askenaz.dev"
command = "tailscale"                       # opcional: ruta o nombre del binario
```

`[identity]` se lee de la configuración **de usuario**
(`<config_dir>/config.toml`): la identidad es de la persona y de la máquina,
no del proyecto —un `login-server` en el `.meltemi/config.toml` versionado se
comitearía—, así que una tabla en la configuración del proyecto se ignora con
diagnóstico y `identity/status` no depende del proyecto activo.

Sin la tabla, `identity/status` responde `available: false` con su razón y el
remedio (en inglés, como todo remedio del daemon); no es un error de arranque.
`login-server` se valida como URL `https`; el lint de secretos se aplica a
todos los valores por costumbre de la casa.

Tres razones distintas, y el contrato las distingue con
`reason: not_configured | client_missing | not_linked`, porque «no hay tabla»,
«no encuentro el binario» y «tu equipo no ha iniciado sesión en la malla» piden
tres remedios distintos.

### D6 — Dónde se ve en cada superficie

- **CLI**: `identity` (quién y qué equipos; `--json`/`--yaml` como todo
  listado) e `identity-link [--hostname <nombre>] [--exec]`.
- **TUI**: entradas de paleta `identity`/`identidad` e `identity-link`. No hay
  «runner genérico» que las atienda —el despacho de la paleta es un `match`
  literal de verbos a variantes concretas de `Effect`, y lo no reconocido cae
  en `_ => None` (`state.rs:436-524`)—, así que la TUI gana dos variantes
  `Effect::IdentityStatus` e `Effect::IdentityLink` con su `Command` en
  `conn.rs`, su estado en `live.rs`, su render en `render.rs` y los glifos de
  presencia en `glyphs.rs`. El estado vacío sin daemon no cambia.
- **GUI**: sección «Identidad» en Ajustes (quién, proveedor, este equipo, la
  lista con presencia, el gesto copiable) y una fila en la zona inferior de la
  barra lateral, **por encima de Ajustes**, que sigue siendo la última entrada
  («Arquitectura visual…» y el design system fijan «Ajustes abajo»). La fila
  no participa del reparto entre navegación y árbol, y plegada a riel conserva
  su etiqueta accesible y su foco mostrando solo su marca. La marca **no** es
  el avatar de iniciales de los agentes —esa paleta identifica agentes y un
  login con las mismas iniciales rendiría el mismo chip—: es una forma propia,
  y la presencia («en línea», «fuera de línea») entra al vocabulario de estados
  del design system con sus dos filas.

  Y solo aparece con `[identity]` configurada: sin tabla, el pie no cambia y
  Ajustes es la única puerta. Un llamado permanente a enlazar en el cromo de
  quien nunca quiso identidad sería pedir algo, y D7 dice que esto no pide
  nada.

### D7 — La promesa de privacidad sigue siendo verdad, y lo dice

«Sin cuentas: Meltemi no tiene login ni servidor» no se toca: es cierto. La
sección Identidad se presenta bajo su propia frase —«la identidad es la de
tu malla; Meltemi la lee, no la guarda»— para que nadie lea la fila del pie
como una cuenta de Meltemi. El onboarding no la menciona: nada de esto es
requisito.

### D8 — Los e2e con un cliente simulado

`core/mock-agent` gana un binario `mock-mesh` que imprime la salida canónica
de `status --json` según una variable de entorno de escenario (enlazado con
tres equipos; sin enlazar; binario que falla). Los e2e apuntan `[identity]
command` al simulado; no hay red, ni cuenta, ni cliente real en CI —lo que
exige la constitución §5 y el escenario «La malla del usuario no es una
dependencia de Meltemi», que esta change conserva con su nombre.

### D9 — Contrato

Métodos `identity/status` e `identity/link`, en un schema `identity.schema.json`
con `$defs` `statusParams`/`statusResult`/`linkParams`/`linkResult`; código de
error `IDENTITY_UNAVAILABLE = 6000` (dominio nuevo, 6xxx: identidad y malla)
añadido al enum cerrado y al mapa de `kind`. `status` no se toca: la máquina
propia se responde en `identity/status`, que es donde tiene sentido.

### D10 — Lo que la maqueta tiene que corregir

El tablero «Inicio de sesión» del lienzo de acabado —que vive fuera del árbol,
en el artefacto publicado de la sesión de diseño, no en `docs/ux/mockups/`—
dibuja un código de dispositivo dentro de la app. La forma construible es: el gesto de vínculo con copiar (y
el URL que el cliente imprima, como texto), el estado «esperando a que tu
cliente termine» leído por `identity/status`, y la pantalla de «este equipo
ya es tuyo» con la lista. Se corrige en el lienzo al abrir la implementación, y la maqueta de la fila
de identidad se añade a `design-system/` como manda su cabecera.

## Risks / Trade-offs

- **Acoplarse a la salida JSON de un cliente ajeno**: `tailscale status
  --json` es estable desde hace años, pero es suyo. El parser toma solo los
  campos nombrados en D2 y tolera campos desconocidos; si cambia, el rehúso
  es honesto («no pude leer la salida de tu cliente») y no un panic.
- **Un solo cliente soportado**: es el que el mantenedor tiene desplegado y
  el que la documentación ya nombra con licencias verificadas. `mesh` es un
  enum para que el segundo cliente sea una tabla nueva, no un rediseño.
- **`--exec` abre el navegador del usuario**: lo abre el cliente de malla, no
  Meltemi, y solo a pedido explícito con salida visible — el mismo trato que
  `tunnel --exec`.
- **La fila del pie puede leerse como cuenta**: D7 la etiqueta y Ajustes lo
  explica; el smoke lo comprueba con la app viva.

## Migration Plan

Aditivo: sin `[identity]` nada cambia —ni el pie de la barra, ni el
onboarding—, y Ajustes es la única puerta, con su sección explicando qué
sería. Con la tabla, la identidad aparece donde D6 dice. Ninguna preferencia
persistida cambia de forma; ningún archivo nuevo en el árbol del usuario.

## Open Questions

- ¿Debe el rumbo de producto decir «BYO-identidad»? Se propone como enmienda
  de una línea en `rumbo/product.md` («BYO-agent, BYOK, BYO-modelo,
  BYO-identidad») a firma del mantenedor; la change funciona sin ella.
- ¿Debe `identity/status` cachearse? No: es una invocación local barata y la
  presencia caduca; cada consulta lee la verdad del momento.
