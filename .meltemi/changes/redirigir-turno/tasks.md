# Tareas — redirigir-turno

Vía completa: toca la semántica de sesión del daemon, el contrato y las dos
superficies. Un commit atómico por tarea, con referencia `(redirigir-turno
N.M)` y sin trailers de co-autoría. Gates del repo en cada tarea.

## 1. La cola y su atomicidad

- [x] 1.1 `InstructionQueue` gana la bandera turn-scoped y una operación que
  **encola el relevo y señala la interrupción bajo el mismo lock**, en ese
  orden (design D1) — escenario «La instrucción releva al turno interrumpido»
- [x] 1.2 El guardián vigente se enmienda con su gemelo: una cola **cancelada**
  sigue sin despachar nada, una cola **interrumpida** despacha su relevo y solo
  ese; los dos tests se escriben juntos para que la diferencia se lea (design
  D5) — escenario «Una cancelación sigue terminando la sesión»
  <!-- 2026-08-10: los dos gemelos quedan **uno junto al otro** en
  `session.rs`, que era el punto: leer el archivo enseña la diferencia sin
  buscarla. Se añadió además el tercero que faltaba —interrumpir una sesión que
  ya dejó de aceptar devuelve `None` y **no señala nada**—, porque una
  interrupción sin relevo sería una cancelación que nadie pidió. -->

## 2. El borde del turno

- [x] 2.1 El borde distingue por bandera y no por estado: cancelación cierra y
  rompe; interrupción con relevo consume y sigue, limpiando la bandera; un
  `Cancelled` espontáneo **rompe como hoy** (design D2) — escenario «Un turno
  cancelado por el agente no continúa»
- [x] 2.2 Las tres carreras con su test: interrupción que llega cuando el turno
  ya terminaba, dos interrupciones seguidas, e interrupción contra cancelación
  simultánea —donde **cancelar gana** (design D1) — escenario «Cancelar gana a
  interrumpir»
  <!-- 2026-08-15: la carrera «llega tarde» resultó no ser hipotética. El borde
  consumía la bandera solo en turnos cancelados, así que una interrupción cuyo
  turno ya había terminado quedaba puesta y habría hecho pasar por interrumpido
  a un turno posterior. Ahora se consume en todo borde y solo se USA en la rama
  cancelada. -->

## 3. Los permisos y el registro

- [x] 3.1 La espera de permisos gana su rama de cancelación y llama a
  `drop_request`, que existe sin llamador desde que se escribió; el desenlace
  cuenta como denegación en el ledger (design D3) — escenario «El permiso en
  vuelo se resuelve al interrumpir»
  <!-- 2026-08-15: `drop_request` estrenó llamador. Además hizo falta lo que el
  design no había previsto: `notify_waiters` solo despierta a quien ya espera,
  así que una interrupción pedida ANTES de que el turno arranque no despertaba
  a nadie — y el daemon respondía `relayed` por algo que no ocurrió. Ahora
  pregunta primero si hay turno en vuelo; si no lo hay, encola y lo dice. La
  bandera se consume en todo borde, no solo en los cancelados, para que no
  gobierne un turno posterior. -->
- [x] 3.2 El registro distingue quién detuvo el turno (design D4) — escenario
  «El registro dice quién detuvo el turno»
  <!-- 2026-08-15: evento propio `turn_interrupted` con la instrucción que
  relevó, porque `turn_completed { cancelled }` se lee igual si el agente se
  rindió que si un humano lo redirigió. Y en el ledger, decisor `interrupted`:
  las alternativas mentían —ningún cliente ausente, ningún reloj vencido. -->

## 4. El contrato

- [x] 4.1 `session/direct` gana `interrupt` opcional en `meltemi-proto` y su
  schema, con la conformidad de tres vías y `gen:forms` commiteado; el
  resultado dice cuál de los dos desenlaces ocurrió

## 5. Las superficies y el mock

- [x] 5.1 El mock honra `CancelNotification` detrás de una bandera **apagada
  por defecto**, para no cambiar lo que leen los e2e de cancelación vigentes
  (design D6)
- [x] 5.2 E2e de interrupción con relevo contra ese mock: turno largo,
  interrupción, y el turno siguiente corriendo la instrucción que relevó
  <!-- 2026-08-15: el e2e del permiso colgado NO espera a que el flujo termine.
  El relevo es un turno nuevo que pide su propio permiso, y esperar el final
  mediría la SEGUNDA espera en vez de la que la interrupción vino a terminar. -->
- [x] 5.3 GUI: interrumpir y enviar junto al envío, solo con texto (design D2)
  — escenarios «Interrumpir y enviar se ofrece con texto y sesión trabajando» y
  «Sin texto no hay nada que relevar»
- [x] 5.4 TUI: el mismo gesto sobre su flujo de dirección, diciendo cuál de los
  dos desenlaces ocurrió — escenario «El shell dice si encoló o relevó»
  <!-- 2026-08-15: el keymap no admite modificadores por diseño y dentro de un
  campo toda letra es texto, así que el gesto es **Tab**, la única tecla que ni
  escribe ni navega. El campo dice cuál de los dos envíos está armado antes de
  pulsar Enter, y el verbo `interrumpir` de la paleta abre ese mismo campo ya
  armado. Interrumpir solo se ofrece con un turno corriendo: una sesión
  terminada no tiene turno que relevar. -->

## 6. Cierre

- [x] 6.1 `meltemi validate redirigir-turno` limpio y `meltemi verify` con los
  nueve escenarios enlazados; suite completa, clippy, fmt, gates del frontend y
  `docs/paridad-nucleo.md` revisado (el verbo existe: se comprueba que no haga
  falta fila nueva)
