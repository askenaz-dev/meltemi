<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — senales-de-vida (2026-09-26)

Medición sobre el **binario de release del taller**: la GUI conducida por CDP,
la ventana observada desde Win32 y la TUI dentro de una pseudoconsola propia.
Cubre lo que la change no puede demostrar desde un test de código: cómo se ve y
cuándo aparece cada movimiento, y cuánto escribe la terminal en reposo.

## Montaje

- Binarios: `cargo build --release -p meltemi -p meltemid -p mock-agent` y
  `tauri build --no-bundle` con una configuración **superpuesta por
  `--config`** desde el scratchpad, que solo añade el puerto de depuración a
  `additionalBrowserArgs`. El `desktop/tauri.conf.json` del repositorio **no se
  tocó**: mejora sobre la receta anterior de parchear y revertir, porque no hay
  nada que olvidar revertir.
- Aislamiento: las cinco variables (`MELTEMI_ENDPOINT` propio,
  `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR`, `MELTEMI_DAEMON_BIN`,
  `MELTEMI_START_HERE=1`) sobre un directorio temporal, y un
  `WEBVIEW2_USER_DATA_FOLDER` propio y nuevo. Fixture `harbour` (repo git de
  cuatro archivos) con `registry.toml` hacia `mock-agent --turn-delay-ms 6000`
  —el turno se sostiene seis segundos en silencio, que es el estado a medir— y
  un `permissions.toml` permisivo.
- Pantalla 4K al 250 %: la geometría persistida de 3200×2000 físicos es el
  viewport lógico de 1280×800 para el que está diseñada la superficie.

## La ventana hasta que la interfaz existe (D6)

Un observador Win32 recorre las ventanas del proceso cada 2 ms desde el
lanzamiento, anota cuándo existe la ventana y cuándo se hace visible, y la
captura con `PrintWindow` en ese instante, a los +100 ms y a los +500 ms.

**Defecto encontrado y corregido (tarea 5.3).** En la primera medición la
ventana nacía oculta como debía (handle a los 20–76 ms, `visible=False`), pero
se hacía visible con el **cuerpo en blanco**:

| arranque | visible a los | al hacerse visible | +100 ms | +500 ms |
| --- | --- | --- | --- | --- |
| frío | 1016 ms | blanco | blanco | interfaz |
| tibio ×3 | 827–869 ms | blanco | blanco o interfaz | interfaz |

Los tiempos de navegación y de recursos del propio documento explican por qué,
en una corrida con el instante de lanzamiento anotado:

| hito | desde el lanzamiento |
| --- | --- |
| ventana visible | 717 ms |
| inicio de la navegación | 579 ms |
| `DOMContentLoaded` | 747 ms |
| IPC `first_frame` | 771–784 ms |
| primer pintado (`first-paint`) | 1019 ms |

Es decir: **el tope gana** —la ventana se mostró antes de la señal e incluso
antes de `DOMContentLoaded`— y, aunque la señal hubiera llegado antes, el
primer pintado ocurre ~250 ms después del montaje. Lo que el webview enseña
hasta entonces es su fondo propio, blanco, que en el tema oscuro es exactamente
el destello que D6 existe para evitar.

La corrección: desde el `setup`, antes de que nada pueda mostrarla, la ventana
lleva el `--bg` del tema elegido en el estado persistido (o el del sistema sin
elección), con los dos colores pineados contra `app.css` por un test.
Re-medido sobre el binario corregido:

| arranque | tema | visible a los | centro al hacerse visible | +100 ms | +500 ms |
| --- | --- | --- | --- | --- | --- |
| frío | oscuro | 834 ms | `#0b1220` (`--bg`) | `#0b1220` | interfaz |
| tibio ×2 | oscuro | 845–846 ms | `#0b1220` | interfaz | interfaz |
| tibio | claro | 698 ms | `#f8fafc` (`--bg`) | interfaz | interfaz |

**Declarado, no medido o no logrado:** en esta máquina el tope gana siempre,
así que el primer frame visible es el fondo del tema y la interfaz llega 100–
500 ms después; lo que se garantiza es que ningún instante enseña un fondo
ajeno al tema, no que el primer frame visible sea ya la interfaz. No se midió
una ventana restaurada maximizada (R3), ni WebKitGTK ni WKWebView.

## La marca, una vez (D7)

Muestreo por `requestAnimationFrame` desde el primer frame de un documento
recargado con la bienvenida sin ver:

| | `stroke-dashoffset` | animación |
| --- | --- | --- |
| primera muestra (162 ms) | `1px` — sin dibujar | `mark-draw 0.6s` |
| a los 260 ms | `0.59px` — dibujándose | |
| última (2,5 s) | `0px` — dibujado | |
| con movimiento reducido (emulado por CDP) | `0px` desde la primera muestra | `none` |

## La llegada en la paleta (clase B)

Al abrirla con Ctrl+K: `arrive 0.12s`, opacidad `0.82` a los 40 ms. Con
movimiento reducido: `none`, opacidad `1`.

## Esqueletos (D3)

Una carga lenta se provocó **reteniendo cada petición IPC al daemon** con el
dominio `Fetch` de CDP: un daemon lento tal como lo ve la vista. No se inyectó
DOM. (El primer intento, envolver `__TAURI_INTERNALS__.invoke`, no tuvo efecto:
la propiedad no es escribible ni configurable, y el driver lo había dado por
bueno comparando contra una copia; se corrigió el método.)

| vista | retención | sin esqueleto hasta | esqueleto a los | retirado a los |
| --- | --- | --- | --- | --- |
| Consumo | 700 ms | 152 ms | 196 ms | 737 ms |
| Consumo, movimiento reducido | 700 ms | 140 ms | 189 ms | 748 ms |
| Proyecto | 100 ms por petición | 145 ms | 193 ms | 661 ms |

Lo medido en el esqueleto visible: 6 filas de 32 px, `role="status"`,
`aria-busy="true"`, `skeleton-pulse 1.2s infinite` con la opacidad a mitad de
pulso, `background-image: none` (ningún gradiente) y «cargando…» presente pero
fuera de la vista (1 px). Con movimiento reducido: `animation: none`, opacidad
`1` y la palabra visible (`position: static`, 1032 px de ancho). Proyecto
encadena seis peticiones, así que 100 ms por petición ya supera el umbral.

Cargas **sin retención** de Proyecto, Consumo y Flota: **ningún** esqueleto
montado en 1,5 s (un `MutationObserver` sobre el documento) — la carga rápida no
parpadea.

## La señal de vida del turno (D4)

Una instrucción escrita en el compositor y enviada con Ctrl+Enter al agente
simulado, que sostiene el turno seis segundos en silencio:

- La línea de espera, muestreada cada ~40 ms: «◔ trabajando… 0:00» → «0:05»,
  un paso por segundo, con el tiempo en `aria-hidden="true"`. **Ninguna**
  muestra dijo «sin salida» mientras el agente trabajaba.
- Tras la tarjeta de permiso —decidida al instante por la regla del fixture—
  la línea vuelve, porque el agente trabaja otra vez sin haber dicho nada
  nuevo, y se retira al cerrarse el turno.
- El tiempo de la sesión en la cabecera avanzó de `7s` a `16s` con
  `font-variant-numeric: tabular-nums`.
- El anillo del compositor estuvo encendido en las 118 muestras con la línea de
  espera.
- La insignia pasó de «▸ activa» a «❯ esperando instrucción» con su glifo en
  `arrive 0.16s` (clase C); los turnos llegan con `arrive 0.12s` y la tarjeta
  de permiso con `animation-name: none`.
- **El cursor ▍ no se observó.** El agente simulado emite su único fragmento,
  pide el permiso y cierra el turno en el mismo instante: el estado «fluyendo»
  dura menos que un muestreo. Lo cubren el caso ejecutado de
  `turn-signal.test.ts` y la guardia de fuente; verlo en pantalla pide un
  agente que transmita por fragmentos, y queda declarado.

## El pulso de la TUI (D5)

La TUI corrió dentro de una **pseudoconsola (ConPTY) de 120×30 creada por el
script**, sin ventana: una consola nueva se delega a la terminal predeterminada
del sistema, y abrir una pestaña en la terminal del usuario no era aceptable.
Se registró cada escritura con su instante.

| fase | duración | escrituras | bytes | frames del spinner |
| --- | --- | --- | --- | --- |
| reposo | 10 s | 6 | 19 | 0 |
| reposo tras la sesión | 10 s | 5 | 17 | 0 |

- En reposo la TUI escribe unos 2 bytes por segundo: el contador de actividad
  del daemon en la cabecera, que redibuja el sondeo de 2 s de la conexión. El
  pulso **no añade nada**.
- Mientras la sesión trabajó, la cabecera dijo «⠙ 1 trabajando» y el spinner
  escribió **80 frames en 7,9 s**, con una mediana de 94 ms entre escrituras
  (79–109 ms): el intervalo de ~100 ms.
- El pulso se detuvo en la misma escritura que dibujó la sesión como
  «finalizada».
- El tiempo del detalle no se miró aquí (la corrida no entró al detalle); lo
  cubren los tests de render con el reloj fijado.
- El tiempo de CPU del proceso quedó en 0–31 ms por ventana, dentro de la
  resolución del contador de Windows (15,6 ms): esas cifras son ruido y la
  medida son los bytes.
- **Declarado:** el parpadeo visible en conhost o Windows Terminal no se midió
  —la pseudoconsola registra bytes, no píxeles— y queda para una mirada humana.

## Hallazgos fuera de alcance, cada uno con su tarea

- La entrada «Nueva sesión» de la barra lateral abre **Ajustes** bajo el título
  «Nueva sesión»: `navigate("home")` no pasa por `openComposer`. El smoke abrió
  el compositor por la acción primaria de la cabecera.
- `agent_process.binary` registra el **último argumento** del comando («6000»)
  y no el programa.
- Una sesión libre espera **~6 s** entre `agent_resolved` y
  `checkpoint_created` en esta máquina —con el agente simulado y en las
  corridas reales conservadas de `corrida-integrada`—, mientras los mismos
  pasos de git a mano tardan ~80 ms cada uno.

## Incidente del método, para no repetirlo

Cada llamada a PowerShell de esta sesión es un proceso nuevo: las variables de
aislamiento **no persisten** entre llamadas. Un `meltemi stop` lanzado a mano
fuera del script, para cerrar un daemon del smoke, corrió sin ellas: arrancó un
daemon en el endpoint propio del usuario —no había ninguno corriendo— y lo
detuvo 36 ms después. Dejó cinco líneas en el log diario del usuario y nada
más: ni sesiones, ni configuración, ni datos. Desde entonces el aislamiento vive
en un archivo que cada llamada carga (`. env-10.ps1`), y los `stop` del smoke
bajan `MELTEMI_START_HERE` a `0` antes de enviarse.

## Reversión

Ningún archivo del repositorio se modificó para el smoke. Los procesos del
smoke (GUI, daemon, agente simulado, TUI) se detuvieron por su ruta y se
verificó que no quedó ninguno.
