<!-- SPDX-License-Identifier: Apache-2.0 -->
# Acceso remoto

[English](en/acceso-remoto.md)

Supervisa y dirige un agente desde otra máquina usando tu conexión SSH. `meltemid` escucha
únicamente en un socket local o named pipe; Meltemi no abre un puerto de red ni ofrece un relé
alojado. Configura y prueba tu SSH antes de usarlo desde la aplicación.

## La frontera: túnel vivo

Monitorear, aprobar y dirigir requieren una conexión establecida hacia el daemon. Para la GUI o
la TUI, ese túnel vivo puede ser un socket reenviado o el canal estándar de `ssh … meltemi bridge`.
También puedes ejecutar comandos individuales directamente por SSH: corren en el PC remoto y
usan su endpoint local.

Esta postura de privacidad mantiene el daemon sin puertos de red. Sin conexión no hay control ni
push del daemon. Un aviso opt-in de un cliente conectado es distinto y se configura al final de
esta guía. La misma política de permisos se aplica desde cualquier superficie.

## Desde el teléfono, hoy

### Aprobar y seguir sin la TUI

No hace falta túnel de socket, puente ni TUI para estos comandos: usa una aplicación de terminal
con SSH en el teléfono. Cada orden corre en la máquina del daemon:

```
ssh <tu-pc> meltemi permissions
ssh <tu-pc> meltemi approve perm-3
ssh <tu-pc> meltemi watch <sesión>
```

`permissions` muestra las peticiones pendientes y las opciones que ofreció el agente.
`approve <id>` elige su aprobación de una sola vez si hay exactamente una; `deny <id>` elige el
rechazo o cancela si no lo hay. `--option <id>` elige una opción concreta. Se decide una petición
por vez, nunca en bloque. La primera decisión gana; una decisión sobre una petición ya resuelta
termina con código `12`.

La llamada completa está en `ssh <tu-pc> meltemi log <sesión>`. `watch` sigue el registro hasta que
termina la sesión; Ctrl+C deja de seguirla sin cancelarla. Para cancelarla, usa
`meltemi cancel <sesión>`.

Un `watch` conectado cuenta como cliente: con la espera `while-connected`, permite que la
petición espere mientras vas a responder desde otra terminal.

### La TUI por SSH

```
ssh -t <tu-pc> meltemi
```

La TUI necesita una terminal de al menos **80×24**. En el teléfono puede hacer falta girar la
pantalla o reducir la fuente. `MELTEMI_BELL=1` activa la campana; la aplicación de terminal decide
si la convierte en un aviso. No hay push sin conexión.

## La GUI y la TUI contra tu PC

Un endpoint `exec:` ejecuta un comando de tu máquina y transporta la conexión por su entrada y
salida estándar:

```
MELTEMI_ENDPOINT="exec:ssh mi-pc meltemi bridge" meltemi status
```

Para la GUI, guarda lo mismo en la configuración **de usuario**:

```toml
# <config-dir>/config.toml
[client]
endpoint = "exec:ssh mi-pc meltemi bridge"
```

`<config-dir>` es la carpeta de configuración indicada por `meltemi doctor`; consulta
[plataformas](es/plataformas.md#rutas-de-datos). El endpoint nunca se acepta desde un repositorio.
Un endpoint `exec:` no arranca un daemon local aunque esté presente `MELTEMI_START_HERE=1`.
La cabecera de la aplicación indica que estás conectado por SSH.

### Prepara SSH

Configura el alias en tu `~/.ssh/config`:

```
Host mi-pc
    HostName 203.0.113.10
    User tu-usuario
    ServerAliveInterval 30
    ServerAliveCountMax 3
```

Comprueba primero la conexión en una terminal y confirma allí la huella del host:

```
ssh mi-pc true
ssh-add ~/.ssh/id_ed25519
```

El transporte `exec:` usa sus canales para el protocolo y no puede usarlos como diálogo para una
passphrase o una huella nueva. Meltemi no añade opciones, gestiona claves ni modifica
`known_hosts`. Si necesitas tuberías o redirecciones, ponlas en un script y usa ese script como
endpoint: la cadena del endpoint no se ejecuta mediante un shell.

## El puente: el último metro

`meltemi bridge` conecta el endpoint local de la máquina donde se ejecuta con su entrada y salida
estándar. Es el último metro de la conexión y permite llegar también al named pipe de Windows:

```
ssh <tu-pc> meltemi bridge
```

Si el daemon no escucha, el puente falla y muestra el siguiente paso. Inicia el daemon en el PC
con la aplicación de escritorio o con `meltemi status` antes de conectarte desde fuera.

## Sockets Unix y el helper de túnel

Con un daemon macOS/Linux, `meltemi tunnel <user@host>` imprime el reenvío inverso `ssh -R`, la
configuración equivalente y el endpoint que debe usar el otro extremo. `--exec` ejecuta tu SSH
visiblemente. También puedes usar reenvío local desde un cliente macOS/Linux:

```
ssh -L <local.sock>:<daemon.sock> <host>
```

El helper incorpora latidos y `ExitOnForwardFailure=yes` para detectar una conexión caída.
Un cliente apuntado a un socket reenviado no inicia otro daemon cuando el túnel falla.

Un named pipe de Windows no se puede reenviar como un socket Unix: usa `meltemi bridge`.
Un PC Windows sí puede ser cliente de un daemon Unix. En Git Bash, consulta el ajuste
`MSYS_NO_PATHCONV` de [plataformas](es/plataformas.md#windows-git-bash).

## Windows como PC servido

Habilita el servidor OpenSSH en una PowerShell de administrador del PC:

```
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
Start-Service sshd
Set-Service -Name sshd -StartupType Automatic
```

Con `meltemi` en el `PATH` remoto, usa `ssh <tu-pc> meltemi bridge`, los comandos individuales o
`ssh -t <tu-pc> meltemi`. El SSH, su acceso de usuario y el firewall los administra el propietario
de esa máquina.

**Medido** por las pruebas del producto: el puente sobre named pipe y los comandos sin terminal.
**No medido**: el tramo a través de un `sshd` real de Windows ni el inicio del daemon desde esa
sesión SSH. Prueba tu recorrido completo antes de depender de él; las pruebas locales no lo
sustituyen. La evidencia histórica está en el [registro de verificación](qa/2026-09-25-acceso-por-tunel-smoke.md).

## Si las dos máquinas están fuera de casa

Puedes usar tu propia VPN o un punto de encuentro SSH que administres. Con un reenvío inverso,
el PC marca hacia afuera y el cliente también se conecta hacia afuera al punto de encuentro;
el salto final ejecuta `meltemi bridge` dentro del PC.

| PC | Cliente | Camino |
| --- | --- | --- |
| En casa | Fuera | cliente → encuentro → túnel inverso → PC |
| Fuera | En casa | el PC mantiene el mismo camino saliente |
| Fuera | Fuera | ambos llegan al mismo punto de encuentro |
| En casa | En casa | el mismo camino o una conexión SSH directa |

El punto de encuentro apagado significa sin acceso remoto; el trabajo local sigue disponible.
Esta es infraestructura del usuario (BYO-network), no se empaqueta con Meltemi. El proyecto no
exige cuenta ni red para sus pruebas locales. La VPN facilita el camino de SSH, no concede
permisos nuevos al daemon.

## Identidad: la de tu malla

Meltemi no tiene cuentas. Opcionalmente puede leer la identidad y presencia que ya conoce tu
cliente Tailscale/Headscale. Configúralo solo en el archivo **de usuario**:

```toml
# <config-dir>/config.toml
[identity]
mesh = "tailscale"
login-server = "https://headscale.mi-casa"
command = "tailscale"
```

Una tabla `[identity]` dentro del proyecto se ignora con diagnóstico. `meltemi identity` hace una
consulta de solo lectura al cliente configurado (`status --json`), con tiempo límite. El daemon
no ejecuta `up`, `login` ni `logout`. Presencia no es control: un equipo en línea no prueba que
esté ejecutando `meltemid`; debes conectar o probar su endpoint.
El camino al daemon sigue siendo tu túnel SSH.

- `not_configured`: agrega la configuración si quieres esta función.
- `client_missing`: comprueba el binario y la ruta configurada.
- `not_linked`: inicia sesión con tu cliente de malla.

`meltemi identity-link` compone el gesto de inicio de sesión y el daemon jamás ejecuta ese login.
Con `meltemi identity-link --exec`, el CLI ejecuta tu cliente con entrada y salida heredadas para
que completes su propio login. La GUI permite copiar el gesto. En la TUI usa `:identity` o
`:identity-link`; en la GUI está en **Ajustes → Conexiones**.

La interoperabilidad se contrastó con las definiciones de tipo de Tailscale `v1.102.5`; la
verificación de campo con un cliente instalado y enlazado sigue pendiente. No es una garantía
sobre otra versión del cliente.

## Tus máquinas, con nombre

Guarda los endpoints en tu configuración **de usuario**:

```toml
# <config-dir>/config.toml
[[client.machines]]
name = "esta"
endpoint = "local"

[[client.machines]]
name = "la-torre"
endpoint = "exec:ssh la-torre meltemi bridge"
```

La lista va sin material de claves; tu `~/.ssh/config` conserva esos datos. Un campo de clave no
admitido rehúsa la máquina entera en vez de aceptarlo en silencio.

`meltemi machines` lista y no cambia el destino de otros clientes. En la TUI, `:machines` lista y
`:machine la-torre` selecciona. En la GUI, usa **Ajustes → Conexiones**, **Probar** y **Usar esta**.
Cambiar de máquina es reconectar; antes se comprueba si tienes trabajo sin guardar.
**Probar** no arranca nada: abre, saluda y cierra. La presencia se lee a petición, nunca de fondo;
no dice que allí corra el daemon. No se agregan sesiones de máquinas distintas: ves la activa.

## El compañero móvil

El compañero Android puede monitorear, aprobar, revisar y dirigir; no es un editor de código ni
de especificaciones. Existe en el código del producto, pero la release publicada no distribuye
un APK. La [guía de compilación](../mobile/README.md) describe el build para desarrolladores.
No hay distribución iOS publicada.

El teléfono usa su propio cliente SSH hasta `meltemi bridge`. La app genera una clave propia y
muestra su parte pública para añadirla a `authorized_keys` en el servidor. Revoca el acceso
retirando esa línea. La clave privada queda cifrada en el almacenamiento de la app; el almacén
del sistema protege el material usado para cifrarla. Un cambio de clave del host rechaza la
conexión y se muestra como tal.

Al pasar a segundo plano no mantiene el socket. Al volver, reconecta y actualiza el estado antes
de mostrarlo como actual. No hay un relé de Meltemi ni notificaciones del daemon sin conexión.

## El aviso de espera (opt-in)

Un cliente GUI o TUI conectado puede ejecutar un comando tuyo cuando algo empieza a esperar.
Está apagado por defecto. El comando se lee solo de tu configuración de usuario:

```toml
# <config-dir>/config.toml
[notices]
on-waiting = "curl -s -d Meltemi https://ntfy.mi-casa/mi-topic-largo-y-privado"
```

Sustituye el ejemplo por tu propio servicio y un comando disponible en tu máquina. El daemon no
lo lee ni ejecuta. El comando recibe únicamente estas variables de evento:

| Variable | Contenido |
| --- | --- |
| `MELTEMI_WAITING` | `permission`, `instruction`, `gate` o `test` |
| `MELTEMI_SESSION` | Id opaco de sesión |
| `MELTEMI_MACHINE` | Nombre de máquina del cliente |
| `MELTEMI_AT` | Instante RFC 3339 en UTC |
| `MELTEMI_COUNT` | Cantidad de elementos que esperan |

El evento no incluye nombres o rutas del proyecto, texto de la petición, herramienta ni título
de la sesión. Llega un aviso por transición a espera, con al menos treinta segundos entre
avisos reales; resolver una espera no envía otro.

Pruébalo desde **Ajustes → Herramientas** o con `notice-test` en la paleta de la TUI
(también `aviso-prueba`). La prueba usa `MELTEMI_WAITING=test` e ignora el intervalo mínimo.
Un comando fallido no impide trabajar, pero no recibirás el aviso. Su servicio, permisos y
credenciales los administras tú; Meltemi no verifica que ese destino entregue una notificación.

## Cuánto se espera al humano

Estos ajustes van en `config.toml` de usuario o `.meltemi/config.toml` del proyecto; el proyecto
prevalece:

| Ajuste | Por omisión | Qué hace |
|---|---|---|
| `[permissions] wait` | `"while-connected"` | Espera interactiva mientras haya un cliente; también acepta segundos. Al vencer, deniega la petición. |
| `[permissions] implement-wait` | `30` | Segundos de espera en los turnos de `implement`. |
| `[permissions] no-client-grace` | `30` | Segundos que tolera una desconexión antes de denegar peticiones pendientes. |
| `[sessions] idle-timeout` | `900` | Segundos que una sesión conserva al agente esperando otra instrucción; `0` no espera. |
| `[sessions] max-idle` | `3` | Sesiones que pueden esperar a la vez; al alcanzar el límite cierra la espera más antigua. |
| `[sessions] lever-wait` | `30` | Segundos para que el agente confirme el modelo o esfuerzo solicitado. |

Un cliente conectado cuenta para `while-connected`, incluida la GUI, la TUI o `meltemi watch`
por SSH. La conexión no concede permisos automáticamente: la persona o una regla deben decidir.
