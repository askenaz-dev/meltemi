# preguntas-del-agente — tasks

> Vía completa. El orden va del cable a la superficie: primero que exista una
> pregunta de verdad que relevar (adaptador y mock), después la superficie que la
> contesta. Al revés se estaría pintando contra un emisor imaginario.

## 1. El adaptador deja de rehusar la pregunta

- [x] 1.1 `AskUserQuestion` sale de `interactive_only`, **y el mecanismo se
  queda**: la lista sobrevive con su motivo para la siguiente herramienta que sí
  lo sea (design D1) — escenario «Lo que de verdad no se puede relevar se sigue
  rehusando»
  <!-- 2026-08-16: la lista queda **vacía**, y eso se dice en voz alta para que
  no se lea como un mecanismo que alguien borró: `AskUserQuestion` era su única
  entrada. El test que probaba el rehúso ahora prueba el mecanismo —cada nombre
  de la lista responde con su motivo, de modo que el día que se añada uno el
  rehúso ya está cableado— y afirma que la lista está vacía hoy. -->
- [x] 1.2 El input se parsea en preguntas con sus opciones (rótulo, descripción,
  `multiSelect`), tolerando una forma que no reconozca **rehusando**, jamás
  adivinando (design D7)
- [x] 1.3 Cada pregunta sale como `session/request_permission` con **las opciones
  del agente** y sus rótulos verbatim — escenario «Una pregunta llega con las
  opciones del agente»
- [x] 1.4 La excepción al input intacto, acotada en código a esta herramienta y
  al campo que el propio input declara (design D2); los dos tests vigentes se
  enmiendan **por adición** y siguen exigiendo input intacto para todo lo demás
  — escenario «Solo una pregunta completa su propio input»
- [x] 1.5 `multiSelect` se descompone en peticiones por pregunta; lo que el cable
  no admite se dice en el rótulo, no se finge (design D3)

## 2. El mock aprende a preguntar

- [ ] 2.1 Bandera **apagada por defecto** (como `--honor-cancel`): con ella, el
  mock emite una pregunta con opciones y una recomendada en su rótulo, para que
  el flujo se ejercite sin red (design D6)

## 3. El compositor contesta

- [ ] 3.1 Con la sesión esperando decisión, el compositor presenta la petición y
  sus opciones, decidiendo por `permission/decide` — el mismo verbo, jamás una
  segunda cola — escenario «La pregunta se contesta donde se escribe»
- [ ] 3.2 Teclado: recorrer con flechas, elegir con Enter, y el foco donde el
  usuario ya está mirando
- [ ] 3.3 Aparece **sin animación de layout** — la regla vigente es literal
  (`gui-shell/spec.md:266-268`) — escenario «La pregunta aparece sin mover nada»
- [ ] 3.4 Tope visual: el listado se desplaza **dentro de sí mismo**, nunca el
  panel (design D4) — escenario «Muchas opciones se desplazan dentro del
  listado»
- [ ] 3.5 La tarjeta del transcript **se conserva**: el registro es la verdad y
  la tarjeta es su lectura

## 4. La salida de texto libre, con la verdad de cada cable

- [ ] 4.1 Última opción «Otra respuesta…» que abre la caja en el mismo sitio
- [ ] 4.2 Su rótulo dice qué hará **según el agente de la sesión**: responder la
  pregunta (adaptador de Claude, por `updatedInput`) o interrumpir el turno y
  relevarlo (ACP nativo, por el verbo de `redirigir-turno`) — escenario «La
  salida de texto libre dice lo que hará»
- [ ] 4.3 i18n es/en de todo lo nuevo, con el lint como guardián

## 5. Cierre

- [ ] 5.1 E2e contra el mock: pregunta con opciones, elección desde el
  compositor, y el turno continuando con ella
- [ ] 5.2 La validación manual contra el CLI real, **documentada como manual**
  con la versión probada (design D7); CI jamás corre agentes reales
- [ ] 5.3 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt y gates del frontend; paridad revisada (no nace verbo:
  se comprueba que no haga falta fila)
