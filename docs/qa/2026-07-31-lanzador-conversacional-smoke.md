# QA — Smoke visual conducido de la GUI (2026-07-31, lanzador-conversacional)

Verificación de `lanzador-conversacional` **mirando la ventana**, según el método
que estableció `gui-acabado-y-cierre-sdd` (docs/qa/2026-07-26-gui-acabado-smoke.md)
y repitió `pulido-pre-anuncio` (docs/qa/2026-07-27-pulido-pre-anuncio-smoke.md).
Esta change es casi toda superficie —vista de llegada, navegación al enviar,
plegado de burbujas, tarjetas en línea, sección de proyectos— y los tests de
cableado leen fuentes: no ven layout, ni foco, ni lo que un texto se convierte al
volver del daemon. Este smoke sí, y encontró cuatro cosas que ninguno de ellos
podía ver (§Hallazgos).

## Método

- Binario de release construido con la CLI de Tauri (`ui/node_modules/.bin/tauri
  build --no-bundle`; **nunca `cargo build --release`**, que no activa
  `tauri/custom-protocol`), con `additionalBrowserArgs:
  --remote-debugging-port=9444` **temporal** en `tauri.conf.json`, revertido y el
  binario **reconstruido limpio** al terminar: el binario publicado no expone
  puerto alguno.
- Entorno aislado: endpoint propio (`\\.\pipe\meltemi-smoke-lanzador`),
  `MELTEMI_DATA_DIR` y `MELTEMI_CONFIG_DIR` propios en el scratchpad, y **tres
  repos fixture** (`harbour` con regla `allow`, `lighthouse` sin reglas para que
  el permiso escale, `spare` con `--load-session` para el estado reanudable). El
  único agente es `mock-agent`. Nunca este repositorio, nunca agentes reales,
  nunca red. El daemon del mantenedor siguió corriendo intacto: el smoke arrancó
  y detuvo **solo** sus dos procesos, por PID.
- Driver Node sobre el WebSocket de CDP: `Runtime.evaluate` midiendo
  `getBoundingClientRect()` y leyendo el DOM real, clicks reales sobre los nodos,
  `Page.captureScreenshot` para la mirada humana. Más un cliente JSON-RPC mínimo
  sobre el pipe para contrastar lo que la ventana muestra con lo que el daemon
  responde — dos fuentes, no una.
- Ventana 1200×800 lógicos; perfil nuevo (sin `desktop-ui.json`), que es también
  la prueba de la vista de llegada sin vista recordada.

## Resultado: lo que la change promete, medido sobre el binario

| # | Lo prometido | Medido |
|---|---|---|
| 1 | Home conversacional como vista de llegada, enfocado | `.home .composer textarea` presente y `document.activeElement === textarea` → **true**, con perfil nuevo |
| 2 | Compositor al centro, sin desbordar | escenario de 760 px dentro de `main` (216→1200): hueco izquierdo **112 px**, derecho **112 px**; `scrollWidth` 1200 = `clientWidth` 1200 |
| 3 | Contexto como chips dentro del compositor | «Proyecto harbour», «Agente o perfil el del proyecto», «Modo Libre» |
| 4 | El método se declara antes de enviar | `método session/start` junto al botón Enviar; el chip de modo lista los tres verbos: `session/start`, `propose`, `sdd/explore` |
| 5 | Los menús de chip no se salen ni tapan | los tres abren **hacia arriba** y `insideViewport` → **true** (cajas 279×95, 278×125, 318×95) |
| 6 | Perfiles ofrecidos por su nombre de suscripción | «MA mock-agent» + pill `work`, «MA mock-agent» + pill `personal`, junto al agente subyacente |
| 7 | Elegir otro proyecto no conmuta el ámbito | chip → `lighthouse`; el switcher del nav y el nodo actual siguen en `harbour` |
| 8 | Enviar navega hacia adentro | **524 ms** desde el click hasta la conversación abierta, con el turno todavía corriendo (se quedó minutos esperando un permiso) |
| 9 | Burbujas de turno | burbuja humana con la instrucción, burbuja de agente con «Filling in the proposal.», bajo el nombre del agente |
| 10 | Tarjeta de permiso en línea | tarjeta con `write-proposal` y opciones Allow/Reject, en su posición dentro del diálogo |
| 11 | La bandeja sigue siendo la vista completa | con la tarjeta visible, el contador de Permisos del nav marcaba **1** |
| 12 | Decidir desde la tarjeta resuelve la misma petición | tras pulsar Allow: «decidido por client · concedido», **0 controles accionables**, `resolved` |
| 13 | La instrucción encolada no se presenta como atendida | segunda burbuja humana marcada `pending` con pill «encolada, aún no enviada» |
| 14 | …y se despacha como siguiente turno | al decidir el permiso: 7 → 11 eventos, la burbuja pierde el `pending`, aparece un segundo turno de agente y su propia tarjeta |
| 15 | El conmutador no pierde nada | cabecera «14 eventos» ↔ log de operador con **14 líneas**; volver a la conversación deja los mismos ítems; el conteo está en pantalla, contable a mano |
| 16 | Estado honesto de sesión terminada | «sesión terminada y no reanudable: su registro se sigue leyendo», sin campo de envío y sin cancelar |
| 17 | …y de sesión reanudable | en el fixture con `--load-session`: «la sesión terminó: enviar la reanuda como sesión nueva enlazada», botón **Reanudar** |
| 18 | Sección Proyectos permanente con sesiones anidadas | «PROYECTOS» en el sidebar sin abrir modal alguno; 3 nodos con sus sesiones debajo, cada uno con «Nueva sesión en X» y «Olvidar X (solo del listado)» |
| 19 | «Abrir carpeta…» abre el diálogo **nativo** | ventana Win32 clase `#32770`, título «Select Folder», **768×480**, del proceso `meltemi-desktop` — no un modal del webview; cancelarla no dio de alta nada |
| 20 | El alta por contrato llega al nav sin reiniciar | `project/register` desde fuera de la app → el nodo aparece en el árbol |
| 21 | El texto de la baja dice lo que no hace | «No borra nada del disco: la carpeta queda intacta, sus sesiones se siguen listando, sus registros se siguen leyendo y la contabilidad local las sigue contando. Reaparecerá en cuanto se vuelva a usar o a dar de alta.» |

Nada estimado: cada cifra sale de `getBoundingClientRect()`, del texto renderizado
o de la respuesta del daemon por el pipe.

## Hallazgos

Cuatro cosas que los tests de cableado no podían ver. Ninguna se arregló aquí: el
rumbo de estructura dice que lo que surge se anota como propuesta futura y no se
cuela en la change activa. Van con su evidencia y su ubicación exacta.

### H1 — El texto no ASCII llega corrupto al agente (grave, previo a esta change)

Se escribe `acción íntegra ñandú` (20 caracteres) y el registro guarda
`acciÃ³n Ã­ntegra Ã±andÃº` (24). No es un problema de pintado: **la burbuja
muestra la corrupción porque el prompt ya está corrupto**, y ese prompt es el que
recibe el agente.

Aislado hasta el fondo, sin GUI y sin shell de por medio: un cliente JSON-RPC
mínimo sobre el pipe manda `session/start` con la cadena correcta y `session/log`
la devuelve doble-codificada. La causa está en
`core/meltemid/src/repo_map.rs:120`, dentro de `expand_refs`:

```rust
out.push(bytes[i] as char);   // un byte UTF-8 → un code point Latin-1
```

Cada byte de un carácter multibyte se convierte en su propio carácter. Afecta a
**todo** prompt que pasa por la expansión de `@` — `propose`, los verbos del
método, la sesión libre y la dirección —, es decir a cualquier usuario que
escriba en español, francés, alemán o con emoji. Es anterior a esta change
(`gestion-contexto-repo`), y esta change es la que lo vuelve visible: ahora el
compositor es la puerta principal y el transcript le devuelve al usuario su
propia frase.

La corrección es de tres líneas —empujar el carácter completo en la frontera y
avanzar `len_utf8()`— y merece su change, su escenario y su test: el escenario que
falta es exactamente «una instrucción con acentos llega idéntica al agente».

### H2 — Olvidar un proyecto con sesiones no lo quita del árbol

Pulsar «Olvidar lighthouse», confirmar, y el nodo **sigue ahí**. El daemon hizo
su parte: `project/list` pasó de tres proyectos a dos y sus sesiones siguen
listándose (medido por el pipe en el mismo instante). Lo que ocurre es del
cliente: `desktop/ui/src/lib/tree.ts:81` crea un nodo **inferido** para toda
sesión cuya raíz no case con un proyecto registrado, y el sidebar no distingue
los inferidos de los registrados. El usuario ve un aviso de éxito y ningún
cambio.

No se arregla aquí porque la respuesta no es obvia y es de producto: esconder el
nodo deja sesiones sin casa en el árbol —lo contrario de lo que el nodo inferido
existe para evitar (`multiproyecto-suscripciones`)—. Las salidas plausibles son
marcar el nodo inferido como tal (glifo y palabra, como el proyecto ausente) o
que el olvido sea explícito sobre la vista. Decisión del mantenedor.

### H3 — Con un permiso pendiente, la posición en la cola no se muestra

La línea de estado del compositor tiene un solo hueco y su cadena de `{:else if}`
pone el permiso por delante: mientras la sesión espera una decisión se lee «la
sesión espera tu decisión sobre permission» y **no** «encolada (posición 1)». La
promesa no se rompe —la instrucción encolada se ve en el transcript como burbuja
pendiente, con su pill, y jamás como atendida—, pero la posición, que es el dato
que la spec nombra, no está en pantalla en ese caso. Añadido menor: la palabra
que se muestra es `permission` y no el nombre de la herramienta esperada.

### H4 — Tras una tarjeta de permiso, el cierre del turno cae como línea neutra

El plegado cierra el turno del agente al llegar una tarjeta (para que la tarjeta
quede en su posición), así que el `turn_completed` posterior no encuentra turno
abierto y se renderiza como línea de sistema: `■ turn_completed
{"stopReason":"completed"}`. El motivo se ve —nada se esconde, que es lo que la
spec exige—, pero no cierra la burbuja. Debajo hay algo más de fondo: el daemon
emite `TurnCompleted` **una vez por sesión**, desde `session_finalize`, de modo
que los turnos intermedios de una sesión con instrucciones encoladas no tienen
evento de cierre en absoluto. El plegado es fiel al registro; el registro es el
que no marca cada turno.

### H5 — El chip de agente puede mostrar el catálogo de la máquina, no el del proyecto

Con el registro de flota declarado **en el proyecto**, el chip listaba los
agentes reales de la máquina en vez del agente del fixture. El daemon no tiene la
culpa, y se comprobó preguntándoselo: `fleet/list` con `projectRoot` devuelve
`["mock-agent","work","personal"]` y sin él devuelve el catálogo embebido de diez
entradas. Lo que pasa es que el primer `fleet/list` del cliente sale **antes** de
que el ámbito de proyecto se resuelva (`Home.svelte` lo pide en su `onMount`;
`App.svelte` resuelve el proyecto tras dos `await`) y nada vuelve a pedirlo. El
smoke siguió con el registro declarado a nivel de usuario, donde el chip muestra
lo correcto (fila 6 de la tabla).

## Deuda declarada

Igual que en los dos precedentes: el smoke es manual y por release. Convertirlo
en gate de CI —arrancar la app, recorrer las vistas, afirmar las invariantes—
sigue fuera de alcance y sigue apuntado. Las capturas de esta corrida
(home, chip de modo, conversación, log de operador) quedaron en el registro de la
evaluación; no se publican porque contienen rutas del sistema de archivos de la
máquina donde se corrió.
