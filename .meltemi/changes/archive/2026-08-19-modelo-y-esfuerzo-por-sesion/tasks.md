# modelo-y-esfuerzo-por-sesion — tasks

> Vía completa. El contrato y el rehúso primero, porque una palanca de cuotas
> que no hace nada y no lo dice es peor que no tenerla. Los adaptadores después,
> cada uno contra lo que su proveedor documenta. Las superficies al final.

## 1. El contrato y la precedencia

- [x] 1.1 `model` y `effort` opcionales, **strings opacos**, en `session/start`
  y `worktree/dispatch`, con la conformidad de tres vías y `gen:forms`
  commiteado; **no** en `propose` ni en los verbos de autoría (design D8) —
  escenario «El modelo pedido viaja sin interpretarse»
- [x] 1.2 Los perfiles ganan `model` y `effort` opcionales, y la precedencia va
  en un solo sentido: lo explícito de la sesión pisa el default del perfil
  (design D4) — escenarios «La sesión pisa el default del perfil» y «Un perfil
  sin declaración no impone nada»
- [x] 1.3 `agent_resolved` registra los valores **efectivos**, no los pedidos —
  sin eso la analítica sabe cuánto gastó una sesión pero no con qué (design D5)
  — escenario «Lo que rigió queda en el registro»

## 2. El rehúso, que es la mitad honesta de la palanca

- [x] 2.1 Pedir una palanca que el agente no admite **rehúsa con diagnóstico**
  que nombra al agente y la palanca (design D3) — escenarios «Una palanca que el
  agente no admite se rehúsa» y «Lo no verificado se rehúsa en vez de
  inventarse»
  <!-- 2026-08-17: el rehúso vive en el núcleo aunque §5 le prohíba entender los
  strings, y no es contradicción: el núcleo no sabe qué **significa** un modelo,
  pero sí sabe **si el binario que va a lanzar tiene un sitio documentado donde
  ponerlo**. Rehúsa antes de crear nada — rehusar después dejaría una sesión que
  nadie pidió. Y de paso salió que el catálogo del schema de errores había
  derivado: **2005 nunca se añadió**; esta lista de constantes es lo que lo
  notó al sumarse 2006. -->
- [x] 2.2 Un valor vacío se rehúsa en vez de viajar como si fuera una elección —
  escenario «Un valor vacío se rehúsa en vez de viajar»

## 3. Los adaptadores, cada uno contra su proveedor

- [x] 3.1 Codex: `model` al arrancar el hilo y `effort` **por turno**, que es
  donde su esquema pineado los define — verificado, no citado de memoria
  (design D3) — escenario «El adaptador manda la palanca donde su proveedor la
  acepta»
- [x] 3.2 Claude: `--model` en `session_args()`; **esfuerzo NO se cablea** y se
  rehúsa con ese motivo, porque no está verificado contra el CLI pineado
  (design D7)
  <!-- 2026-08-17: las palancas viajan al adaptador por el **entorno** que el
  daemon ya compone para él —mismo camino que su otra configuración, sin
  transporte nuevo—, porque nada enlaza los dos procesos en tiempo de
  compilación: `meltemi-adapters` es un crate hoja y meterle el contrato del
  cliente para compartir un string sería la dirección equivocada de dependencia.
  El nombre se escribe en los dos lados y **un test lee la fuente del otro** para
  probar que coinciden: un desacuerdo silencioso ahí sería un modelo elegido que
  nunca llega al CLI, sin que nada lo diga. Y el mock ganó **una palanca y no la
  otra**, que es la forma de un proveedor real y deja el e2e ejercitando las dos
  ramas sin proveedor alguno. -->
- [x] 3.3 Los adaptadores anuncian sus opciones como *session config options* de
  ACP, que es la vía estándar y la anuncia el agente (design D2)
  <!-- 2026-08-19: **no anuncian, y ese es el resultado** (design D9). Un
  `select` de ACP exige la lista de valores, y ninguno de los dos proveedores la
  da: el esquema pineado de Codex no tiene método que enumere modelos
  (`InitializeResponse` trae solo `userAgent`) y el CLI de Claude tampoco.
  Anunciar exigiría incrustar un catálogo de modelos en el adaptador —lo que D1
  prohíbe y D7 ya resolvió para el esfuerzo—, y un selector con un único valor
  es un control que no puede elegir. La vía queda escrita, probada con el mock y
  disponible para cualquier agente ACP que sí anuncie. -->

## 4. El cambio en vivo, solo donde el agente lo anunció

- [x] 4.1 El daemon fija la opción por `session/set_config_option` cuando el
  agente la anunció, sin relanzar — escenario «Se cambia por la vía estándar
  cuando el agente la anuncia»
  <!-- 2026-08-19: el daemon **guarda la conexión ACP** en el registro de
  sesiones y manda la petición por ella. Se puede porque ACP es full-duplex y
  `ConnectionTo` es un handle clonable: la petición no espera a un límite de
  turno, y bloquear esperando su respuesta solo es deadlock dentro de un
  manejador de la conexión, que no es donde vive el verbo. Lo que responde el
  **agente** es lo que se guarda y se registra, no lo que pedimos: uno que
  recorte o reordene se cree por encima de la petición. -->
- [x] 4.2 Sin opción anunciada, la superficie **no lo ofrece** — escenario «Sin
  opción anunciada no se ofrece el cambio en vivo»
  <!-- 2026-08-19: el anuncio viaja por el **registro append-only**, donde ya
  aterrizan los otros hechos del handshake (`mcp_injected`,
  `mcp_not_delivered`). Sin anuncio no hay evento, y esa ausencia es la
  respuesta entera: el control de la GUI se deriva del último evento del
  transcript, así que no puede discrepar del registro del que salió. Y el
  daemon rehúsa con 2007 aunque una superficie lo ofreciera igual. -->
- [x] 4.3 El mock-agent anuncia opciones detrás de una bandera apagada por
  defecto, para ejercitar la vía sin proveedor alguno
  <!-- 2026-08-19: `--config-options`, apagada como las demás y por el mismo
  motivo: anunciar siempre metería un evento nuevo en el registro que leen los
  e2e ya escritos. Anuncia **un selector y un interruptor**, que son las dos
  clases de ACP y dos caminos distintos en el daemon; con una sola quedaría la
  otra sin probar. Y guarda estado real: el cambio se ve en el anuncio
  siguiente, que es lo que el daemon lee de vuelta. -->

## 5. Las superficies

- [x] 5.1 GUI: chip «modelo · esfuerzo» en el lanzador, con búsqueda y entrada
  libre — escenario «Se elige con búsqueda y se admite entrada libre»
- [x] 5.2 GUI: la ficha muestra solo lo que Meltemi sabe —lo anunciado, lo
  declarado, lo medido— y **sin precios ni créditos** (design D6) — escenario
  «La ficha no inventa lo que no sabe»
  <!-- 2026-08-17: la ficha necesitaba saber qué modelos declaró un perfil, así
  que la entrada de flota los expone —solo las filas de perfil, porque un modelo
  pertenece a **cómo** el usuario corre un agente, no al agente—. Y el guardián
  de «sin precios» se tropezó con su propia clave `noPrices` la primera vez que
  corrió: la frase que dice que no hay precios no es un precio, y ahora sale del
  pajar antes de buscar. -->
- [x] 5.3 GUI: cambiar en marcha advierte el efecto sobre caché y costo —
  escenario «Cambiar en marcha se advierte»
- [x] 5.4 TUI: modelo efectivo visible donde muestra el estado, y omitido cuando
  no hay — escenario «El terminal muestra el modelo efectivo»
- [x] 5.5 CLI: `--model` y `--effort` con su ayuda diciendo que son cadenas del
  proveedor que el núcleo no interpreta
- [x] 5.6 i18n es/en de todo lo nuevo, con el lint como guardián

## 6. Cierre

- [x] 6.1 E2e contra el mock: una sesión con modelo declarado, el valor efectivo
  en el registro, y el rehúso de la palanca no admitida
- [x] 6.2 Validación manual contra los CLIs reales, **documentada como manual**
  con las versiones probadas (design D7)
- [x] 6.3 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend y paridad revisada
  <!-- 2026-08-19: `validate` limpio, `verify` 14/14, 91 suites verdes, clippy y
  fmt en cero, `svelte-check` sin errores e `i18n lint` limpio. Paridad: el
  verbo nuevo entra en las tres superficies y en la matriz. Dos guardianes
  encontraron cosas al pasar — el que exige glifo y tono para cada tipo de
  evento, y el que prohíbe variables de estilo inventadas (`--s-1` y `--r-1` no
  existen; son `--sp-1` y `--radius-control`). Y el de «nada se mueve mientras
  se decide un permiso» se disparó con la palabra «animation» de un comentario
  mío, igual que `noPrices` con su propia clave: la prosa cae dentro del pajar
  que el guardián registra. -->
