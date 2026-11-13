<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `companero-que-conecta` (tarea 4.3)

**Fecha**: 2026-11-12 · **Plataforma de build**: Windows 11 Pro ·
**Dispositivo**: emulador Android, AVD `Medium_Phone_API_36.1` (Android 16,
API 36, x86_64) · **APK**: debug universal, `cargo tauri android build --debug
--target x86_64 --apk` con el CLI nativo de Tauri, plugin Kotlin del
almacén (`mobile/plugins/vault`) incluido.

El objetivo de la tarea: emparejar la app con un daemon **real** a través del
canal SSH **propio** de la app, ver lo que el daemon contesta y aprobar un
permiso desde el teléfono. Nada de esto toca el `sshd` ni el daemon del
usuario: el lado remoto es `core/mock-sshd` y el daemon es uno aislado.

## Montaje

| Pieza | Cómo |
| --- | --- |
| Daemon | `meltemid` release 0.2.0 en `\.\pipe\meltemi-companion-smoke`, con `MELTEMI_CONFIG_DIR` y `MELTEMI_DATA_DIR` propios y `MELTEMI_START_HERE=1`; el daemon del usuario siguió en su endpoint, intacto |
| Lado remoto | `mock-sshd --listen 127.0.0.1:2222 --authorized <la línea de la app> --bridge target\release\meltemi.exe --env MELTEMI_ENDPOINT=… --env MELTEMI_CONFIG_DIR=… --env MELTEMI_DATA_DIR=…`; anunció `{"port":2222,"hostKey":"AAAAC3…NnL"}` |
| Destino en la app | `anyone@10.0.2.2:2222` — `10.0.2.2` es el host visto desde el emulador; el usuario es irrelevante para `mock-sshd`, que mira la clave |
| Agente | `mock-agent` por `[agent] command` en el `.meltemi/config.toml` de un repositorio fixture temporal |

La clave de la app se leyó **de la pantalla** (árbol de accesibilidad, no un
archivo de la app) y se escribió tal cual al archivo de `--authorized`: el
mismo gesto que haría una persona con «Copy».

## Qué se midió

| Escenario | Medido |
| --- | --- |
| El APK instala y arranca | el primer `adb install -r` falló con `INSTALL_FAILED_UPDATE_INCOMPATIBLE`: el debug de octubre estaba firmado con otra clave de depuración; `adb uninstall dev.meltemi.mobile` y de nuevo `install` → `Success`; `dev.meltemi.mobile/.MainActivity` en primer plano |
| Sin túnel, la pantalla dice qué falta | arranca en **«No tunnel»**; debajo, en este orden: por qué («There are no relays and no open ports, and that is deliberate»), qué hacer («Copy this app's key into your authorized_keys on that machine, type user@host and connect»), la línea `ssh-ed25519 … meltemi-mobile` con **Copy**, el único campo **Destination** (`user@host[:port]`), **Connect**, y la nota de misma red / VPN propia / sin relés |
| La clave es de la app | la línea apareció en el primer arranque, generada y sellada por el almacén de Android a través del plugin; **ningún campo** de la pantalla admite una clave privada |
| Emparejar | se escribió el destino y se pulsó Connect; en menos de 6 s la vista **Sessions** mostró la respuesta real del daemon: «Nothing running right now.» |
| …por el canal propio y solo por el puente | en el host apareció `meltemi.exe bridge` como hijo de `mock-sshd`; el log del daemon registró `client initialized client=meltemi-mobile client_version=0.2.0`; `mock-sshd` no rechazó ningún comando (stderr vacío) |
| Lo que Settings muestra ya conectada | el destino recordado en su campo, **Connect** y **Forget the destination**, y de nuevo la clave con Copy; sin cuentas («Meltemi has no login and no server») |
| Un permiso, desde el teléfono | con `meltemi session "…" <fixture>` esperando en una terminal real, la pestaña **Permissions** mostró la tarjeta con **Allow** / **Reject**; Allow → «Decided.» y «Nothing is waiting on you.»; la terminal dijo «→ withdrawn: it was resolved elsewhere (decided on another surface, or its session stopped)» y `session … [completed]` con su punto de restauración |
| …y el daemon lo registró como tal | en el JSONL de la sesión `139bdc09…`: `permission_requested` a las 21:56:13Z y `permission_decided` a las 21:56:41Z con `decidedBy: "client"` y `optionId: "allow"` |
| Una sesión viva se ve | segunda sesión (con `PROPOSAL_PATH:` en la instrucción): la pestaña **Sessions** mostró la tarjeta con la instrucción, el estado `waiting_permission` y **Direct**; la pregunta en la terminal nombró la ruta; Allow desde el teléfono → `mock-agent` escribió `proposal-smoke.md` (107 B) y la sesión terminó `[completed]` |
| Ocultar y volver | HOME → a los 5 s **cero** procesos `bridge` en el host (la app cerró el canal); relanzada → a los 7 s de nuevo **uno**, la vista Sessions con la respuesta del daemon y sin tocar nada |
| Un host reinstalado | `mock-sshd` detenido y arrancado de nuevo (clave de host nueva); a los 10 s la app mostró «No tunnel» con la alerta «The key of anyone@10.0.2.2:2222 changed: either you reinstalled that machine, or this is not that machine. Nothing was sent.», el detalle con la clave esperada y la ofrecida, y los botones **Forget this host and connect again** (a la izquierda) y **Do not connect** (primario, a la derecha); **no reintentó sola** |
| …y olvidarlo es un gesto | Forget → a los 7 s un `bridge` nuevo bajo el `mock-sshd` nuevo, y la app de vuelta en la pestaña en la que estaba (Permissions, «Nothing is waiting on you.») |

El e2e de `mobile/tests/channel.rs` ya cubría el canal, el rechazo de otra
clave y el cambio de clave de host en bucle local; lo que este smoke añade es
el **teléfono**: el almacén real, la pantalla real, los gestos y lo que pasa
al ocultar la app.

## Lo que el dispositivo corrigió

La pantalla de rechazo quedó **más ancha que el teléfono**: el detalle del
marcador cita las dos claves de host (la esperada y la ofrecida), una clave no
tiene por dónde partirse, el párrafo ensanchó la página y, con ella, **dejó de
ajustarse todo lo demás** —la alerta y el texto de arriba se cortaban por la
derecha y apareció una barra de desplazamiento horizontal. En el escritorio no
se ve: ahí la ventana es más ancha que una clave.

Corregido en `Mobile.svelte`: los dos párrafos que muestran las palabras del
marcador (conectando y rechazado) llevan `overflow-wrap: anywhere`, como ya
hacía la línea de la clave. `desktop/ui/tests/mobile-nav.test.ts` lo sostiene
contra la fuente (una clase y una regla son lo que un refactor pierde en
silencio). Vuelto a construir y a medir en el emulador el 2026-11-13:

- **Antes** (APK de la primera pasada): el párrafo del detalle desbordaba la
  pantalla; la captura mostraba la alerta cortada por la derecha y una barra
  de desplazamiento horizontal.
- **Después**: con `mock-sshd` detenido y arrancado de nuevo, la misma
  pantalla «No tunnel» con la alerta, el detalle con las dos claves partidas
  a mitad de cadena, y los dos botones enteros. Medido sobre el árbol: los
  diez nodos con texto están centrados en x = 540 y **ninguno pasa de
  1080 px** (el detalle llega justo al borde, como el resto de la página);
  en la captura no hay recorte ni barra. Forget → a los 6 s el daemon
  registró otro `client initialized client=meltemi-mobile` y la app volvió a
  **Sessions**.

Para esta segunda pasada el APK se construyó con la variable de entorno
`CARGO_PROFILE_DEV_STRIP=debuginfo` (de 375 MB a 191 MB: el `/data` del
emulador tenía menos de 1 GB libre y el instalador rechazaba el APK completo
con `INSTALL_FAILED_INSUFFICIENT_STORAGE`); nada cambia en el árbol por ello. La
reinstalación descartó la clave de la app, así que `mock-sshd` se arrancó con
la línea nueva leída otra vez de la pantalla. App y `mock-sshd` llevaban ya
**russh 0.65.1** (subido en 5.3 por RUSTSEC-2026-0495); el emparejamiento, el
rechazo del host reinstalado y la vuelta tras olvidarlo se midieron con esa
versión en los dos extremos.

## Observaciones que se quedan como están

- La última palabra de la app («Decided.», «Forgotten…») se queda en pie hasta
  la siguiente acción: es la última palabra del actor, no un aviso con
  temporizador, y así está diseñado. Verla debajo de la pantalla de rechazo
  es correcto, aunque llame la atención.
- La tarjeta del permiso decía «permission / permission»: el `mock-agent` no
  nombra la herramienta ni dice qué va a hacer, y la terminal mostró lo mismo
  («(not named by the agent)»). Un agente real nombra ambas cosas.
- `meltemi session` **sin terminal** deniega el permiso en el acto (diseño D1
  de `aprobador-en-el-cli`): para que el teléfono tenga algo que decidir, la
  sesión debe partir de una terminal real o del escritorio, nunca de un proceso
  con la salida redirigida. No es un fallo; es la regla de que sin nadie que
  pregunte no hay permiso, y conviene saberlo al repetir este smoke.

## Lo que no se midió

- Un teléfono físico, otra red que la NAT del emulador, ni el consumo de
  batería con la app al frente.
- Un `sshd` real: en Windows, OpenSSH lee las claves de un administrador en
  `C:\ProgramData\ssh\administrators_authorized_keys`, no en `~/.ssh`
  (`docs/acceso-remoto.md` lo dice); `mock-sshd` no tiene esa trampa.
- La firma de release del APK, que es del mantenedor (`apk-en-la-release`).
