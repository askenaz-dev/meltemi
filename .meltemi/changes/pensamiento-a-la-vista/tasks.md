# Tareas — pensamiento-a-la-vista

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(pensamiento-a-la-vista N.M)` y sin trailers de co-autoría. Gates: los del
frontend en `desktop/ui` y la suite del crate tocado.

## 1. La GUI

- [x] 1.1 El bloque de pensamiento se abre mientras el turno está en vuelo y se
  pliega al cerrarse (`open={!item.closed}`), sin deshacer el plegado manual
  del usuario (design D1) — escenarios «El pensamiento se ve mientras el turno
  corre», «Plegarlo a mano no se deshace solo» y «Sin pensamiento no hay
  sección»
  <!-- 2026-08-09: `open={!item.closed}` y nada más, porque Svelte solo escribe
  el atributo cuando la expresión cambia: mientras el turno corre sigue siendo
  `true`, así que plegarlo a mano NO se deshace en el siguiente fragmento. El
  único movimiento automático es el plegado al cerrar. El test lo pinea por el
  lado negativo —ninguna forma recalculada por fragmento— porque ese sería el
  modo de romperlo sin darse cuenta. -->

## 2. El terminal

- [x] 2.1 `summarize_event` rinde lo que los eventos dicen —prosa, pensamiento
  marcado con palabra y gemelo ASCII, herramientas con su estado— y conserva la
  línea de tipo para los eventos sin contenido (design D2, D3, D4) —
  escenarios «El transcript dice lo que el agente dijo», «El pensamiento se
  distingue de la prosa» y «Un evento sin contenido sigue diciendo su tipo»
  <!-- 2026-08-09: los rótulos pasan por el `match Lang` que `conn.rs` ya usa
  para los textos compuestos (§11), no por literales sueltos. Un fragmento vacío
  cae a la línea de tipo en vez de imprimir nada, y un evento sin contenido la
  conserva: no se inventa nada para lo que no llegó. No se reconstruyó el
  pliegue por turnos de `conversation.ts` — agrupar en el shell es estructura
  nueva con su propio estado, y lo que faltaba era que las líneas dijeran algo. -->

## 3. Cierre

- [x] 3.1 `meltemi validate pensamiento-a-la-vista` limpio y `meltemi verify`
  con los seis escenarios enlazados (meta: cero marcas manuales); suite,
  clippy, fmt y gates del frontend verdes
- [x] 3.2 Smoke sobre el binario de release con una sesión que emita
  pensamiento: el bloque abierto en vuelo y plegado al cerrar, y el transcript
  del terminal leyendo prosa y pensamiento distinguidos. **El mock-agent no
  emite pensamiento hoy**: si el escenario lo necesita, se le añade al fixture
  (sigue sin red y sin agentes reales). Nota en `docs/qa/`
  <!-- 2026-08-10: el mock aprendió a pensar detrás de `--think`, y **apagado
  por defecto a propósito**: un mock que piensa siempre cambiaría lo que leen
  todos los tests de transcript que ya existen. La bandera sigue el patrón de
  `--load-session` y `--mcp`, que es como este binario declara lo que sabe
  hacer. Sin red y sin agentes reales, como manda la regla de CI. -->
  <!-- 2026-08-10, conducido: el pensamiento **se ve desplegado** con su rótulo
  y separado de la prosa (`docs/qa/2026-08-10-pensamiento-a-la-vista-smoke.md`)
  — que es el caso que la change existe para servir. **El plegado en reposo no
  se pudo confirmar**, y no por la regla: en una sesión histórica el turno nunca
  llega marcado como cerrado, así que `open={!item.closed}` obedece a un dato en
  falso. Defecto del camino que reconstruye desde el log, anterior a esta change
  (afecta igual al indicador de fin de turno) y anotado en el backlog en vez de
  arreglado aquí. La TUI queda probada por su unitario: conducirla exige un pty
  que hoy no tenemos, y se dice en vez de insinuar que se condujo. -->
