# Tasks — familia-por-titulo

## 1. El gate acepta la convención que el contrato ya tiene

- [x] 1.1 Añadir `workspace` a la lista de schemas nombrados por sus verbos en
  `desktop/ui/tests/forms.test.ts`, y reemplazar el comentario que la describe
  —«Single-method schemas keep their own names», falso para `verify-archive`
  (tres métodos) y para `workspace` (dos)— por el criterio real: llevan el
  nombre de sus verbos, no el de su familia. Cubre «Un schema nombrado por sus
  verbos pasa el gate» y «Los dos métodos del taller se ligan a su schema»
  (design D1, D5)
  <!-- 2026-09-16: `forms.test.ts` verde, 3/3. Verificación negativa por
  mutación hecha y revertida: quitar `workspace` de la lista devuelve
  exactamente el rojo original —«change/workspace resolved to
  workspace.schema.json»—, de modo que la entrada está probada por lo que
  sostiene y no solo por acompañar. El comentario nuevo nombra la razón
  (`title` como llave) además del grupo, para que el próximo schema nombrado
  por sus verbos se añada sin volver a reconstruir el porqué. -->


## 2. Verificación

- [x] 2.1 Gates — la suite completa del frontend (`npm test` en `desktop/ui`),
  `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings` y
  `meltemi validate familia-por-titulo` limpio— más `meltemi verify` con su
  cobertura, y la verificación negativa por mutación de design D5: quitar
  `workspace` de la lista devuelve el rojo original
  <!-- 2026-09-16: frontend `npm test` **66/66** (en `main`, 64/65: el único rojo
  era este; el caso nuevo de los dos métodos del taller suma uno);
  `check:forms` fresco; `lint:i18n` limpio; `svelte-check` 0 errores en 150
  archivos. `cargo clippy --workspace --all-targets -- -D warnings` limpio;
  `cargo test --workspace` **1061 tests / 95 suites, 0 fallos**, incluido
  `the_form_gate_accepts_a_schema_named_after_its_verbs`. `meltemi validate
  familia-por-titulo` limpio; `meltemi verify familia-por-titulo` **2/2
  (complete)**, los dos `linked`. La mutación de D5 se hizo y se revirtió en 1.1.

  La primera pasada de `verify` dio **0/2** con los marcadores ya puestos en
  `forms.test.ts`: el motor solo lee `Scenario:` en archivos `.rs`. Se enlazó
  como manda la convención de la superficie, con un test en
  `desktop/tests/scenarios_shell.rs` que exige que el caso esté nombrado en el
  test que CI ejecuta (design D5), y no con `sdd/verify-mark`, que es para lo
  que ningún test puede probar. Además, el taller no traía `node_modules`: dos
  suites del frontend fallaban por `Cannot find package 'svelte'` hasta
  `npm ci`, lo que no es un defecto sino el precio de un árbol recién creado.

  **Un gate rojo que no era de esta change, y que ya no está**: sobre la base del
  taller (`a36657b`), `cargo fmt --all --check` fallaba en
  `tui/tests/harness_mapping.rs:28` —un `let fixture =` partido en dos líneas que
  rustfmt quiere en una—, y era el único diff del workspace: esta change no
  añade ninguno. Lo introdujo `harness-global-y-por-agente` en su cierre
  (`0d69d59`), cuya nota declaraba fmt limpio. No se tocó aquí, y no hizo falta:
  mientras esta change se implementaba, `sesiones-en-la-barra` aterrizó en `main`
  (`ce7f501`) y su commit `0cd2d52` ya une esa línea. El fmt limpio se comprueba
  sobre el resultado de traer `main` al taller, antes de aterrizar. -->

