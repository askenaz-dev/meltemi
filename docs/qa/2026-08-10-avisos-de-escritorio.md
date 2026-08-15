<!-- SPDX-License-Identifier: Apache-2.0 -->
# Avisos de escritorio — medición y verificación manual (2026-08-10)

Dos cosas que la change pidió expresamente **no suponer**: cuánto pesa el
instalador con la dependencia dentro, y si el sistema entrega el aviso de
verdad. La primera se mide aquí. La segunda **no la puede aseverar CI**, que
corre headless en los tres sistemas, y queda como verificación manual con lo
que hay que mirar en cada uno.

## Presupuesto de instalador, con la dependencia dentro

Construido con bundle completo sobre este árbol, con
`tauri-plugin-notification` dentro:

| | bytes | |
|---|---|---|
| `Meltemi_0.1.1_x64_en-US.msi` | **4 374 528** | 4.17 MB |
| Presupuesto (`MELTEMI_GUI_INSTALLER_BUDGET_BYTES`) | 15 728 640 | 15.00 MB |
| Margen | 11 354 112 | 72 % del techo sin usar |

Referencia anterior sin la dependencia, del QA del 2026-07-25: **4 104 192**
bytes para la v0.1.0. **El coste del plugin es de unos 270 KB** — la diferencia
incluye además lo que la versión trajo por su cuenta, así que ese número es un
techo del coste, no su medida exacta.

El presupuesto codifica «no empaquetamos motor» (`gui-tauri-paridad` D7), y
sigue holgado: la dependencia no acerca la cifra a nada.

## Verificación manual por plataforma

Lo que CI no puede hacer, con el motivo de cada caso —los tres riesgos que la
propia propuesta enumeró y que el design D1 aceptó de frente:

### Windows

- **Cómo**: instalar desde el **MSI** y lanzar desde el menú de inicio. El
  toast de Windows exige **identidad de aplicación**, que da el instalador.
- **Qué debe verse**: con la ventana detrás, un permiso pendiente produce
  parpadeo en la barra de tareas **y** un aviso del sistema.
- **Lo declarado**: el ejecutable suelto (`meltemi-desktop.exe` fuera del MSI)
  **puede no mostrar aviso**. No es un defecto: es la identidad que Windows
  exige y que solo el instalador otorga.

### macOS

- **Cómo**: sobre el **bundle** (`.app` del DMG), no sobre el binario de
  desarrollo. Una app sin bundle firmado puede no mostrar avisos en desarrollo.
- **Qué debe verse**: rebote del dock **y** aviso; y que el permiso se pide en
  el primer momento real, no al abrir.

### Linux

- **Cómo**: con un daemon de notificaciones DBus presente (el de cualquier
  escritorio habitual).
- **Qué debe verse**: *urgency hint* en la barra **y** aviso.
- **Lo declarado**: sin ese servicio, la superficie debe quedar en **silencio
  declarado** —Ajustes dice por qué— y jamás en error fatal. La petición de
  atención sigue funcionando en ese caso, que es la razón por la que el plugin
  se sumó al mecanismo existente en vez de sustituirlo.

## Lo que sí está probado sin intervención

- La decisión de avisar (momento, foco, colapso de ráfaga, apagado) — módulo
  puro con ocho casos ejecutados.
- Que el módulo **no puede** filtrar texto del turno: no lo recibe.
- Que una denegación o un servicio ausente se registran con su motivo y
  **detienen el envío** en vez de darlo por hecho.
- La campana del terminal: apagada salvo que se pida, y solo en los tres
  momentos.
