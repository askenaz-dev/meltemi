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
- [ ] 1.2 `mobile/` (crate nuevo) con `russh` pineado: diálogo SSH hasta un
  canal de sesión que ejecuta el comando puente, expuesto como el mismo
  `Stream` que el resto del cliente ya consume, con el host key fijado en el
  primer uso y comprobado después (design D2) — escenarios «El canal del
  teléfono termina en el socket local», «El puente es el mismo» y «La clave del
  host cambia» — gates: suite del crate
- [ ] 1.3 `mobile/`: el par de claves de la app —generado en el teléfono,
  pública mostrada para autorizar, privada cifrada en reposo con material del
  almacén del sistema—, con la frontera escrita en el código: ni se leen ni se
  piden claves ajenas (design D3) — escenario «La clave es de la app y de nadie
  más» — gates: suite del crate

## 2. La superficie

- [ ] 2.1 `desktop/ui`: segunda entrada `mobile.html` + `Mobile.svelte` que
  reutiliza stores, catálogo ES/EN, i18n y los módulos de decisión ya probados,
  con navegación inferior de cuatro destinos y objetivos de 44 px; la pantalla
  sin canal dice qué falta y cómo se arregla (design D1, D6) — escenario «Sin
  canal» — gates: `npm run check`, lint de i18n, suite de UI
- [ ] 2.2 Monitorear y dirigir: sesiones por estado con `session/list` y
  `session/state`, y el compositor mínimo con `session/direct` y
  `session/cancel` (design D5) — gates: suite de UI
- [ ] 2.3 Aprobar y revisar: la petición que espera con sus opciones
  (`permission/pending`, `permission/decide`, `permission/changed`), y la
  revisión de una carrera —diffs, gates, checklist, adoptar o revertir **con
  confirmación**, sin editar— (design D5) — escenarios «Aprobación de una
  petición de permiso desde el móvil», «Adopción de un archivo de un competidor
  desde el móvil» y «Propuesta de autoría en el móvil» (vigentes) — gates:
  suite de UI
- [ ] 2.4 Suspensión y vuelta: nada sostiene el socket en segundo plano, y al
  volver al frente se reconecta y se reconcilia antes de mostrar nada como
  actual (design D4) — escenarios «La app vuelve al frente» y «La app está en
  segundo plano» — gates: suite de UI

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
