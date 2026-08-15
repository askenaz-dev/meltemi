# Tareas — barra-de-estado-agentica

Vía rápida: gate único al final. Deltas ADDED sobre `gui-shell` y `tui-shell`;
el contrato **no se mueve** y el daemon no gana capacidad. Un commit atómico por
tarea, con referencia `(barra-de-estado-agentica N.M)` y sin trailers de
co-autoría.

## 1. La fuente de las changes

- [ ] 1.1 El listado de changes sube de `Project.svelte` a un store propio en
  `stores.ts`, conservando la guarda `isMeltemiProject`, y la vista Proyecto
  pasa a consumirlo en vez de su copia local (design D4) — una fuente, dos
  lectores

## 2. Los segmentos

- [ ] 2.1 Proyecto (nombre corto con la ruta en el emergente) y change con su
  compuerta —la que reclama decisión, o el número de activas si ninguna—
  (design D1, D2) — escenarios «La barra nombra el proyecto y la compuerta que
  espera» y «Sin compuerta pendiente, la barra dice cuántas changes hay»
- [ ] 2.2 El recuento de sesiones se desglosa en las que trabajan y las que
  esperan una decisión; **la compuerta no se disfraza de estado de sesión**
  (design D2) — escenario «Las sesiones que trabajan se distinguen de las que
  esperan»
- [ ] 2.3 Consumo medido del proyecto en el día, con **silencio o motivo** en
  vez de un cero cuando no hay medición, y refresco al cambiar de proyecto y al
  terminar una sesión —nunca por temporizador— (design D3) — escenarios «El
  consumo medido se muestra» y «Sin medición no se inventa un cero»

## 3. Comportamiento de la barra

- [ ] 3.1 Cada segmento lleva a su vista, con nombre accesible y camino de
  teclado (design D5) — escenario «Un segmento lleva a su vista»
- [ ] 3.2 Prioridad declarada al estrecharse: endpoint, versión, consumo,
  proyecto; conexión y permisos jamás (design D6) — escenario «Al estrecharse,
  lo último que se cae». **Sin mover las expresiones que los guardianes
  vigentes leen literalmente** (`$conn.endpoint`, las tres palabras de
  conexión): añadir sin refactorizar (design D8)

## 4. El terminal

- [ ] 4.1 `LiveData` gana la change y su compuerta, y el header las muestra con
  la prioridad de señales vigente; **el consumo no entra al header** y se queda
  en su vista (design D7) — escenarios «El chrome nombra la compuerta que
  espera» y «La compuerta cede antes que la conexión»

## 5. Cierre

- [ ] 5.1 `meltemi validate barra-de-estado-agentica` limpio y `meltemi verify`
  con los nueve escenarios enlazados (meta: cero marcas manuales); suite,
  clippy, fmt y gates del frontend verdes
- [ ] 5.2 Smoke conducido sobre el binario de release (receta de
  `docs/qa/2026-08-09-piel-de-pestanas-smoke.md`): la barra con proyecto y
  compuerta reales, el desglose de sesiones, el consumo callando donde ACP no
  reporta, y **la ventana mínima de 900 px** para ver qué cede. Nota en
  `docs/qa/`
