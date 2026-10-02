# Tareas — companero-movil

Vía completa. Un commit atómico por tarea, con referencia
`(companero-movil N.M)` y sin trailers de co-autoría. Gates del repo en cada
tarea: `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo deny check`
y la suite del crate tocado. **Esta change se desarrolla en su propio taller
(`meltemi workspace companero-movil`) y aterriza en `main` con `meltemi
land` al cerrar.**

Esta es la única change del proyecto que **añade una dependencia de peso y
toca `deny.toml`**, y lo hace en la dirección contraria a la esperada: no
pide una excepción, añade dos prohibiciones (design D2). Ningún método nuevo
del contrato: si alguna tarea lo necesitara, la change se detiene y vuelve a
design, porque sería la superficie móvil siendo la única consumidora de algo
(regla de subconjunto, §4).

## 1. El canal

- [x] 1.1 `deny.toml`: prohibir `ring` y `aws-lc-rs` con su razón escrita —las
  features opcionales de `russh` son la puerta que nadie debe abrir sin
  argumentarlo aquí—, y dejar anotado que SSH no es HTTP ni TLS y que por eso
  no hace falta excepción alguna (design D2) — escenario «La malla del usuario
  no es una dependencia de Meltemi» (vigente) — gates: `cargo deny check`
  <!-- 2026-10-02: confirmado contra crates.io antes de escribir una línea:
  `russh 0.63.3` es **Apache-2.0** —la misma licencia del proyecto— y su árbol
  **obligatorio** es RustCrypto puro más `tokio` y `futures`, que el workspace ya
  tiene. Ni `rustls`, ni `openssl`, ni `native-tls`, ni `hyper`, ni `reqwest`. De
  ahí que `deny.toml` **gane prohibiciones en vez de una excepción**: `ring` y
  `aws-lc-rs`, que son las dos features opcionales de russh y la única puerta que
  deja entornada. Quien las encienda argumenta aquí por escrito, como `reqwest`.
  Declarado porque §10 lo pide: russh pina `ssh-key =0.7.0-rc.11`, un candidato
  de versión, con `=` y no con un rango. Las alternativas son peores —
  `libssh2-sys` es C y arrastra OpenSSL, prohibido; `thrussh` está sin mantener —
  así que se acepta y queda escrito donde se audita.
  Hallazgo que simplifica el resto: `meltemi_client::transport::Stream` es
  `Box<dyn RawStream>` sobre **cualquier** `AsyncRead + AsyncWrite + Unpin +
  Send`. El canal SSH entra en la pila de cliente existente **sin tocar una línea
  de `meltemi-client`**, que es la forma más fuerte de que «el daemon no gana
  nada» sea verdad: tampoco gana nada el cliente.
  Gates: `cargo deny check` limpio con las dos prohibiciones nuevas. -->
- [x] 1.2 `mobile/` (crate nuevo) con `russh` pineado: diálogo SSH hasta un
  canal de sesión que ejecuta el comando puente, expuesto como el mismo
  `Stream` que el resto del cliente ya consume, con el host key fijado en el
  primer uso y comprobado después (design D2) — escenarios «El canal del
  teléfono termina en el socket local», «El puente es el mismo» y «La clave del
  host cambia» — gates: suite del crate
- [x] 1.3 `mobile/`: el par de claves de la app —generado en el teléfono,
  pública mostrada para autorizar, privada cifrada en reposo con material del
  almacén del sistema—, con la frontera escrita en el código: ni se leen ni se
  piden claves ajenas (design D3) — escenario «La clave es de la app y de nadie
  más» — gates: suite del crate
  <!-- 2026-10-02: **el design D2 estaba equivocado en su afirmación central y
  queda enmendado aquí**. Decía que `ring` y `aws-lc-rs` eran opcionales y se
  quedaban apagados, y que por eso `deny.toml` no necesitaba excepción. La
  realidad: `russh` **exige** uno de los dos —se niega a compilar sin ninguno— y
  además trae `aws-lc-rs` **en sus features por defecto**. Los metadatos de
  crates.io no lo dicen: ahí los dos figuran como «opcionales», y lo son como
  features, pero una es obligatoria en tiempo de compilación.
  Lo que sí funcionó es el guardián: la prohibición de `aws-lc-rs` **mordió en el
  momento exacto** en que se añadió la dependencia, antes de que nada se
  compilara, y señaló la cadena entera. Para eso existe.
  Decisión corregida: `default-features = false` y `features = ["ring"]`.
  `ring` es Apache-2.0 AND ISC —las dos ya en la lista—, es Rust y ensamblador
  sin toolchain de C, y compila para los targets de Android; `aws-lc-rs` arrastra
  `aws-lc-sys`, que es C y pide NASM en Windows. Así que `deny.toml` tiene ahora
  una **excepción acotada** —`ring` solo como envoltorio de `russh`, igual que
  `tauri` lo es de `reqwest`— y mantiene la **prohibición** de `aws-lc-rs`. Sin
  `rsa` ni `flate2`: ed25519 y sin compresión es todo lo que esta app necesita.
  `ssh-key` se declara para **encender su propia feature `getrandom`** y nada
  más: ya está en el árbol porque russh la pina exacta, y `rand_core` 0.10 movió
  `OsRng` fuera, así que tomar el RNG de otro sitio del árbol es como empieza el
  desajuste de versiones.
  **Un test mordió antes de que el código saliera**: `fe80::1` se leía como el
  puerto 1 de un host llamado `fe80:`, porque partir por el último dos puntos no
  distingue una dirección IPv6 de un puerto. Ahora los corchetes mandan, como en
  todas partes donde se escriben host y puerto juntos.
  `AppKey` **no deriva `Debug` a propósito** —una clave que se puede imprimir
  acaba en un log—, y los tests pagan ese precio comparando contra el error en
  vez de pedir que el lado bueno sea imprimible.
  El hallazgo que sostiene todo: el canal SSH se entrega como
  `Box<dyn AsyncRead + AsyncWrite + Unpin + Send>`, que es exactamente lo que el
  `Stream` del cliente ya es. **Ni una línea de `meltemi-client` cambia.**
  Gates: fmt, clippy del crate en 0, `cargo deny check` 0 con la excepción
  acotada, 11 tests del crate verdes. -->

## 2. La superficie

- [x] 2.1 `desktop/ui`: segunda entrada `mobile.html` + `Mobile.svelte` que
  reutiliza stores, catálogo ES/EN, i18n y los módulos de decisión ya probados,
  con navegación inferior de cuatro destinos y objetivos de 44 px; la pantalla
  sin canal dice qué falta y cómo se arregla (design D1, D6) — escenario «Sin
  canal» — gates: `npm run check`, lint de i18n, suite de UI
  <!-- 2026-10-02: **una segunda entrada de Vite, no una segunda aplicación**:
  `mobile.html` + `mobile.ts` + `Mobile.svelte` en el mismo bundle, compartiendo
  todo lo que vive debajo del shell —stores, catálogo ES/EN, i18n, los módulos de
  decisión— y cambiando solo lo que va encima. Encoger el shell de escritorio
  habría sido lo contrario de un compañero: la barra lateral y las tres zonas
  existen porque hay ratón y 1.280 píxeles.
  **La lista de destinos vive en un módulo con su test**, no en el markup: que
  sean cuatro es el límite que la spec pone, y un quinto exige enmienda
  fundacional, no una tarde. `mayAuthor()` devuelve `false` como **función**
  ejecutada y no como comentario, por la misma razón. Dirigir no es un destino:
  es algo que se le hace a una sesión que estás mirando, no un sitio al que ir, y
  su ausencia de la lista es deliberada.
  La pantalla sin canal **no es un error**: dice qué falta y cómo se arregla, y
  no ofrece ninguna otra vía — ni relé, ni puerto, ni cuenta.
  `env(safe-area-inset-*)` en los dos extremos: una navegación inferior debajo del
  indicador de inicio es una navegación que nadie puede pulsar, y el notch de
  arriba no es nuestro para pintar debajo.
  Gates: `npm run check` 0 errores sobre 156 archivos, lint de i18n limpio, 107
  tests de UI y el bundle produce **las dos** entradas. -->
- [x] 2.2 Monitorear y dirigir: sesiones por estado con `session/list` y
  `session/state`, y el compositor mínimo con `session/direct` y
  `session/cancel` (design D5) — gates: suite de UI
- [ ] 2.3 Aprobar y revisar: la petición que espera con sus opciones
  (`permission/pending`, `permission/decide`, `permission/changed`), y la
  revisión de una carrera —diffs, gates, checklist, adoptar o revertir **con
  confirmación**, sin editar— (design D5) — escenarios «Aprobación de una
  petición de permiso desde el móvil», «Adopción de un archivo de un competidor
  desde el móvil» y «Propuesta de autoría en el móvil» (vigentes) — gates:
  suite de UI
- [x] 2.4 Suspensión y vuelta: nada sostiene el socket en segundo plano, y al
  volver al frente se reconecta y se reconcilia antes de mostrar nada como
  actual (design D4) — escenarios «La app vuelve al frente» y «La app está en
  segundo plano» — gates: suite de UI
  <!-- 2026-10-02: dirigir **no es un destino** de la navegación: es algo que se
  le hace a una sesión que estás mirando, así que el compositor se abre dentro de
  su fila y se cierra al enviar. Y es texto **para el agente**, que cae del lado
  de decidir y no del de redactar, que es donde la spec pone la línea.
  La reconciliación cuelga de `visibilitychange` y no de un temporizador: nada
  sostiene el socket en segundo plano (D4), así que lo que había en pantalla es el
  recuerdo de otro momento y se vuelve a preguntar antes de presentar nada como
  actual.
  Los permisos se deciden **de uno en uno**, con las palabras del propio agente
  para cada opción —no «Sí/No» inventados aquí—, y el aviso de resultado va al
  pie en vez de desplazar la lista, porque el layout no debe moverse bajo el
  pulgar mientras alguien lee.
  Gates: `npm run check` 0 errores, lint de i18n limpio, 107 tests de UI. -->

## 3. El APK

- [ ] 3.1 `mobile/tauri.conf.json` + `tauri android init`: el proyecto Android
  generado, `.gitignore` de lo generado y de cualquier keystore, y el APK de
  depuración corriendo en el emulador contra el daemon de este equipo (design
  D7) — gates: el APK instala y conecta
- [ ] 3.2 Presupuesto y release: medir el APK de release por ABI contra el
  techo declarado (≤ 25 MB) y **escribir lo medido**; `arm64-v8a` como ABI de
  release, `x86_64` solo para el emulador; la firma queda como tarea del
  mantenedor con su propio keystore (design D7) — gates: medición escrita

## 4. Cierre

- [ ] 4.1 `docs/acceso-remoto.md` y `docs/plataformas.md`: el compañero como
  capacidad presente, con la clave de la app y su revocación de una línea;
  `rumbo/structure.md` gana `mobile/` en el monorepo; `meltemi validate
  companero-movil` limpio y `meltemi verify` con los escenarios enlazados
  (meta: cero marcas manuales); suite completa, clippy, fmt y `cargo deny`
  verdes; smoke en el emulador de los cuatro verbos; entrada en
  `docs/plan-de-cambios.md`; y la rama aterriza en `main` con `meltemi land
  companero-movil confirm`
