# preguntas-del-agente — design

> El proposal avisó de que esta change relitiga dos decisiones escritas del
> adaptador, y por eso el design es obligatorio. La primera resulta que **no hay
> que relitigarla**: leída con cuidado, la spec viva ya deja sitio. La segunda sí
> es una excepción de verdad y se escribe como tal.

## D1 — El rehúso no contradice la spec: su condición deja de cumplirse

La spec viva dice, literalmente:

> **WHERE** la superficie del proveedor **no puede** relevar una interacción
> (herramientas solo interactivas), la denegación automática MUST mostrarse en la
> sesión con su motivo y el adaptador MUST NOT aprobarla por su cuenta.

Es un requisito **condicional**, y su condición es «no puede relevarse». No dice
«`AskUserQuestion` se deniega»: dice qué hacer cuando no hay dónde preguntar. La
premisa que el código escribió —«no hay interfaz aquí en la que preguntar»— era
verdad cuando se escribió y **ha caducado**: Meltemi es esa interfaz, y ya pinta
opciones como botones sobre la misma cola de permisos.

Así que el delta es **ADDED, no MODIFIED**. El requisito vigente sobrevive
intacto y sigue gobernando toda herramienta que de verdad no pueda relevarse; lo
que cambia es que `AskUserQuestion` deja de estar en ese conjunto. El escenario
«Interacción no relevable denegada con motivo visible» sigue siendo cierto y no
se toca — y para que no quede vacío, la lista `interactive_only` **se conserva
como mecanismo**, con su motivo, para el siguiente caso que sí lo sea.

Esto importa más que el ahorro de ceremonia: un MODIFIED aquí habría reescrito un
requisito de seguridad para hacer sitio a una feature, que es exactamente el
movimiento que la constitución §1 existe para hacer visible.

## D2 — La excepción al input intacto, escrita con su fundamento

El principio vive en el código y en dos tests, no en un requisito:

```rust
Decision::Selected(option) if option == ALLOW_ONCE => json!({
    "behavior": "allow",
    // Unchanged, always: the human approved what they were shown.
    "updatedInput": input,
}),
```
`permission.rs:127-131`, pineado por `no_decision_is_a_denial_and_never_a_shrug`
(«an allowed call runs exactly as it was approved») y por el test del shim.

La excepción tiene fundamento y no es una comodidad: **en `AskUserQuestion` el
input *es* el formulario**. El humano no reescribe lo que el agente iba a hacer;
completa lo que el agente vino a preguntar. La diferencia se puede afirmar en
código y no como intención: la excepción se aplica **solo** cuando el `tool` es
`AskUserQuestion`, y **solo** rellenando el campo de respuesta que el propio
input declara; cualquier otra herramienta sigue viajando byte a byte.

Se escribe como requisito nuevo —no como comentario— precisamente porque es una
excepción a una regla de seguridad, y una excepción que solo vive en un comentario
es una que la siguiente refactorización borra sin enterarse. Y los dos tests
vigentes **se enmiendan por adición**, no por debilitamiento: siguen exigiendo
input intacto para toda herramienta, y ganan su gemelo para la que no.

## D3 — Multi-selección: se descompone, y si no se puede, se dice

ACP devuelve **una** opción o `Cancelled`. `multiSelect` no cabe en un desenlace.

Dos salidas honestas, y se elige la primera: una petición **por pregunta** —el
input de `AskUserQuestion` ya trae una lista de preguntas, así que secuenciarlas
es leer lo que hay, no inventar—. Para una pregunta que además pide varias
respuestas, la descomposición no alcanza, y ahí la respuesta es la segunda
salida: **se releva igual, con sus opciones, y se responde una**; el rótulo dice
que el canal admite una sola. Fingir multi-selección sobre un cable que no la
transporta sería peor que la limitación.

## D4 — El compositor es un acceso más a la misma cola, jamás una segunda

Con la sesión en `waiting_permission`, la zona del compositor presenta la
pregunta y sus opciones. Decide por **`permission/decide`**, el mismo verbo que
la bandeja y que la tarjeta del transcript (`SessionDetail.svelte:427-439`, con
su comentario ya escrito: «the conversation is another view of the same queue,
never a second queue»).

La tarjeta del transcript **se queda**. El log es la verdad y la tarjeta es su
lectura; el compositor es el control vivo. Quitarla dejaría la historia sin la
pregunta en cuanto se contesta.

**Sin animación de layout**, y la regla es literal y vigente: «La bandeja de
permisos y los banners de señal MUST NOT animar su layout: nada se mueve bajo el
cursor mientras se decide un permiso» (`gui-shell/spec.md:266-268`). Aparece de
golpe.

**Tope visual y desbordamiento**: la lista tiene su propio scroll con una altura
máxima; **jamás el panel**. Un compositor que crece con el número de opciones
movería el transcript entero, que es la misma prohibición dicha de otra forma.

## D5 — «Otra respuesta…» dice la verdad de su cable, que no es la misma

La última opción abre texto libre en el mismo sitio. Lo que ocurre después
depende del protocolo, y las dos verdades se dicen en vez de disimularse:

- **Adaptador de Claude**: viaja en `updatedInput` (D2). Es una respuesta a la
  pregunta.
- **ACP nativo**: no hay campo por donde viaje. La pregunta se resuelve
  **cancelada** y el texto entra como **relevo del turno** —el verbo que
  `redirigir-turno` acaba de construir y probar—. No es lo mismo y no se rotula
  igual: el turno en vuelo se interrumpe.

El rótulo de la salida se decide por el agente de la sesión, no por una
preferencia global.

## D6 — El mock aprende a preguntar, detrás de una bandera apagada

Como `--honor-cancel` y `--cancel-turn` de `redirigir-turno`: **apagada por
defecto**, porque un mock que pregunta por su cuenta cambiaría lo que leen los
e2e vigentes de permisos. Con la bandera, emite una pregunta con opciones y una
recomendada en su rótulo, que es lo que el flujo necesita ejercitar sin red.

## D7 — Lo que solo una máquina con el CLI real puede confirmar

La forma exacta del `updatedInput` que el CLI de Claude espera para
`AskUserQuestion` **no la especificamos nosotros** y puede cambiar con su
versión. CI no corre agentes reales (§5), así que:

- la forma se escribe donde se pueda leer, con la versión probada anotada;
- el requisito de **conformidad por versión** que `own-adapters` ya tiene cubre
  el desfase: si la forma cambia, el adaptador rehúsa antes que adivinar;
- y la validación manual queda documentada como tal, no como verificada.
