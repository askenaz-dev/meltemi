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

- [ ] 2.1 Gates — la suite completa del frontend (`npm test` en `desktop/ui`),
  `cargo test --workspace`, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings` y
  `meltemi validate familia-por-titulo` limpio— más `meltemi verify` con su
  cobertura, y la verificación negativa por mutación de design D5: quitar
  `workspace` de la lista devuelve el rojo original
