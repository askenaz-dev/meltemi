# modelo-y-esfuerzo-por-sesion — design

> El proposal pidió que el design verificara tres cosas contra lo pineado en
> vez de citarlas de memoria: la vía ACP estándar, lo que Codex acepta y dónde,
> y lo que el CLI de Claude acepta. Las tres están verificadas abajo, y la
> tercera es la única que no se puede cerrar sin el binario del proveedor.

## Lo verificado, con su cita

**ACP 1.4.0 trae el vehículo, y es exactamente el que hacía falta**
(`agent-client-protocol-schema-1.4.0/src/v1/agent.rs`):

```rust
pub enum SessionConfigOptionCategory { Mode, Model, ModelConfig, ThoughtLevel, Other(String) }
pub enum SessionConfigKind { Select(..), Boolean(..) }
pub struct SessionConfigOption { id, name, description, category, kind, .. }
// NewSessionResponse.config_options : Option<Vec<SessionConfigOption>>   (:1102)
// y el cliente las fija con `session/set_config_option`.
```

Dos hechos que gobiernan el diseño entero:

1. **Las anuncia el agente**, no el cliente. `config_options` viaja en la
   respuesta de `session/new`. Meltemi no inventa listas de modelos: muestra lo
   que el agente dijo poder aceptar.
2. **Llegan DESPUÉS de que la sesión existe.** Es la razón por la que la vía
   estándar no puede ser la única (D2).

**Codex acepta las dos palancas, en dos sitios distintos**
(`core/mock-provider/schemas/codex-app-server/`): `ThreadStartParams` declara
`model`; **`effort` aparece solo en `TurnStartParams`**. No es un capricho de la
implementación: el esquema del proveedor dice que el modelo es del hilo y el
esfuerzo es del turno, y el adaptador lo respeta en vez de aplanarlo.

**Claude tiene el punto de inserción limpio** — `session_args()`
(`claude/wire.rs:75-88`) arma hoy siete banderas fijas y ninguna de modelo.

## D1 — El núcleo transporta, no traduce. Y eso decide el tipo

`model` y `effort` son **strings opacos** en el contrato, como `acp-args` y la
variable de contexto de auth: datos que el núcleo lleva y **no lee**. §5 no
admite un enum de modelos en `proto/` — sería el núcleo asumiendo un proveedor,
y se pudriría en silencio con cada modelo nuevo.

Consecuencia que hay que aceptar entera: **el núcleo no puede validar el
string**. Quien lo rechaza es el CLI del proveedor, y el rechazo se muestra con
su motivo. Un núcleo que validara sería un núcleo con una tabla de modelos
dentro.

## D2 — Dos vías, y la frontera es *cuándo*, no *quién*

| | vía | por qué |
|---|---|---|
| **Al arrancar** | argv del adaptador (`--model`, `model` del thread) | Las config options llegan con la respuesta de `session/new`: **no existen todavía** cuando hay que lanzar el proceso. El default del perfil tiene que aplicarse aquí o no aplica. |
| **A mitad de sesión** | `session/set_config_option` de ACP | Es la vía estándar (§6), la anuncia el agente, y no exige relanzar nada. |

No compiten: la primera es la única posible antes de que exista la sesión, la
segunda es la única honesta después. Y la segunda **solo se ofrece si el agente
anunció la opción** — ofrecer un selector que el agente no declaró sería
prometer en su nombre.

## D3 — Lo no soportado se rehúsa con diagnóstico, y por adaptador

Pedir `effort` a un agente cuyo protocolo no lo transporta **rehúsa**, con el
motivo y el nombre del agente. Jamás se ignora en silencio: una palanca de
cuotas que no hace nada y no lo dice es peor que no tenerla, porque el usuario
cree haber bajado el gasto.

La tabla la fija cada adaptador contra lo que su proveedor documenta, no el
núcleo:

- **Codex**: `model` al arrancar el hilo, `effort` **por turno** (su esquema lo
  define solo ahí). Ambos aceptados.
- **Claude**: `--model` en `session_args()`. **`effort` no se cablea**, porque
  no está verificado contra el CLI pineado y esta change no cita de memoria
  (D7). Se rehúsa con ese motivo exacto, y el día que se verifique es una línea.
- **Cualquier agente ACP de terceros**: lo que anuncie en `config_options`, y
  nada más.

## D4 — Perfil < sesión, y el perfil es la unidad que el usuario ya administra

Los perfiles ganan `model` y `effort` opcionales. «Perfil = agente + cuenta +
modelo» es lo que convierte «docs con el modelo barato» en una elección de una
vez en lugar de un ritual por sesión.

Precedencia declarada y en un solo sentido: **lo explícito de la sesión pisa el
default del perfil**, y un perfil sin campos no impone nada. No hay tercer
nivel: un default global sería una preferencia que se olvida encendida, que es
exactamente lo que `modos-de-autonomia` decidió no hacer con la autonomía.

## D5 — Lo que corrió queda escrito donde se consulta

`agent_resolved` gana `model` y `effort` **efectivos** — no los pedidos: los que
quedaron tras aplicar perfil y sesión. Hoy el modelo solo asoma por vías
laterales (el meta `providerModel` que el adaptador de Claude deja en sus
updates, el `usage_reported` de un nivel 3), nunca como dato de resolución.

Sin esto, la analítica local puede decir cuántos tokens gastó una sesión pero no
**con qué modelo**, que es justamente la pregunta que una palanca de cuotas
existe para responder.

## D6 — La ficha del picker muestra solo lo que Meltemi sabe de verdad

Tres fuentes, y ninguna inventada: lo que el agente anunció, lo declarado en
perfiles, y el consumo medido por la analítica local. Admite entrada libre,
porque el string es opaco también para la UI.

**Sin precios ni créditos.** Meltemi no tiene ni lo uno ni lo otro
(BYO-suscripción), y una tabla de precios embebida sería asumir proveedores (§5)
y pudrirse en silencio — el peor tipo de dato, el que parece autoridad.

## D7 — Lo que solo el binario del proveedor puede confirmar

Que el CLI de Claude acepte `--model <string>` en modo headless, y si expone
algo equivalente a esfuerzo, **no lo puede verificar CI** (§5: nunca agentes
reales). Por eso:

- se cablea `--model`, que es lo que el punto de inserción y la documentación
  del CLI sostienen;
- **no** se cablea esfuerzo para Claude: se rehúsa con el motivo, que es la
  única forma honesta de no saber;
- y la validación manual queda documentada con la versión probada, como en
  `preguntas-del-agente`.

## D8 — Alcance de verbos, y lo que se deja fuera

`session/start` y `worktree/dispatch` — los que arrancan una sesión. **No**
`propose` ni los verbos de autoría: llevan su propia postura de reglas y su
propio destino, y darles una palanca de cuotas por sesión invitaría a usar el
método SDD como banco de pruebas de modelos.

El cambio a mitad de sesión se ofrece **donde el agente anunció la opción**. Con
el aviso técnico —cambiar de modelo reinicia la caché del proveedor y puede
aumentar el costo— que es verdad, no retórica.

## D9 — Los adaptadores propios no anuncian nada, y eso es el hallazgo

> Añadida durante la implementación de 3.3. La tarea decía «los adaptadores
> anuncian sus opciones como session config options de ACP». Al ir a
> escribirlas, resultó que **no tienen ninguna que anunciar**.

Una `SessionConfigOption` de tipo `select` exige `current_value` **y** la lista
de valores seleccionables. Para anunciar un selector de modelo, el adaptador
tendría que saber qué modelos existen. Y no lo sabe:

- **Codex**: el esquema pineado (`core/mock-provider/schemas/codex-app-server/`)
  no tiene método alguno que enumere modelos. `InitializeResponse` trae
  exactamente un campo, `userAgent`. `ThreadStartParams` **acepta** un `model`
  pero no dice cuáles.
- **Claude**: el CLI acepta `--model <string>`; nada en lo verificado enumera.

Así que la única forma de anunciar sería **incrustar una lista de modelos en el
adaptador** — precisamente lo que D1 prohíbe («se pudriría en silencio con cada
modelo nuevo») y lo que D7 ya resolvió para el esfuerzo: lo no verificado se
rehúsa en vez de inventarse. Un selector con un solo valor —el actual— sería
peor que no anunciar: un control que no puede elegir.

**Entonces los adaptadores propios no anuncian.** No es una tarea pendiente
disfrazada: es la respuesta correcta con lo que los proveedores dan hoy, y se
revierte sola el día que uno publique un método de enumeración —serán las mismas
diez líneas que ya existen para traducir lo anunciado.

Lo que **no** se pierde por esto:

1. **La vía de arranque sigue siendo completa** (D2, columna «al arrancar»): el
   modelo y el esfuerzo viajan por argv, que es donde los proveedores los
   aceptan. Nada de lo que el usuario pide deja de llegar.
2. **La vía estándar existe y funciona de punta a punta**, ejercitada por el
   mock detrás de `--config-options` con las dos clases de ACP (`select` y
   `boolean`). No es código sin probar esperando un proveedor.
3. **Cualquier agente ACP de terceros que sí anuncie lo obtiene gratis**, que es
   justamente por qué §6 manda usar el estándar en vez de inventar un canal: la
   capacidad no depende de que la escribamos agente por agente.

Y la frontera queda escrita donde se lee: sin anuncio no hay evento
`config_options_announced` en el registro, la superficie no ofrece nada, y el
daemon rehúsa con 2007 si alguien lo intenta igual. Tres capas diciendo lo
mismo, porque la que importa —«Meltemi no promete en nombre del agente»— es una
promesa de seguridad, no de comodidad.
