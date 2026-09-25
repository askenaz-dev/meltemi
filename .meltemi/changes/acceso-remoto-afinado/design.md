# Design — acceso-remoto-afinado

## Context

Leído el código antes de diseñar, estos hechos fijan el diseño:

- **El arranque no mira dónde arranca.** `connect_or_start(endpoint)`
  (`core/meltemi-client/src/bootstrap.rs:33`) conecta y, si nadie responde,
  lanza `meltemid` y reintenta cinco segundos. El daemon lanzado hereda el
  entorno y escucha en `paths::endpoint()`: el `MELTEMI_ENDPOINT` del cliente si
  está fijado, el propio del usuario si no. Con la variable apuntando a un
  socket reenviado por un túnel caído, el cliente levanta un daemon local vacío
  con ese nombre, y a partir de ahí el usuario habla con él creyendo hablar con
  su PC.
- **Los cuatro que llaman al arranque** son la CLI (`run.rs`), la TUI
  (`shell/conn.rs`), la GUI (`desktop/src/bridge.rs`) y el `meltemi-devclient`;
  la TUI y la GUI ya muestran «inalcanzable» con el detalle del error y
  reintentan con espera creciente, así que un rehúso rápido les llega bien. El
  puente (`run::bridge`) ya no arranca nada, por decisión propia.
- **Quién depende del accidente.** `scripts/capture-desktop.ps1` fija un
  `MELTEMI_ENDPOINT` aislado y deja que la CLI arranque ahí el daemon;
  `tui/tests/unreachable.rs` fija un endpoint de prueba y un binario
  inexistente para probar el fallo de arranque. Los e2e que lanzan el binario
  real (`bridge`, `permission_verbs`, `session_verbs`) conectan con un daemon
  que ya escucha, y no arrancan nada.
- **Tres escenarios sin prueba.** «Primer cliente arranca el daemon» y
  «Segunda invocación reutiliza la instancia» (`daemon-lifecycle`) y «Arranque
  bajo demanda al no haber daemon» (`cli-contract`) no tienen marcador en
  ningún test: el camino que lanza el binario nunca se ejercitó, solo el
  rápido (`fast_path_connects_without_spawning`).
- **El túnel no sabe que murió.** `tunnel::compose` imprime `ssh -N -R …` sin
  latido ni `ExitOnForwardFailure` (`tui/src/tunnel.rs:77`), y `--exec` arma
  sus propios argumentos aparte (`run.rs`, `["-N", "-R", …]`), así que lo
  impreso y lo ejecutado pueden separarse; cuando `ssh` termina, `--exec`
  responde con éxito sea cual sea su estado.
- **Lo que ya existe para el teléfono.** La TUI exige 80×24
  (`tui/src/shell/render.rs:91`); la campana se pide con `MELTEMI_BELL`
  (ausente, vacía o `0` es apagada; cualquier otro valor la enciende,
  `tui/src/shell/bell.rs:20`); y desde `verbos-del-agent-boss` la sección
  «Aprobar desde el teléfono, hoy» de `docs/acceso-remoto.md` da la receta sin
  TUI.
- **Las esperas tienen valores y ninguna página.** `[permissions] wait`
  (`while-connected` por omisión, o segundos), `implement-wait` (30 s),
  `no-client-grace` (30 s); `[sessions] idle-timeout` (900 s; 0 no espera),
  `max-idle` (3), `lever-wait` (30 s) (`core/meltemid/src/config.rs:126-193`).
- **La doc de plataformas contradice al helper.** `docs/plataformas.md:50-52`
  dice «reenvía el socket con `ssh -L`» mientras el helper imprime `ssh -R`.
  Los dos funcionan, cada uno desde un extremo, y la página no dice cuál.
- **Qué puede medir esta máquina.** Es Windows y es la única: no hay un host
  macOS/Linux al que reenviar, y el cliente de Windows habla con named pipes, no
  con un socket reenviado. Instalar OpenSSH Server es cambiar un servicio del
  sistema: decisión del mantenedor.

## Goals / Non-Goals

**Goals**

- Que un cliente jamás levante un daemon impostor en un endpoint que no es
  suyo, y que diga por qué no conecta en vez de esperar cinco segundos.
- Que un túnel caído muera visible, y que lo impreso sea exactamente lo que
  `--exec` ejecuta.
- Que la documentación diga lo que hay hoy —PC Windows servido, teléfono,
  esperas— y que una página no contradiga a otra ni al helper.
- Que el acceso por túnel quede medido o declarado, nunca prometido.

**Non-Goals**

- Que un cliente lance su propio `ssh` (`via-ssh-del-cliente`), perfiles de
  máquinas (`maquinas-remotas`) o el aviso de espera
  (`aviso-de-espera-opt-in`).
- Instalar servicios en esta máquina para medir: el tramo de `sshd` queda
  declarado para el mantenedor.

## Decisions

### D1 — Un cliente arranca un daemon solo en su endpoint propio, o donde se le pide

`connect_or_start` intenta conectar como hoy. Si nadie responde, lanza el daemon
**solo** cuando el endpoint es el propio del usuario en esta máquina
(`paths::default_endpoint()`, que pasa a ser público) **o** cuando la
bandera `MELTEMI_START_HERE` lo pide. En cualquier otro caso rehúsa de
inmediato: un endpoint explícito que no responde puede ser un túnel caído, un
daemon de otra herramienta o un error de tecleo, y en ninguno de los tres un
daemon local nuevo es la respuesta. Fijar `MELTEMI_ENDPOINT` a su valor
propio sigue arrancando: es el mismo endpoint.

### D2 — La bandera tiene la forma de `MELTEMI_BELL`

`MELTEMI_START_HERE`: ausente, vacía o `0` es no; cualquier otro valor es sí,
como `MELTEMI_BELL` y `MELTEMI_ASCII`. Una variable y no un flag de la CLI
porque la necesitan los cuatro clientes y los scripts que los lanzan, y porque
quien aísla un entorno ya lo hace con variables (`MELTEMI_ENDPOINT`,
`MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR`, `MELTEMI_DAEMON_BIN`).

### D3 — El rehúso, inmediato y con los dos remedios

Sin esperar el presupuesto de arranque: `<endpoint> no acepta conexiones, y un
daemon solo se arranca en el endpoint propio de este usuario — si es un túnel,
revísalo; si el daemon debe correr aquí, arráncalo sin MELTEMI_ENDPOINT (por
ejemplo `meltemi status`) o fija MELTEMI_START_HERE=1 para arrancarlo en este
endpoint` (en inglés, como todo error del núcleo). La CLI lo reporta como
inalcanzable (`10`), la TUI y la GUI como su estado «inalcanzable» con ese
detalle, y siguen reintentando con su espera: cuando el túnel vuelve,
conectan.

### D4 — Dos MODIFIED, y los escenarios que nunca tuvieron prueba la ganan

La regla estrecha dos requisitos vigentes —`daemon-lifecycle` «Arranque bajo
demanda e instancia única» y `cli-contract` «Arranque del daemon bajo
demanda»—, que dicen que un cliente arranca el daemon cuando no hay ninguno. Se
escriben como MODIFIED. Sus escenarios sin marcador ganan prueba de verdad: el
binario real de `meltemid` arrancado por el bootstrap en un endpoint aislado
con la bandera, una segunda conexión que lo reutiliza —con el binario apuntado
a uno inexistente, para que un segundo arranque fallara si se intentara—, y la
CLI que arranca bajo demanda. Y el rehúso se prueba con el binario real
disponible: que no arranque nada no es porque no pudiera.

### D5 — Túnel con latido, compuesto en un solo lugar

`-o ServerAliveInterval=30 -o ServerAliveCountMax=3 -o ExitOnForwardFailure=yes`
en el comando, y sus tres líneas en el snippet de `~/.ssh/config`, con la nota
de qué hace cada una: sin respuesta del otro extremo en unos 90 s, `ssh`
termina en vez de seguir vivo como proceso y muerto como camino; y si el
reenvío no se puede montar (el socket remoto ocupado, `sshd` que no lo
permite), `ssh` termina al arrancar en vez de quedarse conectado sin reenviar.
`TunnelPlan` gana los argumentos (`ssh_args`) de los que salen el comando
impreso y lo que `--exec` ejecuta: lo impreso y lo ejecutado no pueden
separarse.

### D6 — Un túnel que muere se dice

Cuando el `ssh` de `--exec` termina: estado `0` → «el túnel terminó», código
`0`; cualquier otro estado, o una señal → error inalcanzable (`10`) con el
estado y lo que suele significar (la red, el reenvío, la autenticación), y el
recordatorio de que el otro extremo ya no alcanza este daemon. `10` porque el
propósito del verbo es mantener el daemon alcanzable desde lejos, y dejó de
estarlo. Un `Ctrl+C` del usuario termina los dos procesos a la vez y no deja
nada que reportar. La decisión es una función pura del estado, probada en toda
plataforma; el `spawn` queda fino.

### D7 — La documentación

- **«Windows como PC servido»**: OpenSSH Server (`Add-WindowsCapability`,
  `sshd` automático, la shell por omisión), y desde el otro extremo
  `ssh <pc> meltemi bridge`, los verbos y la TUI por `ssh -t`. Lo medido: el
  puente contra un named pipe con stdio en tuberías, y los verbos sin TTY
  (e2e en Windows). Lo no medido: el tramo por un `sshd` real, y qué pasa si el
  primero en arrancar el daemon es una sesión SSH en vez de la de escritorio.
- **«Desde el teléfono, hoy»** absorbe «Aprobar desde el teléfono, hoy»: los
  verbos sin TUI (lo que ya estaba), la TUI por `ssh -t` con su mínimo de 80×24,
  la campana (`MELTEMI_BELL=1`) que algunas apps de terminal convierten en
  notificación local, y lo que **no** hay: push sin conexión.
- **«Cuánto se espera al humano»**: cada ajuste de `[permissions]` y
  `[sessions]` con su valor por omisión y lo que pasa cuando vence, y que un
  `watch` o una GUI conectados cuentan como cliente.
- **`docs/plataformas.md`** deja de contradecir al helper: `-R` desde la máquina
  del daemon (lo que imprime `meltemi tunnel`), `-L` desde la del cliente, y el
  puente cuando el daemon está en Windows; con enlace a la página de acceso
  remoto.
- **La variable nueva** se documenta donde el aislamiento se documenta, y
  `scripts/capture-desktop.ps1` la fija.

### D8 — El smoke dice qué se midió y qué no

`docs/qa/2026-09-25-acceso-por-tunel-smoke.md`: medido en esta máquina, con
fecha y versiones, el rehúso del endpoint explícito con el binario release
(antes arrancaba un daemon; ahora no) y el arranque con la bandera; heredado
de los e2e, el puente contra un named pipe y los verbos sin TTY; **pendiente**,
con lo que falta para medirlo, GUI y TUI contra un daemon macOS/Linux reenviado
(un host macOS/Linux) y el tramo `sshd → meltemi bridge` en Windows (instalar
OpenSSH Server, decisión del mantenedor). La frase de `docs/acceso-remoto.md`
«con el túnel establecido, funciona todo» pasa a decir que es lo esperado y
remite al smoke para lo medido.

## Risks / Trade-offs

- **R1 — Alguien arrancaba a propósito en un endpoint propio con otro nombre.**
  Los flujos del repositorio que lo hacían quedan enumerados en el Context y
  ganan la bandera; uno de fuera la necesitará también, y el error se la
  nombra.
- **R2 — Comparar endpoints por texto.** Una ruta de socket escrita de otra
  forma (`~` sin expandir, barras distintas) no es «el propio» y no arranca: se
  rehúsa con el remedio, que es el lado seguro del error.
- **R3 — `ServerAliveCountMax=3` a 30 s** corta un túnel tras unos 90 s sin
  respuesta, también en una red lenta pero viva; el snippet lo deja a mano para
  quien quiera otro valor.

## Migration

Quien aísle un entorno y cuente con que la CLI, la TUI, la GUI o el devclient
arranquen el daemon en su endpoint explícito fija `MELTEMI_START_HERE=1`. El
endpoint propio arranca igual que antes. El contrato no cambia.
