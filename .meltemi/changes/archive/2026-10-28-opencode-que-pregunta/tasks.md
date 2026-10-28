# Tasks — opencode-que-pregunta

Vía rápida. Un commit atómico por tarea, con referencia
`(opencode-que-pregunta N.M)` y sin trailers de co-autoría.

## 1. El entorno de lanzamiento

- [x] 1.1 `launch-env` en el registro, hasta la entrada del catálogo (design
  D1) — gate: tests de `fleet.rs`
  <!-- 2026-10-28: tabla `launch-env` en `RawRegistryAgent`, `Vec` ordenado en
  `RegistryAgent` y `CatalogEntry`; una entrada personalizada no declara
  ninguno. Test de parseo con una entrada que lo declara y otra que no. -->
- [x] 1.2 Las cuatro resoluciones lo aplican con su precedencia, y el comando
  literal no (design D2, D3) — escenarios «Lo que el usuario fijó manda» y «Un
  comando literal no hereda el entorno del registro»; gates: tests de
  `levels.rs`
  <!-- 2026-10-28: `launch_env` compone registro < entorno del usuario <
  perfil; una variable que el usuario ya tiene se omite para que el agente la
  herede del daemon. `configured_env` no aplica nada con `agent.command`. Dos
  tests: precedencia por id, perfil, configurado y etiqueta libre; y el comando
  literal junto a un id que sí declara. -->
- [x] 1.3 La entrada de OpenCode lo declara (design D4); la guía de agentes y
  la pierna manual de `docs/conformidad-manual.md` (design D5) — escenario
  «OpenCode se lanza de modo que pregunte»; gates: tests de la guía y de la
  resolución contra la instantánea real del registro
  <!-- 2026-10-28: `launch-env` en la entrada `opencode`, documentado en la
  cabecera del registro, cuya revisión pasa a 2026-10-28. La guía nombra la
  variable, muestra el valor y dice qué hacer si el usuario ya la fija; el test
  de la guía lo exige para toda entrada que declare `launch-env`. La pierna
  «Pierna de permisos — OpenCode lanzado de modo que pregunte» queda pendiente.
  El test contra la instantánea real resuelve por id, por perfil y como agente
  del proyecto, y parsea el JSON. -->

## 2. Cierre

- [x] 2.1 `meltemi validate` y `verify` con los tres escenarios; entrada en
  `docs/plan-de-cambios.md`
  <!-- 2026-10-28: validate limpio, verify 3/3 enlazados; entrada «5º» en el
  avance de la tanda. -->
