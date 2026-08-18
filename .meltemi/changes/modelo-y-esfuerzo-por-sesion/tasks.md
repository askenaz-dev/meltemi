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

- [ ] 3.1 Codex: `model` al arrancar el hilo y `effort` **por turno**, que es
  donde su esquema pineado los define — verificado, no citado de memoria
  (design D3) — escenario «El adaptador manda la palanca donde su proveedor la
  acepta»
- [ ] 3.2 Claude: `--model` en `session_args()`; **esfuerzo NO se cablea** y se
  rehúsa con ese motivo, porque no está verificado contra el CLI pineado
  (design D7)
- [ ] 3.3 Los adaptadores anuncian sus opciones como *session config options* de
  ACP, que es la vía estándar y la anuncia el agente (design D2)

## 4. El cambio en vivo, solo donde el agente lo anunció

- [ ] 4.1 El daemon fija la opción por `session/set_config_option` cuando el
  agente la anunció, sin relanzar — escenario «Se cambia por la vía estándar
  cuando el agente la anuncia»
- [ ] 4.2 Sin opción anunciada, la superficie **no lo ofrece** — escenario «Sin
  opción anunciada no se ofrece el cambio en vivo»
- [ ] 4.3 El mock-agent anuncia opciones detrás de una bandera apagada por
  defecto, para ejercitar la vía sin proveedor alguno

## 5. Las superficies

- [ ] 5.1 GUI: chip «modelo · esfuerzo» en el lanzador, con búsqueda y entrada
  libre — escenario «Se elige con búsqueda y se admite entrada libre»
- [ ] 5.2 GUI: la ficha muestra solo lo que Meltemi sabe —lo anunciado, lo
  declarado, lo medido— y **sin precios ni créditos** (design D6) — escenario
  «La ficha no inventa lo que no sabe»
- [ ] 5.3 GUI: cambiar en marcha advierte el efecto sobre caché y costo —
  escenario «Cambiar en marcha se advierte»
- [ ] 5.4 TUI: modelo efectivo visible donde muestra el estado, y omitido cuando
  no hay — escenario «El terminal muestra el modelo efectivo»
- [x] 5.5 CLI: `--model` y `--effort` con su ayuda diciendo que son cadenas del
  proveedor que el núcleo no interpreta
- [ ] 5.6 i18n es/en de todo lo nuevo, con el lint como guardián

## 6. Cierre

- [ ] 6.1 E2e contra el mock: una sesión con modelo declarado, el valor efectivo
  en el registro, y el rehúso de la palanca no admitida
- [ ] 6.2 Validación manual contra los CLIs reales, **documentada como manual**
  con las versiones probadas (design D7)
- [ ] 6.3 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend y paridad revisada
