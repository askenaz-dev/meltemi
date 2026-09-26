# Smoke conducido — `sesiones-en-la-barra`

**Fecha**: 2026-09-16 · **Plataforma**: Windows 11, WebView2 (Edg/153.0.4234.32)
**Binario**: `target/release/meltemi-desktop.exe` construido con
`tauri build --no-bundle`, sobre un repositorio fixture temporal y `mock-agent`.
Nunca contra este repositorio, nunca contra la red.

## Método

Puerto de depuración remoto **temporal** en `tauri.conf.json`, revertido al
terminar —el archivo quedó idéntico a `HEAD` y `git grep remote-debugging` solo
encuentra notas de QA— y el binario instrumentado **borrado**, no reconstruido:
no se publica desde `target/` y un binario con puerto abierto no debe quedar
por ahí. Se intentó primero por `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`, que
Tauri ignora porque fija sus propios argumentos de navegador; ya estaba escrito
en `2026-08-09-piel-de-pestanas-smoke.md` y se redescubrió por no leerlo antes.

El conductor CDP lee el DOM y los estilos computados, sintetiza eventos de
teclado reales y fotografía. Un proyecto, seis sesiones de `mock-agent` en un
directorio de datos aislado con su propio endpoint y su propia configuración.

**Gotcha confirmado**: el input sintético **no mueve los bindings de Svelte**. El
conductor escribe el borrador fijando el valor y despachando un `input` real,
que es lo que `bind:value` escucha.

## Resultado

| Escenario | Medido |
| --- | --- |
| Cuatro estados, cuatro cubetas en orden de señal | `❯ Listas para tu instrucción 1` · `■ Detenidas 6`, en ese orden |
| Cada cabecera lleva glifo, palabra y cuenta | 100 % de las cabeceras dibujadas |
| Una cubeta vacía no ocupa sitio | toda cubeta dibujada tiene cuenta > 0; las dos ausentes no se dibujan |
| Las detenidas no desbordan la barra | declara 6, dibuja 5, ofrece «Ver las 6» |
| La fila dice el título | «documenta el toggle», «prepara las pruebas del toggle», «revisa el toggle de modo oscuro»; sin título cae a `mock-agent 06ef3f8e` |
| Las suscripciones siguen distinguiéndose | pastillas `work` y `personal` en sus filas |
| Un cambio de estado salta sin animarse | **0** elementos animados en toda la barra (`animationName` ≠ none o `transitionDuration` > 0) |
| La lista es la vista, no una pestaña | 0 pestañas con la app recién abierta; el panel de la lista visible, **sin** `role=tabpanel` |
| Pedir una sesión nueva abre su pestaña y da el foco | `Ctrl+N` → 1 pestaña «Nueva sesión», seleccionada, `document.activeElement` es el `textarea` del compositor |
| Exactamente una pestaña en el orden de tabulación | 1 |
| Pedirla de nuevo enfoca, no duplica | sigue habiendo 1 pestaña |
| Enviar convierte la pestaña en la sesión | la pestaña pasa a «▸ revisa el contraste del tema oscuro»; **no** queda pestaña de compositor |
| La barra gobierna las pestañas | «Abiertas (1)», fila marcada `aria-current=true`, con control de cierre con nombre |
| La fila marca que está abierta | 1 marca `•` en la cubeta |
| El atajo conserva su afordancia | `kbd` con `Ctrl Enter` en el botón de envío |

Se fotografió la barra con sus dos cubetas, la sección «Abiertas» y la tira de
una sola pestaña, a 1440×900 lógicos. Como en los smokes anteriores, la captura
no se versiona: se entregó al mantenedor, y lo que prueba está en esta tabla.

## Lo que el smoke encontró

### 1. El nombre de la cubeta no cabía (corregido aquí, y re-medido)

A 216 px la cabecera leía **«LISTAS PARA TU INSTRU… 1»**. La causa no era la
frase sino el estilo: `.bucketName` había heredado el `text-transform:
uppercase` y el `letter-spacing: 0.04em` de `.sectionTitle`, y entre los dos se
comían la línea. Un nombre de cubeta es una frase **sobre las filas que
encabeza**, no el título de una región, así que las dos propiedades se quitan y
la cabecera gana un `title` con la frase entera. El mantenedor ya había señalado
etiquetas truncadas en el árbol una vez; esta habría sido la segunda.

| | antes | después |
| --- | --- | --- |
| «Listas para tu instrucción» | 176 px en un hueco de 153 → **truncada** | 131 px en 131 → entera |
| `text-transform` / `letter-spacing` | `uppercase` / `0.48px` | `none` / `normal` |
| `title` de la cabecera | ausente | la frase entera |

La primera re-medida **midió el binario viejo sin saberlo**: la reconstrucción
falló con «Access is denied» porque la app seguía abierta y tenía el `.exe`
tomado, y el código de salida quedó tapado por un `| tail`. El `dist/` del
frontend sí se había regenerado, así que el CSS parecía corregido y el binario
no. Se cerró la app, se reconstruyó leyendo el código de salida real, y se
volvió a medir. Queda anotado porque es exactamente la clase de falso verde que
un smoke existe para evitar.

Y la segunda corrida trajo una confirmación que la primera no podía dar: al
relanzar, la app **llegó directamente a la pestaña «Nueva sesión»**, porque la
primera corrida había dejado `lastView = home`. Es el escenario «Llegar es
llegar a la pestaña nueva», observado sobre el binario y no solo en el test.

### 2. La superficie nunca ve sus propias sesiones «trabajando» (no se corrige aquí)

**El hallazgo grande, y es anterior a esta change.**

Muestreando las cabeceras de cubeta 80 veces cada 100 ms alrededor de un turno
real, el conjunto observado fue **siempre** `Listas para tu instrucción |
Detenidas`. La cubeta **Trabajando nunca apareció**.

En la misma corrida el daemon respondió `queued` a `session/direct` y el
compositor mostró «encolada en la posición 1: el turno en curso sigue intacto y
se despachará al terminar». Es decir: **había un turno en vuelo, el daemon lo
sabía, y el listado de la superficie no**. Por eso tampoco apareció el par de
controles que esta change especifica —«Interrumpir y enviar» junto a
«Encolar»— y `Ctrl+Enter` encoló en vez de relevar.

La causa está a la vista una vez medida: `refreshSessions()` se llama al
conectar, al conmutar de proyecto y en el `finally` de un envío — **nunca
mientras un turno corre**. Y de ese listado sale el estado que leen tanto
`working` (el anillo ambiental de `compositor-que-trabaja`) como `relays` (el
par de esta change).

Conviene ser exacto sobre a quién pertenece esto. **No lo introdujo esta
change**: `compositor-que-trabaja` y `redirigir-turno` ya dependían del mismo
listado, con la misma latencia. Lo que esta change hizo fue ponerlo donde se ve,
porque el mantenedor pidió las sesiones «diferenciadas por en curso» y la cubeta
que lo diría está vacía por una razón que no tiene que ver con las cubetas.

El comportamiento observado es **seguro** —encolar es el resultado conservador, y
el texto del resultado dice la verdad—, pero la promesa «el compositor lo dice
antes» tiene una ventana en la que no la cumple. Se declara y se saca a su
propia change, en vez de colarse aquí: refrescar el listado durante un turno es
un cambio de conducta sobre un store compartido, toca la regla de que la barra
no anima (las filas saltarían más seguido), y merece decidir si lo correcto es
que el cliente repregunte o que el daemon empuje.

## Lo que este smoke no midió

- **El par con turno en vuelo**, por la razón de arriba: el estado que lo dibuja
  nunca llegó. La sonda que lo buscaba está en el hallazgo 2, no en la tabla.
- **El tope de ocho pestañas** y el diálogo de descarte del borrador: cubiertos
  por tests ejecutados, no por esta corrida.
- **La barra plegada a riel**: cubierta por el test de cableado sobre la regla
  CSS, no medida aquí.
