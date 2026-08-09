# titulo-de-sesion

> Vía completa (proposal → design → specs → tasks). Los deltas son ADDED sobre
> `acp-session`, `gui-shell` y `tui-shell` más una propiedad opcional del
> contrato, pero cruzan daemon, proto y las dos superficies: el design debe
> decidir derivación, herencia en resume y truncado antes de escribir nada.

## Why

Una pestaña de Meltemi dice hoy `mock-agent bc514e22`. Con seis sesiones del
mismo agente, la tira es seis veces la misma palabra seguida de ocho
caracteres de hex: nadie recuerda qué trabajo vive detrás de `97040fb6`. El
mantenedor lo pidió con la referencia exacta: como ChatGPT o Claude, donde el
primer mensaje se convierte en el título de la conversación. Lo que importa,
en sus palabras, es «diferenciar qué agente y a qué pertenece».

La materia prima ya existe y es del daemon: `session/start` recibe la
instrucción, el log la persiste como `prompt_sent`, y `session/list` es la
vista que ambas superficies consumen. Lo que falta es que esa instrucción se
vuelva un **título** que viaje con la sesión — y hacerlo en el daemon, no en
un cliente, porque un título calculado en la GUI sería una feature de una
sola superficie (§4) y porque dos clientes no deben poder mostrar dos títulos
distintos para la misma sesión.

Sobre el «título que le da el agente»: ChatGPT y Claude lo generan con una
llamada barata a un modelo propio. Meltemi no tiene modelo propio (§5:
ninguna dependencia puede exigir cuenta de proveedor) y generarlo inyectando
un prompt oculto en la sesión de pago del usuario gastaría sus tokens en algo
que no pidió y pondría en su log palabras que no escribió — y el log es la
verdad. Así que la v1 **deriva en local** y la generación por modelo queda
declarada como futuro opt-in de `motor-propio-byok`, cuando exista un motor
con la clave del propio usuario.

## What Changes

- **El daemon deriva el título al iniciar la sesión**: primera línea de la
  instrucción, espacios colapsados, truncado honesto con elipsis a un tope
  fijado en el design (~64 caracteres). Determinista, local, sin red y sin
  modelo (§9).
- **`sessionInfo` gana `title` opcional** en `session/list` (y el evento
  `session_started` lo lleva, para que una pestaña recién abierta no espere
  al próximo refresh). Opcional porque las sesiones históricas anteriores a
  la change no lo tienen y el contrato no miente.
- **Herencia en resume**: una sesión reanudada continúa la misma conversación
  y conserva el título original (como ChatGPT); el design fija si un resume
  con instrucción muy distinta lo re-deriva o lo mantiene — decisión escrita,
  no accidente.
- **La GUI lo adopta donde nombra sesiones**: el rótulo de la pestaña pasa a
  ser `título`, con el avatar del agente (identidad por color estable que ya
  existe) como el «favicon» que diferencia agente; el hash corto baja al
  tooltip junto al id completo y el proyecto. La lista de sesiones y el
  encabezado del detalle muestran el título junto al id.
- **La ambigüedad de proyecto se resuelve sola**: cuando las pestañas
  abiertas cruzan más de un proyecto, el rótulo antepone el nombre del
  proyecto (`harbour · Corrige el login`); con un solo proyecto abierto, no
  gasta ancho en repetirlo. El tooltip lleva siempre la historia completa
  (requisito vigente: la tira nunca miente sobre dónde vive una sesión).
- **La TUI lo adopta en su lista de sesiones** con la misma verdad del
  contrato: paridad por construcción, no por promesa.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `acp-session`: + requisito «Título derivado de la sesión» — derivación
  local determinista al iniciar, presencia en `session/list` y en
  `session_started`, herencia en resume.
- `gui-shell`: + requisito «Las pestañas y listas nombran el trabajo» —
  título como rótulo, avatar como identidad de agente, proyecto antepuesto
  solo ante ambigüedad, id en tooltip.
- `tui-shell`: + requisito «La lista de sesiones muestra el título».

## Impact

- Archivos: `core/meltemid/src/session.rs` (derivación y registro),
  `proto/schemas/v1/session-list.schema.json` y `session-event.schema.json`
  (+ `meltemi-proto` y su test de conformidad), `tui/` (lista),
  `desktop/ui` (`SessionTabs`, `Sessions`, `SessionDetail`, `tree.ts`).
- Cero dependencias nuevas. La derivación es una función pura con sus tests
  (unicode, líneas vacías, instrucciones de una palabra, truncado).
- Las sesiones históricas sin título muestran lo de hoy (agente + hash):
  degradación honesta, sin migración de logs.
- Riesgo: bajo. Lo único con juicio es el truncado y la herencia en resume;
  por eso van al design y no a la improvisación.

## Fuera de alcance

- **Generación de título por modelo** (opt-in, clave del usuario): declarada
  como extensión futura de `motor-propio-byok`; esta change deja el campo del
  contrato listo para que esa mejora no lo mueva.
- **Renombrar títulos a mano** desde las superficies: exigiría RPC de
  escritura y decisiones de persistencia; change propia si la evidencia la
  pide.
- **Buscar sesiones por título**: la búsqueda vive en el palette y en las
  listas; adoptará el campo cuando exista, sin requisito aquí.
- **La piel de la tira**: es de `piel-de-pestanas`; esta change cambia qué
  dicen las pestañas, no cómo se visten.
