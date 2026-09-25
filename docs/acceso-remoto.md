<!-- SPDX-License-Identifier: Apache-2.0 -->

# Acceso remoto

Meltemi se controla en remoto sin abrir un solo puerto de red. El daemon
`meltemid` escucha únicamente en un socket local; un túnel SSH del propio usuario
lleva ese socket al otro extremo, y el daemon no distingue una conexión así de
una local. Esta página declara la **frontera vigente** del acceso remoto: qué se
puede hacer, cómo, y qué deliberadamente no existe.

## La frontera: túnel vivo

Todas las capacidades del daemon operan en remoto **únicamente a través de un
túnel vivo** del usuario hacia el socket local. Con el túnel establecido,
funciona todo; sin túnel, no funciona nada — no hay ni notificación ni control
alguno sin una conexión establecida.

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
usuario, sin túnel, los ejerce la CLI (ver [Aprobar desde el teléfono,
hoy](#aprobar-desde-el-teléfono-hoy)):

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

## Aprobar desde el teléfono, hoy

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

## El helper de túnel

`meltemi tunnel [user@host]` compone —usando el `ssh` que el usuario ya tiene— el
comando exacto de reenvío inverso (`ssh -R`) que lleva el endpoint local de este
daemon al host remoto, más el snippet equivalente de `~/.ssh/config` y el valor
de `MELTEMI_ENDPOINT` que el extremo remoto debe fijar. Con `--exec` lanza ese
`ssh` como proceso visible; jamás deja túneles en segundo plano sin pedirlo.

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

## Notas para el compañero móvil (fase 3)

Lo que sigue **no existe todavía**: son decisiones de diseño anotadas para la
change `companero-movil`, no capacidades presentes.

- **Identidad del usuario (BYO-identity)**: entrar con una cuenta del dominio
  propio se resuelve con un IdP autohospedado que emite **certificados SSH de
  vida corta**; el punto de encuentro y los PCs confían en esa CA. Meltemi no
  gana cuentas: la lectura contraria —«Meltemi con login»— choca con el rumbo
  («ni un servicio en la nube», sin lock-in) y exigiría enmienda fundacional.
- **Selector de máquinas**: varios Meltemi en varios equipos no toca el daemon;
  cada `meltemid` sigue siendo de su máquina. El multi-daemon es un concepto de
  la aplicación — una lista de endpoints con su presencia.
- **Aviso de espera**: ya especificado arriba (opt-in, mínimo, jamás emitido por
  el daemon). La infraestructura del usuario es su casa natural.

## Sin push sin túnel — y por qué

No existe notificación push sin conexión, ni la habrá sin una enmienda
fundacional previa. Toda propuesta de cambio que introduzca notificaciones o
control sin un túnel establecido —un relé en la nube, un puerto de red, un push
sin túnel— **se rechaza** salvo que una enmienda a la constitución la apruebe
antes. La frontera se documenta como postura, no se negocia caso por caso.
