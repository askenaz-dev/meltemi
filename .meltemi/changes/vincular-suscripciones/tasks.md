# Tareas — vincular-suscripciones

Orden: contrato y registro primero (1), daemon después (2), superficies sobre
el contrato verde (3 CLI, 4 GUI, 5 TUI), cierre transversal (6). Un commit
atómico por tarea, con referencia `(vincular-suscripciones N.M)` y sin
trailers de co-autoría. Gates del repo en cada tarea: `cargo clippy -- -D
warnings`, `cargo fmt --check` y la suite del crate tocado.

## 1. Contrato y registro: el conocimiento se vuelve dato

- [x] 1.1 `proto/`: métodos `subscription/link`/`subscription/unlink`,
  `SubscriptionLinkParams/Result` (agente, nombre; respuesta con perfil,
  variable, valor, gesto de login y ruta del contexto) y
  `SubscriptionUnlinkParams/Result` (nombre; respuesta con la ruta que queda
  atrás); esquema `subscription.schema.json` + conformidad (design D5, D7) —
  gates: `cargo test -p meltemi-proto`
- [x] 1.2 Registro: campos `auth-context-var` y `login-hint` por entrada
  (`#[serde(default)]`, datos de la instantánea), poblados para claude-code
  (`CLAUDE_CONFIG_DIR`) y codex-cli (`CODEX_HOME`) con la verificación del
  2026-08-08 anotada y `version` de la instantánea actualizada; expuestos en
  el catálogo (design D3) — escenarios «El registro declara la variable por
  entrada» y «Registro sustituido declara sus propias variables» — gates:
  `cargo test -p meltemid`

## 2. Daemon: el vínculo nace, muere y se advierte

- [x] 2.1 Almacén `subscriptions.toml` en el directorio de config del daemon
  (cabecera de archivo gestionado, reescritura completa, solo bloques de
  perfil), cargado en `Config::load` **antes** que el config de usuario;
  validación kebab del nombre (design D2, D4) — escenario «El nombre inválido
  como ruta rehúsa» — gates: `cargo test -p meltemid`
- [x] 2.2 `subscription/link`: rehúso sin variable declarada (remedio: vía
  manual) y de nombre ya vinculado; creación del directorio de contexto
  vacío bajo datos del daemon; perfil persistido; respuesta con el gesto
  compuesto por plataforma (design D3, D4, D5) — escenarios «Vincular crea el
  perfil y la sesión lo honra», «Vincular sobre un agente sin variable
  declarada rehúsa», «El vínculo entrega el gesto de login» y «Nombre ya
  vinculado rehúsa» — gates: `cargo test -p meltemid`
- [x] 2.3 `subscription/unlink`: retiro solo del archivo gestionado, rehúso
  con remedio para perfiles manuales, directorio de contexto intacto y
  nombrado en la respuesta (design D2, D4) — escenarios «Lo escrito a mano
  gana y no se desvincula por superficie» y «Desvincular deja el contexto
  intacto» — gates: `cargo test -p meltemid`
- [ ] 2.4 Higiene afinada y duplicados: la rama `opaque` de
  `looks_like_plaintext_secret` excluye valores con separador de ruta (una
  ruta no es una credencial opaca; un token sin separadores sigue cayendo) y
  el diagnóstico de contexto duplicado entra a `Config::apply` (mismo agente,
  mismo valor resuelto) (design D4, D6) — escenarios «La ruta de contexto no
  es un secreto» y «Mismo contexto dos veces se advierte» — gates: `cargo
  test -p meltemid`

## 3. CLI: los verbos del vínculo

- [ ] 3.1 Gramática `link <agente> <nombre>` / `unlink <nombre>` (nombre
  verbatim), ejecución y render (gesto de login impreso; rehúsos con remedio
  como error de contrato); referencia CLI regenerada (design D7) — escenarios
  «link crea y responde con el gesto de login» y «unlink de un vínculo manual
  rehúsa con remedio» — gates: `cargo test -p meltemi`

## 4. GUI: la ficha del agente vincula

- [ ] 4.1 Ficha de Flota: «Vincular suscripción» solo con variable declarada
  (formulario de un campo: el nombre), fila nueva sin recargar, gesto de
  login con copia; entradas sin variable señalan la vía manual; strings ES/EN
  (design D3, D5, D7) — escenarios «Vincular desde la ficha del agente», «El
  gesto de login queda a un clic de copiar» y «La entrada sin variable señala
  la vía manual» — gates: suite de cableado +
  `npm run check:forms`
- [ ] 4.2 Desvincular desde la ficha con la declaración de lo que no borra
  (design D4) — escenario «Desvincular dice lo que no borra» — gates: suite
  de cableado

## 5. TUI: el verbo con captura verbatim

- [ ] 5.1 Verbo `link` en la paleta (overlay de captura verbatim para
  `agente nombre`, patrón del alta de proyectos), `unlink` por línea de
  paleta; Command/Update en el actor; avisos con gesto o remedio; entradas en
  el registro de paleta para los dos métodos (design D7) — escenarios «El
  verbo de vínculo captura el nombre tal cual» y «El rehúso llega con su
  remedio al shell» — gates: `cargo test -p meltemi`

## 6. Cierre: paridad, docs y verificación

- [ ] 6.1 Matriz de paridad con las dos filas nuevas; `docs/agentes.md` con
  el flujo vinculado (tabla de variables por proveedor citando la
  verificación, el gesto de login, la vía manual conservada); la tabla se
  verifica por el lint de docs vigente (initial-docs: la guía verificada
  contra el registro), sin delta nuevo (design D3, D5, D7)
  — gates: `cargo test -p meltemi --test parity --test docs`
- [ ] 6.2 E2e integrador con el ejemplo fundacional: dos vínculos de un
  proveedor y tres de otro (registro fixture), carrera/despacho honrando dos
  de ellos con contextos distintos, `session/list` con la suscripción de cada
  uno; este e2e lleva el marcador del escenario «Vincular crea el perfil y la
  sesión lo honra» (design D1) — gates: `cargo test -p meltemid`
- [ ] 6.3 `meltemi validate vincular-suscripciones` limpio y `meltemi verify
  vincular-suscripciones` con los diecinueve escenarios enlazados (meta: cero
  marcas manuales); suite completa, clippy y fmt verdes; registro del cierre
  en `docs/plan-de-cambios.md`
