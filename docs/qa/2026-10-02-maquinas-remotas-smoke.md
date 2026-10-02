<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `maquinas-remotas`

**Fecha**: 2026-10-02 · **Plataforma**: Windows 11 Pro, WebView2
**Binarios**: `target/release/{meltemi-desktop,meltemid,meltemi}.exe`, el de
escritorio construido con `tauri build --no-bundle`.

**Dos daemons reales en dos endpoints** hacen de dos máquinas. Todo es
desechable: endpoints, datos y configuración propios. Sin `ssh`, sin red y sin
malla — lo que se mide es la superficie, y para ella un segundo endpoint local es
lo mismo que uno remoto.

## Método

Como el smoke anterior: `PrintWindow` con DPI por monitor fijada antes de
cualquier llamada de ventana, clics sintéticos y rueda con delta **con signo**.

**Gotcha nuevo, y costó dos intentos**: el clic sintético sobre la fila «Ajustes»
del pie **no entra** (sobre otras filas sí). No se investigó —no es de esta
change— y se rodeó por el camino determinista: la app persiste su última vista en
`desktop-ui.json`, así que se fijó `lastView: "settings"` y se relanzó. Vale la
pena anotarlo para el próximo smoke que necesite abrir Ajustes.

## Qué se midió, por el CLI

```
$ meltemi machines
* esta      \\.\pipe\meltemi-smoke-esta-16688
  la-torre  \\.\pipe\meltemi-smoke-torre-16688

`machines` lists. To switch, use `:machine <name>` in the terminal shell or the
Machines section of the desktop's Settings.
```

`--json` devuelve la lista estructurada con `active` por fila.

## Veredicto por escenario

| Escenario | Medido |
| --- | --- |
| Ajustes lista las máquinas con su endpoint | sección «Machines» con las dos, cada una con su endpoint, y la activa marcada con `● … (here)` |
| …y la activa no ofrece «Usar esta» | sólo `Test`; la otra trae `Test` y `Use this one` |
| Probar una máquina dice si respondió y cuánto tardó | `answered · 0.1.1 · 0 ms` bajo la fila probada |
| Probar no cambia la máquina activa | tras probar `la-torre`, la activa seguía siendo `esta` |
| Cambiar de máquina reconecta y lo dice | un clic en «Use this one» y la barra pasó de `on esta \\.\pipe\…-esta-…` a **`on la-torre \\.\pipe\…-torre-…`**, sin reiniciar la app; las marcas de las dos filas se invirtieron |
| Sin máquinas configuradas el cromo no cambia | con una sola máquina el segmento no aparece (`worthShowing`, con su test); aquí, con dos, sí |
| La presencia no se inventa | sin `[identity]`, ninguna fila trae presencia, y la sección Identidad dice que está apagada |

Capturas en `%TEMP%\meltemi-machines-smoke\` (no se publican: llevan rutas de
este equipo).

## El defecto que encontró

**La prueba de conexión informaba `unknown` como versión.** Leía
`answer["daemonVersion"]`, que es lo que responde `status`; `initialize` responde
`daemon: { name, version }`. El fallo era **invisible en los tests**: un campo
ausente cae al `unwrap_or("unknown")` y todo seguía verde. Corregido, y el e2e
pasó a exigir una versión **real** —no el literal `unknown` y empezando por
dígito—, que es la aserción que faltaba; comprobado que muerde volviendo a poner
el campo equivocado.

Es exactamente para lo que sirve un smoke: ningún test unitario iba a notar que
el valor correcto venía de otro campo, porque el fallback hacía que el código
pareciera funcionar.

## Lo que este smoke **no** midió

- **Una máquina remota de verdad**: los dos endpoints son locales. El canal
  `exec:ssh <host> meltemi bridge` tiene sus propios tests en
  `tui/tests/exec_endpoint.rs` y en el e2e de esta change; aquí se midió la
  superficie.
- **La guarda de trabajo sin guardar al cambiar**: no había editor abierto con
  cambios. Está cubierta por `scenarios_shell.rs`, que exige que el cambio pase
  por la misma guarda que salir del editor, por los dos extremos.
- **macOS y Linux**: corrida de Windows.
