<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — piel-de-pestanas (2026-08-09)

Medición sobre el **binario de release** con la GUI conducida por CDP, no sobre
las fuentes. Cubre las seis comprobaciones que la change escribió como tareas
para el smoke por no ser demostrables desde un test de código.

## Montaje

- Binario: `tauri build --no-bundle` con `CARGO_TARGET_DIR` a un directorio del
  scratchpad. **No se reutilizó `target/release`**: la GUI del mantenedor
  estaba abierta y el enlazador falla con «Access is denied» sobre su propio
  ejecutable. Construir aparte evita cerrarle la aplicación.
- Fixture aislado: repo git temporal (`harbour`) con su `registry.toml`
  apuntando al `mock-agent`, `permissions.toml` permisivo, y **su propio
  endpoint** (`\\.\pipe\meltemid-smoke-piel`) más `MELTEMI_DATA_DIR` y
  `MELTEMI_CONFIG_DIR` propios. El daemon del mantenedor **no se detuvo ni se
  consultó**: el aislamiento por endpoint sustituye al «matar meltemid antes de
  montar fixtures» de smokes anteriores.
- Seis sesiones del mismo agente, que es el caso que la piel debía sobrevivir.

## Hallazgo de método: dos condiciones, no una

El patch de `additionalBrowserArgs` en `desktop/tauri.conf.json` **sigue siendo
necesario** — se probó lo contrario y falló. Lo que este smoke añade es la
segunda condición, que faltaba y explica los arranques mudos:

- **`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` no sirve**: Tauri pasa sus propios
  argumentos de navegador y sobrescribe esa variable. Verificado leyendo la
  línea de comandos real del proceso `msedgewebview2`: sin el patch, no lleva
  `--remote-debugging-port` por más que la variable esté puesta.
- **WebView2 comparte el entorno del navegador entre instancias de la misma
  aplicación.** Con la GUI del mantenedor ya corriendo, una segunda instancia
  reutiliza su entorno y **descarta** sus argumentos, patch incluido. Por eso el
  puerto no abría al principio: el entorno lo había creado su app, con el suyo
  (9460). La condición que faltaba es un **`WEBVIEW2_USER_DATA_FOLDER` propio**,
  y además **nuevo**: una carpeta ya usada por una instancia anterior conserva
  su proceso de navegador y repite el problema.

La receta completa, entonces: patch de puerto en la config **+** user data
folder propio y nuevo **+** revertir el patch al terminar. El historial
documenta lo que cuesta olvidar ese último paso — *«take back a port I shipped
by accident»*.

## Lo que la change pedía medir

### 2.1b — El contraste entre la capa de la tira y el panel

**Falló, y la palanca declarada se aplicó.** Medido en el tema oscuro:

| Superficie | Color | |
|---|---|---|
| Capa de la tira | `rgb(26, 37, 64)` | `--surface-2` |
| Pestaña activa | `rgb(17, 26, 46)` | `--surface` |
| **Ratio** | **1.142 : 1** | invisible |

El design D1 había anticipado exactamente esto («están a un paso de tono… si no
separan, la palanca es un hairline») y nombrado la salida por adelantado. Un
1.14:1 no lo registra el ojo: la anatomía no podía descansar en la capa.

**Corrección aplicada**: la línea de base que la pestaña activa rompe — las
inactivas colorean su borde inferior con `--border`, la activa lo deja
transparente y se une al panel. El borde se **reserva** en todas
(`border-bottom: 1px solid transparent`), así que colorearlo no cambia la
altura de nada. Es un hairline, no un color inventado fuera de la paleta.

Re-medido tras la corrección, en los dos temas:

| | oscuro | claro |
|---|---|---|
| línea de las inactivas | `rgb(44, 58, 87)` (`--border`) | `rgb(203, 213, 225)` |
| línea contra la tira | 1.338 : 1 | 1.321 : 1 |
| línea de la activa | transparente | transparente |

Los ratios de la línea siguen siendo modestos, y se dicen tal cual: es el
**mismo token con el que toda la superficie separa** (tablas, paneles,
cajones). Si `--border` no bastara aquí, el problema sería del design system
entero y no de esta tira; no se le inventa una excepción local.

### Defecto encontrado y corregido: la costura izquierda no se pintaba

El hallazgo que justifica conducir el binario. Medido el `::before` de la
pestaña activa: **`background-image: none`** — es decir, **el pie izquierdo de
la costura no existía**, mientras el derecho (`::after`) sí.

La causa, una vez vista, es evidente y ningún test de fuente podía verla: las
reglas que apagan el separador entre inactivas usan el **mismo
pseudo-elemento** que la costura, tienen **la misma especificidad** y van
**después** en la hoja. Cuando la pestaña activa es la primera de la fila o
sigue a una etiqueta de grupo, `.tab:first-child::before` y
`.groupTag + .tab::before` la alcanzaban y le ponían el fondo transparente.

Corregido añadiendo `:not(.active)` a los cinco selectores apagadores, con la
razón escrita junto a ellos y una aserción que ahora los exige así.
Re-medido: la costura aparece en **los dos temas** y en **los dos pies**.

### 2.2b — La rama `forced-colors`

**Confirmada** en la hoja que la página realmente carga:
`.tab.active { border-color: highlight; }`. Convive con las otras reglas de
`forced-colors` de la superficie (marca, wordmark, píldoras).

### 3.1b — Hover contra activa

El riesgo era que compartieran relleno y se confundieran. **La línea de base lo
resuelve de paso**: una pestaña bajo el puntero toma la superficie del panel
pero **conserva su línea inferior**; solo la activa la pierde. La distinción ya
no depende del relleno.

### 3.2b — Revelar el cierre no mueve nada

**Confirmado, medido en píxeles.** Con el control oculto y revelado:

| | ancho de la pestaña | ancho del rótulo | izquierda de la vecina |
|---|---|---|---|
| oculto (`visibility: hidden`) | 134.725 | 87.575 | 474.325 |
| revelado | 134.725 | 87.575 | 474.325 |

Cero desplazamiento, cero recorte.

### 3.3b — El rótulo recupera ancho

**Confirmado** con seis pestañas del mismo agente. Cada pestaña muestra
`■ mock a296f430` —glifo, sin la palabra— y **dice** `mock a296f430 — ended`
como nombre accesible; el emergente lleva la historia completa
(`mock · ended · <uuid> · <proyecto>`). Los rótulos miden 87–91 px y **ninguno
queda truncado**.

### 3.4b — Los tonos de grupo

**Confirmado**, con el grupo creado por la vía real (menú de la pestaña →
nombre → crear; el texto se escribió con `Input.insertText` de CDP, porque un
evento sintético no mueve un binding de Svelte):

- Banda de la pestaña miembro: `rgb(74, 222, 128)` — el token `--ok`, no un
  color local.
- Ancho **reservado en todas** las pestañas: `2.8px` (los 3 px del módulo
  escalados por el DPI de la pantalla). Entrar a un grupo no desplaza nada.
- Etiqueta del grupo: `Revision`.
- Y el color no es el único portador: el nombre accesible de la pestaña miembro
  quedó **`mock 9a4d524f — ended — Revision`** — rótulo, estado y grupo, los
  tres en palabras.

Los cuatro tonos comparten regla y token; se ejercitó el que el grupo nuevo
recibe. El resto queda cubierto por el test de fuente, que exige la regla de
los cuatro.

### Tema claro

Todas las medidas se repitieron con `data-theme="light"`: la anatomía se
comporta igual (activa sin línea de base, costura presente en los dos pies,
banda reservada), con los valores de la paleta clara ya tabulados arriba.

## Hallazgo ajeno a esta change, anotado y no colado

**El CLI guarda la raíz del proyecto tal como se la escriben.** `meltemi session
"…" .` deja sesiones cuyo `projectRoot` es el literal `.`, mientras
`projects` sí canonicaliza y muestra la ruta absoluta. En la GUI eso produce dos
nodos: el proyecto real con **0 sesiones** y un nodo inferido llamado `.` con
todas. Con ruta absoluta el árbol es correcto. Es la misma clase de defecto que
la tanda del 2026-07-31 corrigió por otras puertas
(«Spell a project's root the same way whichever door it comes through»), y esta
es una puerta que quedó sin cerrar. No se arregla aquí porque no es de esta
change.
