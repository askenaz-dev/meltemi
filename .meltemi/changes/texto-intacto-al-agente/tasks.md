# Tasks — texto-intacto-al-agente

## 1. Expansión que respeta el texto

- [x] 1.1 Copiar el tramo literal entre referencias como una sola rebanada
  `&str` en `expand_refs` (`core/meltemid/src/repo_map.rs`) — `find('@')` para
  hallar el próximo `@`, `push_str(&text[i..at])` para empujarlo, sin ninguna
  conversión de byte a `char` (design D1) — y cubrir «Prompt en español
  íntegro», «Arroba doble literal» y «Referencia pegada a un carácter
  multibyte» en el módulo de tests, con la cadena medida por el smoke
  («acción íntegra ñandú», 20 caracteres) como caso de regresión con nombre
  propio (design D3)

- [x] 1.2 Pasar `is_ref_char` de `u8` a `char` con `is_alphanumeric()` y
  escanear el token con `char_indices` acumulando `len_utf8()` (design D2), y
  cubrir «Ruta con carácter no ASCII resuelta» y «Puntuación no ASCII cierra
  el token» en el módulo de tests

## 2. Verificación

- [x] 2.1 Gates locales — `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` y
  `meltemi validate texto-intacto-al-agente` limpio — y `meltemi verify
  texto-intacto-al-agente` con los cinco escenarios enlazados a sus tests
