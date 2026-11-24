# companero-que-conecta

> Vía completa (proposal → design → specs → tasks). No por el tamaño del
> código —mediano— sino porque toca tres cosas a la vez: mueve el actor del
> puente del escritorio al cliente compartido, mete código nativo de Android
> (el sello de la clave en el almacén del sistema) y añade un servidor SSH de
> prueba al workspace. Nace de una pregunta del mantenedor del 2026-10-02
> («¿tengo que instalar dos apps?») que llevó a abrir Ajustes del móvil y
> encontrar dos líneas.

## Why

`companero-movil` construyó y probó el canal (`mobile/src/ssh.rs`,
`mobile/src/keys.rs`) y la superficie de cuatro verbos, y los entregó sin
conectar entre sí. Hoy, en `main`:

- `mobile/src/lib.rs` es `tauri::Builder::default().run(…)`: no registra
  ningún comando. Nada expone el canal al webview.
- El webview del móvil importa `lib/daemon.ts`, que invoca `daemon_request` y
  escucha `daemon:state`: el contrato del **puente del escritorio**, que vive
  en `desktop/src/bridge.rs` y que el crate móvil no puede usar.
- La vista Ajustes del móvil es un título y una frase. No hay campo para el
  destino, ni sitio donde ver la clave pública de la app, ni botón de conectar.
- Toda la superficie vive detrás de `connected`, y la pantalla «Sin túnel»
  manda a «conéctate desde Ajustes».

El resultado es un círculo: el APK firmado el 2026-10-02 arranca, dice qué
falta y no ofrece manera de hacerlo. La spec no mintió —dice «un túnel SSH
gestionado por el usuario» y nunca pidió emparejar desde la app—, así que el
7/7 de verificación fue honesto; pero la app promete lo que no cumple, y
`apk-en-la-release` (D7) da por hecho «conectar por el túnel SSH con la clave
del compañero» en la prueba de un desarrollador. Sin esto, publicar el APK es
publicar una app que no conecta.

La confusión de las «dos apps» se aclara de paso: el teléfono necesita llegar
a tu `sshd`, y en la misma red eso no pide nada más. La malla es una opción
para cuando estás fuera, no un requisito, y esta change lo escribe en la
pantalla.

## What Changes

- **Un puente, dos marcadores.** El actor del puente deja `desktop/src/` y
  entra a `meltemi-client::bridge`, parametrizado por quien marca: el
  escritorio con `connect_or_start(endpoint)`, el teléfono con
  `ssh::open(destino, clave, hosts)`. El contrato hacia el webview
  (`daemon_request`, `daemon_notify`, `daemon:state`, `daemon:incoming`) es el
  mismo en los dos, así que `daemon.ts` y todas las stores corren en el
  teléfono sin cambios. El escritorio conserva su archivo como capa de
  anfitrión (avisos del sistema, reenvío de eventos).
- **El anfitrión móvil.** `mobile/src/lib.rs` registra `daemon_request` y
  `daemon_notify` con los mismos nombres, más `companion_key` (la línea para
  `authorized_keys`), `companion_destination` (la recordada),
  `companion_connect`, `companion_disconnect` y `companion_forget_host`. El
  actor corre solo con la app al frente: `visibilitychange` lo apaga al
  ocultarse y lo arranca al volver, que es la suspensión honesta que la spec
  ya exige.
- **Una clave rechazada es terminal, no un reintento.** `daemon:state` gana
  el estado `refused` (`hostKeyChanged` | `rejected`): el actor detiene su
  bucle y la pantalla lo muestra como su propio estado con las dos
  posibilidades («reinstalaste esa máquina, o no es esa máquina»; «la clave no
  está en `authorized_keys`»). Un fallo de transporte sigue con backoff,
  porque el túnel va y viene.
- **El sello, de verdad.** Un plugin móvil de Tauri en Kotlin
  (`meltemi-vault`) sella y abre bytes con una clave AES-GCM del almacén de
  claves de Android; el `Vault` de Rust lo llama. En el escritorio y en los
  tests sigue `InMemory`. Las huellas de host y el destino recordado van a
  archivos del directorio de datos de la app: no son secretos.
- **La pantalla cumple lo que promete.** «Sin túnel» pasa a ser la pantalla
  de emparejar: la clave pública con copiar, el destino
  (`usuario@host[:puerto]`), Conectar, y mientras conecta el estado honesto
  de la maqueta (intento N, backoff). Ajustes se alcanza sin conexión y lleva
  la misma clave, el destino y «olvidar este host». `nochannel.how` deja de
  apuntar a un sitio que no existe.
- **La prueba en bucle local.** Un crate `core/mock-sshd` (servidor `russh`):
  acepta exactamente una clave pública, corre `meltemi bridge` del binario que
  se le indique como canal `exec`, contra un endpoint aislado. Lo usan un e2e
  en `mobile/tests/` (`ssh::open` → `initialize` → `status` contra un daemon
  real) y el smoke en el emulador (la app empareja contra `10.0.2.2:2222`).
  Ni `sshd` del sistema ni ajustes de la máquina: bucle local y nada más.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `mobile-companion`: + «El emparejamiento vive en la app»; + «Un puente,
  compartido»; + «El canal se prueba contra un daemon real»; MODIFIED «Sin
  túnel, la pantalla dice qué falta» (la misma pantalla ofrece el camino).

## Impact

- Archivos: `core/meltemi-client/src/bridge.rs` (nuevo, desde
  `desktop/src/bridge.rs`), `desktop/src/{bridge.rs,lib.rs}`,
  `mobile/src/{lib.rs,host.rs,dial.rs,store.rs,vault.rs}`,
  `mobile/plugins/vault/` (Rust + Kotlin), `core/mock-sshd/` (nuevo),
  `mobile/tests/channel.rs`, `desktop/ui/src/Mobile.svelte`,
  `desktop/ui/src/lib/{daemon.ts,messages.ts,mobile-pair.ts}`,
  `desktop/tests/scenarios_mobile.rs`, `docs/acceso-remoto.md`,
  `mobile/README.md`, `deny.toml`, `docs/plan-de-cambios.md`.
- Dependencias: **ninguna nueva en el árbol**. `russh` gana un segundo
  consumidor (`core/mock-sshd`, el servidor de prueba); `tauri-plugin` ya
  entra por los plugins existentes. `deny.toml` nombra el segundo consumidor
  y mantiene a `ring` con `russh` como único envoltorio.
- Riesgo: el plugin Kotlin compila solo en el job de Android; el almacén del
  emulador es por software (vale para probar el flujo, no la protección por
  hardware). El actor cambia de archivo y hay tests que pinean su texto: se
  actualizan hacia el módulo compartido, no se relajan.

## Fuera de alcance

- El APK en la release y su firma en CI: `apk-en-la-release` y
  `firma-de-plataforma`. Esta change es su prerrequisito, no su sustituta.
- Firmar **por** el almacén (backend de firma en la biblioteca SSH vía JNI):
  sigue como está escrito en `keys.rs`, dicho y no insinuado.
- iOS, push, relés: no.
- Los tres defectos anotados el 2026-10-02 (el `&` del gesto para PowerShell,
  `*.p12` en el ignore, la fidelidad del fixture `unlinked`): siguen siendo
  suyos, no de esta change.
