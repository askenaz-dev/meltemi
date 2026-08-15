# Tareas — redirigir-turno

Vía completa: toca la semántica de sesión del daemon, el contrato y las dos
superficies. Un commit atómico por tarea, con referencia `(redirigir-turno
N.M)` y sin trailers de co-autoría. Gates del repo en cada tarea.

## 1. La cola y su atomicidad

- [ ] 1.1 `InstructionQueue` gana la bandera turn-scoped y una operación que
  **encola el relevo y señala la interrupción bajo el mismo lock**, en ese
  orden (design D1) — escenario «La instrucción releva al turno interrumpido»
- [ ] 1.2 El guardián vigente se enmienda con su gemelo: una cola **cancelada**
  sigue sin despachar nada, una cola **interrumpida** despacha su relevo y solo
  ese; los dos tests se escriben juntos para que la diferencia se lea (design
  D5) — escenario «Una cancelación sigue terminando la sesión»

## 2. El borde del turno

- [ ] 2.1 El borde distingue por bandera y no por estado: cancelación cierra y
  rompe; interrupción con relevo consume y sigue, limpiando la bandera; un
  `Cancelled` espontáneo **rompe como hoy** (design D2) — escenario «Un turno
  cancelado por el agente no continúa»
- [ ] 2.2 Las tres carreras con su test: interrupción que llega cuando el turno
  ya terminaba, dos interrupciones seguidas, e interrupción contra cancelación
  simultánea —donde **cancelar gana** (design D1) — escenario «Cancelar gana a
  interrumpir»

## 3. Los permisos y el registro

- [ ] 3.1 La espera de permisos gana su rama de cancelación y llama a
  `drop_request`, que existe sin llamador desde que se escribió; el desenlace
  cuenta como denegación en el ledger (design D3) — escenario «El permiso en
  vuelo se resuelve al interrumpir»
- [ ] 3.2 El registro distingue quién detuvo el turno (design D4) — escenario
  «El registro dice quién detuvo el turno»

## 4. El contrato

- [ ] 4.1 `session/direct` gana `interrupt` opcional en `meltemi-proto` y su
  schema, con la conformidad de tres vías y `gen:forms` commiteado; el
  resultado dice cuál de los dos desenlaces ocurrió

## 5. Las superficies y el mock

- [ ] 5.1 El mock honra `CancelNotification` detrás de una bandera **apagada
  por defecto**, para no cambiar lo que leen los e2e de cancelación vigentes
  (design D6)
- [ ] 5.2 E2e de interrupción con relevo contra ese mock: turno largo,
  interrupción, y el turno siguiente corriendo la instrucción que relevó
- [ ] 5.3 GUI: interrumpir y enviar junto al envío, solo con texto (design D2)
  — escenarios «Interrumpir y enviar se ofrece con texto y sesión trabajando» y
  «Sin texto no hay nada que relevar»
- [ ] 5.4 TUI: el mismo gesto sobre su flujo de dirección, diciendo cuál de los
  dos desenlaces ocurrió — escenario «El shell dice si encoló o relevó»

## 6. Cierre

- [ ] 6.1 `meltemi validate redirigir-turno` limpio y `meltemi verify` con los
  nueve escenarios enlazados; suite completa, clippy, fmt, gates del frontend y
  `docs/paridad-nucleo.md` revisado (el verbo existe: se comprueba que no haga
  falta fila nueva)
