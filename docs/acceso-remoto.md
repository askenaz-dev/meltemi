<!-- SPDX-License-Identifier: Apache-2.0 -->

# Acceso remoto

Meltemi se controla en remoto sin abrir un solo puerto de red. El daemon
`meltemid` escucha únicamente en un socket local; un túnel SSH del propio usuario
lleva ese socket al otro extremo, y el daemon no distingue una conexión así de
una local. Esta página declara la **frontera vigente** del acceso remoto: qué se
puede hacer, cómo, y qué deliberadamente no existe.

## La frontera: túnel vivo

Todas las capacidades del daemon operan en remoto **únicamente a través de un
túnel vivo** del usuario hacia el socket local. Con el túnel establecido, lo
esperado es que funcione todo —qué está medido y qué falta medir está en el
[smoke del acceso por túnel](qa/2026-09-25-acceso-por-tunel-smoke.md)—; sin
túnel, no funciona nada — no hay ni notificación ni control alguno sin una
conexión establecida.

Esto es una **postura de privacidad, no una carencia**. Habilitar control o
notificaciones sin túnel exigiría un relé en la nube o un puerto de red en el
daemon, y ambos están prohibidos por la constitución (§3, seguridad por defecto)
y por el rumbo del producto (Meltemi no es un servicio en la nube). Preferimos
que el acceso remoto sea indistinguible de estar sentado frente a la máquina —
con la misma superficie de ataque: ninguna.

## Los tres verbos remotos

Sobre un túnel vivo, cualquier cliente (TUI, GUI, y en fase 3 el compañero
móvil) ejerce los tres verbos del control remoto, todos por paridad de núcleo
(§4) y ninguno exclusivo de una sola superficie. Y por el SSH del propio
usuario, sin túnel, los ejerce la CLI (ver [Desde el teléfono, hoy](#desde-el-teléfono-hoy)):

- **Monitorear** — `session/list` y `session/log` consultan las sesiones activas
  e históricas y su transcripción JSONL; `session/watch` sigue una en vivo
  (`meltemi sessions`, `meltemi log`, `meltemi watch`).
- **Aprobar** — la cola de permisos sobrevive a las reconexiones: una petición
  de permiso pendiente sigue ahí cuando el cliente vuelve, y se resuelve en
  remoto igual que en local (`meltemi permissions`, `meltemi approve`,
  `meltemi deny`).
- **Dirigir** — `session/direct` envía una instrucción a una sesión existente:
  se encola como el siguiente turno de una sesión activa, o reanuda una sesión
  terminada y reanudable. Cada instrucción queda registrada en el log (encolada,
  luego despachada), de modo que la dirección remota es auditable.

## Desde el teléfono, hoy

### Aprobar y seguir sin la TUI

No hace falta túnel, ni puente, ni la TUI: basta el SSH del propio usuario hasta
la máquina del daemon, desde cualquier app de terminal del teléfono. Cada orden
corre **en esa máquina** y habla con su daemon por el socket local —también por
el named pipe de Windows, que la orden alcanza desde dentro—, y ninguna pregunta
nada, lee el teclado ni pide un tamaño de pantalla:

```
ssh <tu-pc> meltemi permissions
ssh <tu-pc> meltemi approve perm-3
ssh <tu-pc> meltemi watch <sesión>
```

`permissions` lista lo que espera una decisión —id, sesión, herramienta, título,
cuánto lleva esperando y cuándo vence— y, bajo cada petición, las opciones que
su agente ofreció. `approve <id>` elige la aprobación de una sola vez cuando el
agente ofreció exactamente una; `deny <id>` elige su rechazo, o cancela si no
ofreció ninguno; `--option <id>` nombra otra de esas opciones, y nombrarla es
consentirla. **Una petición por vez, nunca en bloque**: para conceder sin que te
pregunten está la regla de permisos, que queda escrita, auditada y con alcance.

La primera decisión gana. Si la bandeja de la GUI o de la TUI ya decidió la
petición, la tuya no se aplica y la orden termina con `12`; si la decides tú,
las otras superficies la retiran. La cola lleva la herramienta y el título que
el agente puso; la llamada completa —comando, rutas, contenido— está en el
registro de la sesión: `ssh <tu-pc> meltemi log <sesión>`.

`watch` imprime cada registro de la sesión a medida que ocurre, uno por línea, y
termina cuando la sesión termina —o con `10` si el daemon se va—. `Ctrl+C` deja
de seguirla sin cancelarla; cancelarla es `meltemi cancel <sesión>`, lo mismo
que la `x` de la TUI.

Un `watch` conectado cuenta como cliente: con la política de espera por omisión
(`while-connected`), una petición espera tu decisión en lugar de denegarse
—exactamente como con la GUI abierta—, y eso es lo que da tiempo a aprobar desde
el teléfono.

### La TUI por SSH

La interfaz completa también llega al teléfono: `ssh -t <tu-pc> meltemi` abre la
TUI en la app de terminal. `-t` pide la terminal que la TUI necesita, y la TUI
exige al menos **80×24**: por debajo muestra un aviso en vez de dibujarse a
medias, así que en un teléfono suele pedir la pantalla en horizontal o una
fuente más chica.

### La campana como aviso local

Con `MELTEMI_BELL=1`, la TUI toca la campana del terminal cuando algo te
necesita: una petición de permiso que espera, un gate que espera tu decisión,
una sesión que terminó. Algunas apps de terminal convierten la campana en una
notificación del teléfono —si lo hacen y cómo depende de cada app—, y ese aviso
no pasa por ningún servidor: es la misma conexión SSH que ya tenías abierta.
En un PC macOS/Linux, `ssh -t <tu-pc> MELTEMI_BELL=1 meltemi`; en uno Windows,
fija `MELTEMI_BELL=1` entre las variables de entorno de tu usuario.

### Lo que no hay

No hay push sin conexión: si el teléfono no tiene una sesión SSH abierta con el
PC, nada le avisa. Es la frontera de esta página, no un olvido (ver [Sin push
sin túnel — y por qué](#sin-push-sin-túnel--y-por-qué)). El aviso de espera
opt-in —emitido por un proceso tuyo, nunca por el daemon— está especificado y
todavía no existe.

## Cuánto se espera al humano

Cada espera tiene su ajuste, en el `config.toml` global o en el
`.meltemi/config.toml` del proyecto (el del proyecto manda):

| Ajuste | Por omisión | Qué hace |
|---|---|---|
| `[permissions] wait` | `"while-connected"` | Cuánto espera una petición de los flujos interactivos (`session`, `propose`, el ciclo SDD, `direct`): mientras haya un cliente conectado, o los segundos que digas (`wait = 300`). Al vencer se deniega y queda registrada como vencida. |
| `[permissions] implement-wait` | `30` | Lo mismo para los turnos autónomos de `implement`: acotado, para que un pipeline desatendido no se quede parado en silencio. |
| `[permissions] no-client-grace` | `30` | Segundos que una petición sobrevive sin ningún cliente conectado antes de la denegación constitucional (§3): absorbe un corte breve del túnel. |
| `[sessions] idle-timeout` | `900` | Segundos que una sesión espera su siguiente instrucción con el agente vivo; `0` no espera. |
| `[sessions] max-idle` | `3` | Cuántas sesiones pueden esperar a la vez; al llegar, se cierra la que más lleva esperando. |
| `[sessions] lever-wait` | `30` | Segundos que el agente tiene para confirmar un modelo o un esfuerzo pedidos por lo que anunció, antes de que la sesión se rehúse sin enviar el prompt. |

Cualquier cliente conectado cuenta: la GUI abierta, la TUI, un `meltemi watch`
por SSH. Con `while-connected`, mientras alguno siga conectado, una petición
espera tu decisión.

## El helper de túnel

`meltemi tunnel [user@host]` compone —usando el `ssh` que el usuario ya tiene— el
comando exacto de reenvío inverso (`ssh -R`) que lleva el endpoint local de este
daemon al host remoto, más el snippet equivalente de `~/.ssh/config` y el valor
de `MELTEMI_ENDPOINT` que el extremo remoto debe fijar. Con `--exec` lanza ese
`ssh` como proceso visible; jamás deja túneles en segundo plano sin pedirlo.

El comando lleva latido (`ServerAliveInterval=30`, `ServerAliveCountMax=3`) y
`ExitOnForwardFailure=yes`: sin respuesta del otro extremo en unos 90 segundos,
o si el reenvío no se puede montar, `ssh` termina en vez de seguir vivo como
proceso y muerto como camino. Con `--exec`, cuando termina, `meltemi` dice con
qué estado y qué suele significar, y un túnel que murió nunca se presenta como
un éxito.

Del otro lado, un cliente con `MELTEMI_ENDPOINT` apuntando al socket reenviado
**nunca arranca un daemon ahí**: si el túnel está caído, lo dice y nombra el
túnel, en lugar de levantar un daemon local vacío con ese nombre que responda
en nombre de tu PC.

El helper es un **formateador**: no abre sockets propios, no introduce transporte
de red en el daemon y nunca genera, lee ni almacena material de claves. Se
ejecuta en la máquina donde vive el daemon.

Un daemon en Windows escucha en un named pipe, que OpenSSH estándar no sabe
reenviar; el helper lo **rehúsa con honestidad** y su remedio, en lugar de
imprimir un comando que no puede funcionar. Una máquina Windows sí puede ser el
**cliente** que ejecuta el comando contra un daemon remoto Unix. Al fijar
`MELTEMI_ENDPOINT` desde git-bash, recuerda `MSYS_NO_PATHCONV=1` para que la ruta
del socket no se transforme (ver [plataformas.md](plataformas.md)).

## El puente: el último metro

`meltemi bridge` conecta el endpoint local del daemon de **esta** máquina y lo
bombea contra su propia entrada y salida estándar. No abre sockets, no reenvía
nada y no toca material de claves: copia bytes en dos direcciones hasta que un
extremo cierra.

Con él, esto es un canal JSON-RPC completo:

```
ssh <tu-pc> meltemi bridge
```

Es el **último metro** de todo acceso remoto, y el que faltaba en Windows. Un
named pipe vive en el espacio de nombres del kernel, no en el sistema de
archivos, así que **ninguna forma de reenvío lo alcanza desde fuera**: toda ruta
termina en algo que corre dentro de esa máquina. El puente es ese algo. Por eso
`meltemi tunnel` sigue rehusando en Windows —reenviar un pipe es imposible, no
incómodo— y su remedio nombra este comando.

Si no hay daemon escuchando, el puente rehúsa de inmediato con su remedio en
lugar de esperar: al otro lado de un `ssh` no hay nadie mirando un indicador de
progreso.

## La GUI y la TUI contra tu PC

El puente de arriba es un canal completo, pero hasta ahora solo lo hablaba un
`ssh` abierto a mano. Ahora lo habla **cualquier cliente de Meltemi**: un
endpoint que empieza por `exec:` nombra un comando en vez de un socket, y el
cliente lo lanza y habla el protocolo por su entrada y salida.

```
MELTEMI_ENDPOINT="exec:ssh mi-pc meltemi bridge" meltemi status
```

Para la GUI, que se abre desde un icono y nunca vio una variable exportada en un
shell, lo mismo va en tu configuración **de usuario**:

```toml
# <config>/meltemi/config.toml
[client]
endpoint = "exec:ssh mi-pc meltemi bridge"
```

El cromo de las dos superficies dice **vía `ssh mi-pc`** mientras esté hablando
por ahí. No es decoración: si la flota en pantalla es de otra máquina, revertir
un archivo en la equivocada no debe parecer revertirlo en la propia.

### El `~/.ssh/config` mínimo

Nada de esto configura `ssh` por ti, y eso es deliberado. Un alias y un latido
bastan:

```
Host mi-pc
    HostName 203.0.113.10
    User guillermo
    ServerAliveInterval 30
    ServerAliveCountMax 3
```

`IdentityFile` no es obligatorio: si tu clave está en el agente, `ssh` la
encuentra.

### Lo que Meltemi no hace, y no va a hacer

- **No añade opciones a tu comando.** Ni `BatchMode`, ni `-o` de ningún tipo. El
  comando es tuyo, entero.
- **No gestiona claves, passphrases ni hosts conocidos.** Los decide tu `ssh`,
  con tu agente y tu `known_hosts`. Meltemi no lee ni transporta nada de eso.
- **No te da un terminal para escribir una passphrase.** La entrada y la salida
  del comando **son** el canal del protocolo; no pueden ser también un diálogo.

De lo último se sigue lo único incómodo: **la primera conexión a un host nuevo
va a fallar**, porque nadie puede aceptar la huella. El rehúse lo dice y trae lo
que `ssh` escribió en su salida de error, que es donde explica de verdad lo que
pasó. El remedio es una vez y en un terminal:

```
ssh mi-pc true        # acepta la huella
ssh-add ~/.ssh/id_ed25519   # carga la clave en el agente
```

Después, el cliente entra sin preguntar nada.

### Dos garantías que conviene saber

- **Un endpoint `exec:` nunca arranca un daemon local.** Ni con
  `MELTEMI_START_HERE=1`. Si el comando falla, arrancar uno aquí serviría esta
  máquina en el nombre de la otra, y respondería en su lugar.
- **El endpoint no se lee del repositorio.** Solo de tu entorno o de tu
  configuración de usuario. Un `.meltemi/config.toml` que pudiera fijarlo sería
  un repositorio que ejecuta un comando en tu máquina por el hecho de abrirlo, y
  clonar no debería ser una decisión de seguridad.

Sin tuberías ni redirecciones en la cadena, porque no hay shell en medio: si las
necesitas, nombra un script. Un `.cmd` en Windows funciona, y el proceso que
lance muere con el cliente igual que el resto.

## Windows como PC servido

Un PC con Windows se sirve con el servidor OpenSSH que Windows trae como
característica opcional. En una PowerShell **de administrador** del PC:

```
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
Start-Service sshd
Set-Service -Name sshd -StartupType Automatic
```

Opcional, que las sesiones SSH abran PowerShell en vez de `cmd`:

```
New-ItemProperty -Path "HKLM:\SOFTWARE\OpenSSH" -Name DefaultShell -Value "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe" -PropertyType String -Force
```

Desde el otro extremo, con `meltemi` en el `PATH` del PC:

- `ssh <tu-pc> meltemi bridge` — un canal JSON-RPC completo hacia el daemon: el
  último metro de arriba, y lo único que alcanza un named pipe desde fuera.
- `ssh <tu-pc> meltemi permissions`, `approve`, `watch`… — los verbos sin TUI
  (ver [Desde el teléfono, hoy](#desde-el-teléfono-hoy)).
- `ssh -t <tu-pc> meltemi` — la TUI.

Conviene que el daemon lo arranque la sesión de escritorio del PC —abrir la
aplicación, o `meltemi status` en una terminal local— antes de irse.

**Medido**, en Windows y por las pruebas e2e del repositorio: el puente contra
un named pipe con su entrada y salida en tuberías, y los verbos sin terminal
contra el daemon. **No medido**: el tramo a través de un `sshd` real de
Windows, y qué pasa si lo primero que arranca el daemon es una sesión SSH en vez
de la de escritorio. El estado de cada tramo está en el [smoke del acceso por
túnel](qa/2026-09-25-acceso-por-tunel-smoke.md).

## El punto de encuentro: acceso en dos vías

El acceso remoto tiene que funcionar con **cualquiera de los dos extremos fuera
de su red habitual** — tú con el portátil en otra ciudad, o tu PC de escritorio
detrás de un router que no controlas. Eso se resuelve con una sola regla:

> Nadie recibe conexiones. Ambos extremos marcan **hacia afuera**, a un punto de
> encuentro que el usuario opera.

```
      PC (donde esté)            punto de encuentro           cliente (donde esté)
    ┌────────────────┐          ┌──────────────────┐         ┌──────────────────┐
    │ meltemid       │ saliente │  sshd del        │saliente │  meltemi / TUI /  │
    │ + sshd         │═════════▶│  usuario         │◀════════│  compañero móvil  │
    │ túnel inverso  │  ssh -R  │  (su dominio,    │  ssh -J │                   │
    │ permanente     │          │   su máquina)    │         │                   │
    └────────────────┘          └──────────────────┘         └──────────────────┘
                     el cliente salta por el punto de encuentro
                     y aterriza en el sshd del PC, esté donde esté:
                     ssh -J <encuentro> <pc> meltemi bridge
```

Los cuatro casos usan el **mismo camino**:

| PC | Cliente | Camino |
| --- | --- | --- |
| En casa | Fuera | cliente → encuentro → túnel inverso → PC |
| Fuera | En casa | idéntico: el PC ya marcó desde donde esté |
| Fuera | Fuera | idéntico: ambos convergen en el punto de encuentro |
| En casa | En casa | idéntico, o directo por la red local |

Detalle que simplifica todo: el túnel inverso reenvía **el puerto TCP del sshd
del PC**, y reenvío TCP sí lo hace el OpenSSH de Windows. El named pipe no viaja
por ningún túnel: solo lo toca el puente, en el último salto.

**Precauciones del punto de encuentro**, porque es una máquina expuesta: cuenta
dedicada para el túnel del PC, solo claves (sin contraseña), `PermitListen`
acotado al puerto que se publica y sin shell interactiva. Que comprometerla no
regale más que un puerto en loopback que aún exige la clave del PC para servir
de algo.

**El costo, dicho y no escondido**: el punto de encuentro apagado significa sin
acceso remoto. Es la disponibilidad que se acepta a cambio de no depender de la
nube de nadie. El trabajo local nunca se ve afectado.

## Red privada del usuario (BYO-network)

Una malla WireGuard con plano de control autohospedado es la otra forma del
punto de encuentro, y resuelve además el caso ideal: conexión **directa** entre
PC y cliente cuando el NAT lo permite, y relé por la máquina del usuario cuando
no — la red elige por conexión, sin que nadie configure dos topologías.

Piezas y licencias, verificadas: el protocolo WireGuard es abierto; el módulo
del kernel es GPLv2 y ya viene en Linux; `wireguard-go`, el cliente de Windows y
su driver son MIT; el cliente de Tailscale y Headscale (plano de control
autohospedado) son BSD-3; Keycloak, si se quiere un IdP, es Apache-2.0.

**La frontera, que es lo que importa aquí**: nada de esa infraestructura es de
Meltemi. No se empaqueta, no es dependencia del workspace, y compilar o testear
Meltemi no exige cuenta ni red. Es infraestructura **del usuario**, igual que su
`ssh` — la misma postura BYO que rige los agentes y las claves. El transporte
final hacia el daemon sigue siendo el SSH del usuario hacia el socket local; la
malla es solo el enrutamiento por el que ese SSH viaja.

## Identidad: la de tu malla

Meltemi **no tiene cuentas y no las va a tener**. Lo que sí tienes es una
identidad en tu malla —tu dominio, tu IdP, verificada por quien debe, y ya
presente en cada equipo— y Meltemi la **lee**. No la crea, no la guarda y no la
usa para nada más: ninguna conexión se etiqueta por identidad y ningún uso local
la necesita.

Se enciende con una tabla en tu configuración **de usuario**:

```toml
# <config>/meltemi/config.toml   (nunca el .meltemi/ de un proyecto)
[identity]
mesh = "tailscale"                           # único valor hoy
login-server = "https://headscale.mi-casa"   # si corres tu propio servidor
command = "tailscale"                        # opcional: ruta o nombre del binario
```

De usuario y solo de usuario: la identidad es de la persona y de la máquina, y un
`login-server` en un archivo versionado se comitearía. Una tabla `[identity]` en
el `.meltemi/config.toml` de un proyecto **se ignora con diagnóstico**, para que
quien la escribió ahí sepa por qué no pasó nada.

Con eso, `meltemi identity` responde quién eres, de quién es la malla, cómo se
llama este equipo, y tus equipos con su presencia:

```
guillmar@mi-casa on https://headscale.mi-casa
this machine: mi-pc

  mi-pc        online   windows  (this machine)
  el-portatil  online   macOS
  la-torre     offline  linux    last seen 2026-09-30T18:21:05Z
```

**Cómo se lee, exactamente.** El daemon invoca una sola vez el binario oficial
que configuraste, en modo de solo lectura (`status --json`), con un **tiempo
límite**; nunca `up`, `login`, `logout` ni nada que cambie estado. Es la misma
regla con la que ejecuta `git`: el proceso es tuyo, la red la hace tu propio
demonio de malla en su socket, y del código de Meltemi no sale un byte. De la
salida del cliente **nada fuera del modelo sobrevive**: ni direcciones, ni claves
públicas —el mapa de pares viene indexado por ellas—, ni la salida cruda.

**Presencia no es control.** Saber que un equipo está en línea no abre nada: el
camino hacia un daemon remoto sigue siendo tu túnel SSH, y si en un equipo corre
`meltemid` se sabe al conectar, nunca se supone. La lista es para **reconocer**
equipos, no para conectarse a ellos.

**Cuando no se puede, se dice por qué.** Tres rehúsos, porque piden tres remedios
distintos: no hay tabla (`not_configured`), no se encuentra el binario donde lo
nombraste o no se pudo leer (`client_missing`), o el cliente está ahí y este
equipo no ha iniciado sesión (`not_linked`). Ninguno degrada nada más: Meltemi
sigue funcionando igual con la identidad apagada, que es el estado por defecto.

**Enlazar este equipo** es un gesto que Meltemi **compone y jamás ejecuta**:

```
meltemi identity-link                 # lo imprime, en la forma de tu shell
meltemi identity-link --exec          # lo corre: es tu binario, con tu terminal
```

Un daemon que ejecutara un login sería un daemon con la sesión de alguien en las
manos. Así que el daemon devuelve texto; el CLI lo imprime y, solo con `--exec`,
corre **tu** binario con la entrada y la salida heredadas, para que el URL que
imprima tu cliente llegue a ti sin filtro y decidas en tu navegador. El gesto no
lleva secretos: una clave de preautorización sería un secreto, y no se compone
ninguna. La GUI lo ofrece con un botón de copiar y nada más — no tiene con qué
abrir un navegador ni lo gana aquí.

En la TUI, `:identity` e `:identity-link` desde la paleta (también `:identidad`);
la respuesta se dibuja bajo el catálogo. En la GUI, la sección «Identidad» de
Ajustes, y una fila en el pie de la barra **solo si configuraste la tabla**: un
llamado permanente a enlazar en el cromo de quien nunca quiso identidad sería
pedir algo, y esto no pide nada.

> **Verificación de campo pendiente.** La forma del JSON del cliente se verificó
> contra sus **definiciones de tipo publicadas**, fijadas en `v1.102.5`
> (`ipn/ipnstate/ipnstate.go` y `tailcfg/tailcfg.go`), que es de donde el CLI
> serializa su salida — más fuerte que una captura, que solo muestra los campos
> que una máquina pobló en un estado. Queda correr `meltemi identity` una vez
> contra un cliente instalado y enlazado; no cambia el contrato, y si algo
> apareciera se anota aquí con la versión contra la que se leyó.

## Tus máquinas, con nombre

Varios Meltemi en varios equipos **no toca el daemon**: cada `meltemid` sigue
siendo de su máquina y no sabe que existen otras. El multi-daemon es un concepto
del cliente — una lista de endpoints con nombre, que vive en tu configuración de
usuario:

```toml
# <config>/meltemi/config.toml   (nunca el .meltemi/ de un proyecto)
[[client.machines]]
name = "esta"
endpoint = "local"                           # el default de esta plataforma

[[client.machines]]
name = "la-torre"
endpoint = "exec:ssh la-torre meltemi bridge"
```

De usuario y solo de usuario, por la misma razón que el endpoint suelto y sin
ninguna nuance: un endpoint puede nombrar un comando, así que un repositorio
capaz de fijarlo ejecutaría algo en tu máquina por el hecho de abrirlo.

**Sin material de claves.** Ni ruta de clave, ni passphrase, ni puerto, ni
usuario: `la-torre` es un alias que resuelve tu `~/.ssh/config` con tu agente de
claves, que es donde `ssh` ya los lee. Un campo aquí para una ruta de clave sería
el primer paso hacia leerla, así que escribir uno **rehúsa la máquina entera** y
te dice dónde vive ese dato — ruidoso a propósito: ignorarlo te dejaría creyendo
que tu clave está en uso sin ver por qué no pasa nada.

Dónde se ve y qué hace cada superficie:

```
meltemi machines              # lista, con la que estás en uso marcada
:machines  /  :maquinas       # lo mismo en la paleta de la TUI
:machine la-torre             # cambiar de máquina, sin reiniciar
```

En la GUI, la sección «Máquinas» de Ajustes: la lista, «Probar» y «Usar esta».
El CLI **lista y no cambia**, y lo dice: cada corrida resuelve su endpoint y
termina, así que no hay a quién cambiarle la máquina.

**Cambiar de máquina es reconectar**, no reiniciar: el cliente corta y marca el
endpoint de la nueva. Si eliges una mientras el cliente está reintentando contra
otra que no responde, el cambio **corta la espera** en vez de hacerte cumplir el
castigo de la mala suerte de la anterior. Y antes de cambiar se aplica la misma
guarda de trabajo sin guardar que al salir del editor —no una segunda—, porque
las pestañas de la máquina que dejas apuntan a sesiones de **otro** daemon.

**«Probar» abre, saluda y cierra.** Informa la versión del daemon y cuánto tardó,
o por qué no respondió. No arranca nada: probar una máquina remota apagada
**falla**, que es la respuesta correcta y no un arranque remoto sorpresa.

**La presencia se lee cuando abres la lista**, nunca de fondo: un sondeo
periódico hacia máquinas remotas levantaría conexiones `ssh` que nadie pidió. Y
la presencia es la de **tu malla** (ver arriba), no una prueba de conexión: que un
equipo esté en línea no dice que allí corra `meltemid`. Eso se sabe al probar o al
conectar, nunca se supone.

**Nada agrega sesiones de varias máquinas.** Lo que ves es lo de la máquina
activa, y el cromo dice cuál. Buscar entre máquinas exigiría un agregador, y un
agregador es un servicio.

## Notas para el compañero móvil (fase 3)

Lo que sigue **no existe todavía**: es una decisión de diseño anotada para la
change `companero-movil`, no una capacidad presente. La identidad de la malla y
el selector de máquinas sí existen ya y viven arriba, cada uno en su sección.

- **Certificados SSH del bastión**: entrar al punto de encuentro con una cuenta
  del dominio propio se resuelve con un IdP autohospedado que emite
  **certificados SSH de vida corta**; el bastión y los PCs confían en esa CA. Eso
  es autorización de transporte y es otra cosa que leer la identidad de la malla:
  Meltemi no gana cuentas en ninguno de los dos casos, y la lectura contraria
  —«Meltemi con login»— choca con el rumbo («ni un servicio en la nube», sin
  lock-in) y exigiría enmienda fundacional.

## El aviso de espera (opt-in)

«Empecé algo y me fui a hacer la compra; qué bien recibir la notificación en el
teléfono.» Eso se puede, sin relé y sin puerto, y así:

```toml
# <config>/meltemi/config.toml
[notices]
on-waiting = "curl -d 'Meltemi te espera' https://ntfy.mi-casa/meltemi"
```

Lo ejecuta **el cliente conectado** —la GUI o la TUI— cuando algo empieza a
esperarte. El daemon no lo lee, no lo valida y no lo ejecuta jamás. Está
**apagado por defecto** y solo se lee de tu configuración de usuario: nunca del
`.meltemi/` de un repositorio, porque abrir un proyecto no debe ejecutar nada en
tu máquina.

### Qué recibe el comando, exactamente

Esto es un contrato, no una descripción: la constitución exige que el contenido
de cualquier aviso esté especificado antes de existir, y ampliarlo requiere una
enmienda.

| Variable | Contenido |
| --- | --- |
| `MELTEMI_WAITING` | `permission`, `instruction`, `gate` o `test` |
| `MELTEMI_SESSION` | El identificador de sesión, opaco |
| `MELTEMI_MACHINE` | El nombre de la máquina del cliente |
| `MELTEMI_AT` | El instante, RFC 3339 en UTC |
| `MELTEMI_COUNT` | Cuántas cosas esperan ahora |

Y **nada más**. No viaja la ruta del proyecto, ni su nombre, ni el texto de la
petición, ni el nombre de la herramienta, ni el título de la sesión.

La razón no es timidez. El destino es un servicio que tú operas pero que no
siempre operas solo: un topic de ntfy es una URL que otra persona puede llegar a
conocer. Lo que viaja tiene que ser inútil para cualquiera que no seas tú, y
suficiente para que decidas si vale la pena volver. «Algo espera en esta
máquina» es justo eso.

### Cuántos avisos, y cuándo no

- **Uno por espera, no uno por cosa en cola.** Si llegan tres permisos juntos,
  hay un aviso con `MELTEMI_COUNT=3`.
- **Piso de treinta segundos** entre avisos. Seis notificaciones seguidas es cómo
  uno aprende a silenciar un canal.
- **Al resolverse no se avisa.** Decirte que ya no hace falta es ruido que llega
  cuando ya vienes en camino.

### Pruébalo antes de confiar en él

En Ajustes de la GUI, y en la paleta de la TUI con `notice-test` (o
`aviso-prueba`). Usa el mismo camino que el aviso real, con
`MELTEMI_WAITING=test`, e ignora el piso: pulsarlo dos veces manda dos.

Es el único momento en que una mala configuración se ve. Un fallo del aviso real
no se grita: el cliente sigue funcionando y tú te quedas sin notificación, que es
recuperable y visible.

### Recetas, todas con tu propia infraestructura

Cada una de estas es **tuya**, como tu `ssh`. Meltemi no opera ninguna, no
gestiona sus claves y no sabe si funcionan.

**ntfy autohospedado** — el caso más simple:

```toml
on-waiting = "curl -s -d Meltemi https://ntfy.mi-casa/mi-topic-largo-y-privado"
```

Un topic corto es una dirección que alguien adivina. Usa uno largo, o pon
autenticación en tu servidor.

**Gotify**:

```toml
on-waiting = "curl -s -F title=Meltemi -F message=espera https://gotify.mi-casa/message?token=TU_TOKEN"
```

El token queda en tu archivo de configuración, que es tuyo; Meltemi no lo lee
más que para pasárselo al comando que tú escribiste.

**Apprise**, si ya lo usas para todo lo demás:

```toml
on-waiting = "apprise -t Meltemi -b espera"
```

**Sin servidor: la campana del terminal.** Si trabajas por `ssh -t` desde el
teléfono, `MELTEMI_BELL=1` hace que la TUI suene, y las apps de terminal
convierten esa campana en notificación local. Es el push más barato que existe y
no necesita nada más.

### Lo que este aviso no es

- **No es push del daemon.** El daemon no abre puertos ni habla con nadie fuera
  de su socket, y esto no lo cambia.
- **No es telemetría.** Nada sale hacia Meltemi ni hacia nadie que no sea el
  servicio que tú pusiste en esa línea.
- **No lleva tu trabajo.** Las cinco variables de arriba son el contrato entero.

## Sin push sin túnel — y por qué

No existe notificación push sin conexión, ni la habrá sin una enmienda
fundacional previa. Toda propuesta de cambio que introduzca notificaciones o
control sin un túnel establecido —un relé en la nube, un puerto de red, un push
sin túnel— **se rechaza** salvo que una enmienda a la constitución la apruebe
antes. La frontera se documenta como postura, no se negocia caso por caso.
