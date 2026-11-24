# Design — companero-que-conecta

## Context

Lo que hay: un canal probado (`ssh::open` devuelve el `Stream` del cliente y
un `Tunnel` que sostiene la sesión; `AppKey` genera y sella la clave tras un
`Vault`; `HostKeys` fija la huella en el primer uso) y una superficie que no
puede llamarlo. Lo que falta es el anfitrión —el equivalente móvil de
`desktop/src/lib.rs`— y una pantalla que lo use. El actor del puente que ese
anfitrión necesita ya existe, escrito una vez para el escritorio y otra para
la TUI, con las mismas garantías (desconexión ruidosa, backoff, peticiones
concurrentes, empujes del daemon reenviados).

## Decisions

### D1 — El actor del puente se comparte; lo que cambia es quien marca

`desktop/src/bridge.rs` depende solo de `meltemi-client` y `meltemi-proto`.
Se mueve a `meltemi_client::bridge` y gana un parámetro: el **marcador**.

```rust
pub trait Dialer: Send {
    /// Opens the stream the actor will serve. Transient failures are retried
    /// with backoff; terminal ones stop the actor and are published as such.
    fn dial(&mut self) -> impl Future<Output = Result<Stream, Dial>> + Send;
    /// What the state shows as the endpoint: a socket path, or `user@host`.
    fn shown(&self) -> String;
    /// `BridgeCommand::Switch` lands here: the desktop changes endpoint,
    /// the phone changes destination.
    fn switch(&mut self, to: String);
}
pub enum Dial { Transient(String), Terminal { kind: Refused, detail: String } }
pub enum Refused { HostKeyChanged, Rejected }
```

El escritorio marca con `bootstrap::connect_or_start(&endpoint)` y nunca
devuelve terminal. El teléfono marca con `ssh::open`, guarda el `Tunnel` en el
marcador (sustituido en cada marcación, soltado cuando el actor termina) y
mapea `Refusal::HostKeyChanged` y `Refusal::Rejected` a terminal,
`Refusal::Transport` a transitorio.

Alternativas descartadas: copiar el actor al crate móvil (dos bucles que
divergen; ya hay un tercero en `tui/src/shell/conn.rs`, que esta change no
toca y una futura podría plegar); un crate nuevo solo para el actor (no tiene
más dependencias que las del cliente). La memoria de este repositorio avisa
que `desktop/tests/scenarios_shell.rs` pinea texto de `bridge.rs`: los pins se
reescriben hacia el módulo compartido, que es donde vive ahora el
comportamiento, no se relajan.

### D2 — Los avisos del sistema son del anfitrión, no del actor

`notice_for` (el aviso de espera opt-in) lee los empujes del daemon y corre un
comando del usuario. En el teléfono no hay comandos que correr, y la spec dice
que lo que trae al usuario de vuelta sale de **su** infraestructura. El actor
recibe `Option<notices::Emitter>`: `Some` en el escritorio, `None` en el
teléfono. El emisor sigue creado fuera del bucle de reconexión, por la misma
razón de siempre (una caída no resetea el piso).

### D3 — `refused` es un estado, no un diagnóstico de `unreachable`

`ConnState` gana `Refused { kind, detail, endpoint }`. La diferencia con
`Unreachable` es de **comportamiento**, no de texto: el actor sale de su bucle
y no vuelve a marcar hasta que el anfitrión lo arranque de nuevo. Una pantalla
que pudiera mostrar un host cambiado como «reintentando en 4 s» ofrecería
precisamente lo que la spec prohíbe. El escritorio nunca lo emite hoy;
`daemon.ts` conoce la variante por paridad del contrato, y ningún consumidor
del escritorio cambia de comportamiento.

### D4 — El actor vive con la app al frente, y solo entonces

`companion_connect` arranca el actor y guarda su emisor de comandos en el
estado de Tauri; `companion_disconnect` lo suelta, el actor devuelve
`HostGone` y el `Tunnel` cae con él. `Mobile.svelte` ya escucha
`visibilitychange`: oculta → desconectar; visible con destino recordado →
conectar y reconciliar. Es la suspensión honesta de `companero-movil` D4
hecha mecánica: nada sostiene el socket en segundo plano porque no hay actor
en segundo plano.

### D5 — El sello usa el almacén de Android por un plugin propio y pequeño

Un plugin móvil de Tauri, `mobile/plugins/vault/` (crate
`tauri-plugin-meltemi-vault`, `publish = false`, miembro del workspace), con
dos comandos: `seal(bytes) → blob` y `open(blob) → bytes`. En Kotlin: una clave
AES-256-GCM en `AndroidKeyStore` con alias fijo,
`setUserAuthenticationRequired(false)` (la app corre sin desbloqueo
biométrico: el deny-by-default del daemon es la puerta), IV de 12 bytes
delante del texto cifrado, base64 en el JSON del comando. El `Vault` móvil
guarda el blob en `<datos de la app>/key.sealed` y llama al plugin para sellar
y abrir; si el almacén rehúsa, `write` falla y `AppKey::load_or_create` se
detiene, como su test ya exige. En targets que no son móviles el plugin no
compila y los tests siguen con `InMemory`.

Por qué un plugin y no JNI a mano: el código Kotlin de la app vive en
`gen/android`, que es salida de build y está ignorada; un plugin lleva su
`android/` versionado y `tauri-plugin` (ya en el árbol por los plugins de
diálogo y avisos) lo enlaza al Gradle generado. Por qué no sellar con un
archivo cifrado por una clave derivada: sería exactamente la frase cómoda que
`keys.rs` se niega a decir.

### D6 — Lo que no es secreto va a archivos planos

Las huellas de host (`known_hosts.json`) y el destino recordado
(`destination`) son datos que el usuario puede leer en su `~/.ssh` de
cualquier máquina. Van al directorio de datos de la app, con `serde_json`,
detrás de los traits que ya existen (`HostKeys`) y de uno nuevo y mínimo para
el destino. «Olvidar este host» borra una huella; es la acción que un host
reinstalado necesita y la única salida honesta de `hostKeyChanged`.

### D7 — La pantalla, con la maqueta en la mano

La maqueta `design-system/Meltemi Mobile.dc.html` (tablero «1 · Conexión»)
dibujó el estado sin túnel como **glifo de conexión, no de error**: sin rojo,
sin alarma, sin promesa de push, y el estado conectando con «intento N ·
backoff». Eso se conserva. Lo que la maqueta no sabía —es anterior a D2 de
`companero-movil`— es que el teléfono habla SSH por sí mismo, así que el
«ejecuta en tu máquina `meltemi tunnel …`» se reemplaza por lo que la app sí
puede hacer:

1. **Sin destino**: el cuerpo de siempre (no hay relés ni puertos, y es
   deliberado), la línea de `authorized_keys` en monoespaciada con **Copiar**,
   la frase «añádela a `~/.ssh/authorized_keys` en esa máquina», el campo
   `usuario@host[:puerto]` (el único campo de texto nuevo; es un destino, no
   contenido, y el test que cuenta campos pasa a contar dos y a nombrar cuál
   es cada uno) y **Conectar**.
2. **Conectando**: destino, intento N, backoff, y **Cancelar**.
3. **Rechazada**: la causa en palabras y la acción que le corresponde —
   `rejected`: «la máquina no aceptó la clave: ¿está en su
   `authorized_keys`?» con la clave a mano y **Reintentar**; `hostKeyChanged`:
   «la clave de esa máquina cambió: o la reinstalaste, o no es esa máquina»
   con **Olvidar este host y volver a conectar** y **No conectar**. Nunca un
   reintento automático.
4. **Ajustes**, alcanzable sin conexión: la misma clave con Copiar, el
   destino editable, «Olvidar este host» y la frase de sin cuentas que ya
   está.

Los estados viven en `lib/mobile-pair.ts` como funciones puras con test, y la
vista los pinta.

### D8 — La prueba: un servidor SSH del workspace, en bucle local

`core/mock-sshd`, un binario con el lado servidor de `russh` (misma versión,
mismo backend `ring`; el workspace no gana dependencias): escucha en
`127.0.0.1:<puerto>`, acepta **una** clave pública (archivo por argumento) y
cualquier nombre de usuario, rehúsa todo canal que no sea `exec` con
exactamente `meltemi bridge`, y corre ese puente con el binario y el entorno
que se le indican (endpoint aislado). Anuncia por stdout una línea JSON con
puerto y huella del host. Su clave de host se genera por arranque: reiniciarlo
es «reinstalar la máquina», que es justo lo que el e2e necesita para probar
`hostKeyChanged` sin fingirlo.

Con él, `mobile/tests/channel.rs` hace lo que ningún test hacía: `ssh::open`
con la clave de la app → `initialize` y `status` contestados por un `meltemid`
real a través del puente; un comando distinto rehusado; el host reinstalado
rehusado y, tras olvidarlo, aceptado. Y el smoke del emulador empareja la app
real contra `10.0.2.2:2222`, con el daemon del host detrás, sin tocar ningún
ajuste de la máquina. Lo que el smoke de octubre dejó sin medir («teléfono →
sshd → puente») se mide así, y se mide en CI.

`deny.toml` sigue diciendo que `ring` solo entra envuelto por `russh`; gana el
comentario de que `russh` tiene dos consumidores, el cliente del teléfono y el
servidor de prueba, y `scenarios_mobile.rs` sigue afirmando que ni el
escritorio ni el cliente compartido lo tienen.

### D9 — Lo que no cambia

El transporte del cliente (`Stream`, `Peer`) no se toca: el puente se mueve,
no se reescribe. El escritorio se comporta igual: mismo actor, mismos avisos,
mismo `Switch` entre máquinas. La TUI conserva su actor espejo. El teléfono no
gana ninguna capacidad del daemon: consume lo mismo que antes, por el mismo
método.

## Risks / Trade-offs

- **El plugin Kotlin solo se compila en Android.** El job de Android de
  `apk-en-la-release` (no entregado) es donde se vería un fallo de Gradle;
  mientras tanto lo ve el build local y el smoke. Se anota en el informe.
- **El almacén del emulador es por software.** Prueba el flujo de sellado y
  el fallo honesto, no la protección por hardware. El informe lo dice.
- **Un segundo campo de texto.** El test que mantiene «un solo campo, el
  compositor» pasa a mantener «dos, y cada uno nombrado»: un destino no es
  contenido, y la spec sigue diciendo que no se redacta nada.
- **Mover el actor toca pins del escritorio.** Se reescriben con el mismo
  rigor hacia `meltemi_client::bridge`.
