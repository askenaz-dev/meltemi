# Tareas — aviso-de-espera-opt-in

Vía completa. Un commit atómico por tarea, con referencia
`(aviso-de-espera-opt-in N.M)` y **sin trailers de co-autoría**. Gates del repo
en cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite del crate tocado. **Ninguna tarea añade una
dependencia**; el daemon no se toca y no hay RPC nuevo.

## 1. El emisor

- [ ] 1.1 `core/meltemi-client/src/notices.rs`: el contrato del contenido como
  un tipo —clase de espera, sesión, máquina, instante, cuenta— y su traducción a
  las cinco variables de entorno; el lector de ámbito usuario de
  `[notices] on-waiting` con el mismo lector que `via-ssh-del-cliente` añadió,
  para que la frontera exista una vez; tests de mesa del contenido y de que
  ninguna variable lleva ruta ni proyecto (design D1, D3) — escenarios «El aviso
  lleva el hecho y nada del proyecto» y «Un repositorio no configura tu aviso» —
  gates: suite de `meltemi-client`
- [ ] 1.2 El lanzamiento: sin esperar, sin heredar stdio, dentro de un ámbito de
  proceso y con tope de diez segundos; sin comando configurado no se ejecuta
  nada; test con un comando que no termina que comprueba que el emisor vuelve de
  inmediato y que el proceso muere al agotarse el tope (design D4) — escenarios
  «Sin configurar, no se emite nada» y «Un comando que no responde no bloquea al
  cliente» — gates: suite de `meltemi-client`
- [ ] 1.3 La coalescencia: un aviso por evento de espera con su cuenta, un piso
  de treinta segundos entre avisos, y nada al resolverse la espera; el registro
  de cuándo disparó vive en el emisor (design D5, D7) — escenarios «Una ráfaga de
  esperas produce un aviso con su cuenta» y «Resolver la espera no avisa» —
  gates: suite de `meltemi-client`

## 2. Las superficies

- [ ] 2.1 GUI: el emisor se invoca desde el proceso nativo y no desde la vista
  web; los ajustes dicen si está configurado y ofrecen «enviar aviso de prueba»
  con su resultado a la vista (design D2, D6) — escenario «La prueba usa el mismo
  camino que el aviso real» — gates: suite de `meltemi-desktop`, tests de la UI
- [ ] 2.2 TUI: emite al empezar una espera y ofrece el aviso de prueba en su
  paleta, diciendo la verdad cuando no hay comando configurado (design D6) —
  escenario «La paleta de la TUI envía un aviso de prueba» — gates: suite de
  `meltemi`
- [ ] 2.3 `docs/acceso-remoto.md`: el contrato del contenido **publicado con la
  change** (§9), las recetas con infraestructura del usuario (ntfy
  autohospedado, Gotify, Apprise) y la campana de la TUI como el caso sin
  servidor, cada una con la advertencia de que es del usuario — gates:
  `cargo test -p meltemi --test docs`

## 3. Cierre

- [ ] 3.1 `meltemi validate aviso-de-espera-opt-in` limpio; `meltemi verify` con
  los nueve escenarios enlazados; suite completa con `--no-fail-fast`, clippy y
  fmt verdes; `cargo deny check` verde; entrada en `docs/plan-de-cambios.md`; y
  la rama aterriza en `main` con `meltemi land aviso-de-espera-opt-in confirm`
