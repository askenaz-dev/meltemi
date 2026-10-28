<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `primer-arranque-del-home`

**Fecha**: 2026-10-28 · **Plataforma**: Linux (contenedor), WebKitGTK 2.52.6
bajo Xvfb a 1440×900.
**Binarios**: `target/debug/{meltemi-desktop,meltemid,meltemi}`, el de
escritorio construido con `cargo build -p meltemi-desktop` y su interfaz
servida por el servidor de desarrollo de Vite en el puerto que declara
`tauri.conf.json` (`devUrl`), que es como corre `tauri dev`. Daemon, datos y
configuración **aislados** (endpoint, `MELTEMI_DATA_DIR` y `MELTEMI_CONFIG_DIR`
propios por pasada), sobre un repositorio fixture temporal. Nunca contra este
repositorio, nunca contra la red, nunca un agente real.

## Método

Dos fixtures de registro, por `MELTEMI_FLEET_REGISTRY`:

- **Flota vacía**: una sola entrada cuyo binario no existe en ningún sitio y
  que no viaja empaquetada. `meltemi fleet` contra ese daemon: `1 agent(s), 0
  detected`.
- **Dos lanzables**: dos entradas cuyos binarios son guiones vacíos en un
  directorio del `PATH` del daemon. `meltemi fleet`: `2 agent(s), 2 detected`.
  Nada se lanza: el chip solo lee la detección.

La bienvenida de primer uso se dio por vista creando su marca en el directorio
de datos, y el idioma se fijó en español en el estado de la interfaz, para que
la ventana abriera directamente en el compositor. Sin sintetizador de entrada
en el contenedor, el conductor no hace clics: abre la app, espera y fotografía
la pantalla con `import`. El reconocimiento se midió en dos ventanas seguidas
del mismo perfil.

## Resultado

| Escenario | Medido |
| --- | --- |
| La flota vacía se dice antes de fallar | la cara del chip dice **«sin agentes detectados»** entera, con el borde en el tono de advertencia; el estado no registra saludo |
| El reconocimiento se dice una vez | primera ventana: **«2 detectados»**, y el estado guarda `fleetGreeted: true`; segunda ventana del mismo perfil: **«el del proyecto»** |
| El menú vacío abre la flota | **no medido aquí**: pide un clic. Lo cubre el test de cableado (`scenarios_shell.rs`) |

Las capturas no se versionan, como en los smokes anteriores: lo que prueban
está en esta tabla.

## Lo que el smoke encontró

### 1. La advertencia estaba en las palabras y no en el tono (corregido aquí)

La primera captura con la flota vacía mostró el chip con el mismo aspecto que
los demás. El escenario pide decirlo «con tono de advertencia», y el chip solo
tomaba ese tono ante un rehúso del daemon: `tone={refusal ? "warn" : "plain"}`
desde que se escribió. El test de cableado comprobaba la etiqueta, no el tono.
Ahora el tono es de advertencia también con la flota vacía, y el test lo exige.

### 2. La advertencia se cortaba (corregido aquí)

La cara de un chip mide como mucho 26 caracteres, y «Agente o perfil sin
agentes detectados» no cabe: se leía **«sin agentes …»**, que es la mitad que no
dice nada. Un chip en tono de advertencia ya no se recorta. Los chips en tono
normal mantienen el recorte por diseño; con la flota detectada el chip lee «el
del proye…», que se anota y no se toca aquí.

### 3. El saludo volvía en cada ventana, y el interruptor de avisos no se guardaba (corregido aquí)

Tras la primera ventana con dos lanzables, el archivo de estado no tenía
`fleetGreeted`. La interfaz envía su estado entero y el lado Rust escribe solo
los campos que declara: `fleetGreeted` y `noticesEnabled` no estaban
declarados, así que cada guardado los tiraba. El saludo se decía en cada
ventana —una insignia, justo lo que el diseño D3 rechaza— y apagar los avisos de
escritorio (`avisos-de-escritorio`) nunca quedaba apagado al reabrir. Los dos
campos se declaran; `noticesEnabled` queda ausente hasta que el usuario decide,
para que un perfil anterior siga con los avisos encendidos. Un test nuevo lee la
interfaz de estado de la superficie y falla si el lado Rust no conserva alguno
de sus campos.

### 4. El id interno de la pestaña nueva se ve (no es de esta change)

Con el compositor abierto, la miga dice **«Registro de la sesión · \_\_new\_\_»** y
la sección «Abiertas» de la barra lista **«\_\_new\_\_»**: las dos cortan el id de
la pestaña a ocho caracteres sin reconocer la del compositor, que la tira de
pestañas sí nombra «Nueva sesión». Es lo primero que ve quien abre la app por
primera vez. Se nombra como change propia, `pestana-nueva-con-nombre`.

## Lo no medido

- El gesto del menú vacío hacia la vista de flota (pide un clic).
- Windows y macOS: esta corrida es de Linux. La que corre el mantenedor con
  `docs/prueba-de-uno.md` pasa por el primer arranque en Windows.
