# Tareas — companero-que-conecta

## 1. Un puente, dos marcadores

- [x] 1.1 Mover el actor a `core/meltemi-client/src/bridge.rs` con el trait
  `Dialer` (marcar, nombre mostrado, cambiar) y `Dial::{Transient, Terminal}`;
  `ConnState` gana `Refused`; los avisos entran como `Option<Emitter>`. El
  escritorio marca con `LocalDialer` desde su anfitrión (`desktop/src/
  bridge.rs`: marcador local, emisor de avisos), con comportamiento idéntico;
  los pins de `scenarios_shell.rs` apuntan al módulo compartido. Con los
  tests del actor sobre un marcador de prueba: transitorio reintenta y las
  peticiones en espera fallan rápido; terminal se detiene y no vuelve a
  marcar; `Switch` durante el backoff corta la espera (reloj pausado de
  tokio, `test-util` solo en dev).
- [x] 1.2 `daemon.ts` conoce `refused`; ningún consumidor del escritorio
  cambia de comportamiento.

## 2. El anfitrión móvil

- [x] 2.1 `mobile/src/host.rs`: `daemon_request`, `daemon_notify`,
  `companion_key`, `companion_destination`, `companion_connect`,
  `companion_disconnect`, `companion_forget_host`; el actor vive mientras la
  app está al frente.
- [x] 2.2 `mobile/src/dial.rs`: el marcador SSH sobre `ssh::open` con la clave
  de la app y las huellas persistidas; `HostKeyChanged` y `Rejected` son
  terminales, `Transport` es transitorio; el `Tunnel` vive en el marcador.
- [x] 2.3 `mobile/src/store.rs`: huellas de host y destino en archivos del
  directorio de datos de la app, con sus tests.
- [x] 2.4 El plugin `meltemi-vault` (Rust + Kotlin): sellar y abrir con
  AES-GCM del almacén de claves; el `Vault` móvil que lo usa; un sellado que
  falla detiene el flujo.

## 3. La pantalla que cumple

- [x] 3.1 `lib/mobile-pair.ts`: los estados de la pantalla (sin destino,
  conectando con intento y backoff, rechazada por causa) como funciones puras
  con test.
- [x] 3.2 «Sin túnel» con clave, Copiar, destino y Conectar; `refused` con su
  causa y sus acciones; Ajustes alcanzable sin conexión con clave, destino y
  «Olvidar el destino»; la superficie arranca los listeners de estado y de
  empujes que el escritorio ya arrancaba y el móvil nunca arrancó. Con
  `scenarios_mobile.rs` y los tests de UI llevando los escenarios nuevos y el
  MODIFIED; el recuento de campos de texto pasa a dos, cada uno nombrado.
- [x] 3.3 Mensajes EN/ES `mobile.pair.*` y `mobile.refused.*`;
  `nochannel.how` corregido.

## 4. La prueba

- [x] 4.1 `core/mock-sshd`: servidor `russh` que acepta una clave, ejecuta solo
  `meltemi bridge`, anuncia puerto y huella; `deny.toml` nombra el segundo
  consumidor de `russh`.
- [x] 4.2 `mobile/tests/channel.rs`: `ssh::open` → `initialize` y `status`
  contra un `meltemid` real; comando distinto rehusado; host reinstalado
  rehusado y aceptado tras olvidarlo.
- [ ] 4.3 Smoke en el emulador contra `mock-sshd` en el host: emparejar, ver
  la flota, aprobar desde el teléfono; informe en `docs/qa/`.

## 5. Documentación y cierre

- [x] 5.1 `docs/acceso-remoto.md`: cómo se empareja; en la misma red no hace
  falta nada más; la malla como opción; la trampa de
  `administrators_authorized_keys` en Windows. `mobile/README.md`: probar con
  `mock-sshd`.
- [ ] 5.2 Fila en `docs/plan-de-cambios.md` junto a `apk-en-la-release`, como
  su prerrequisito.
- [ ] 5.3 `meltemi validate` y `verify` en verde; `clippy -D warnings`,
  `fmt --check`, tests y `cargo deny` verdes; PR.
