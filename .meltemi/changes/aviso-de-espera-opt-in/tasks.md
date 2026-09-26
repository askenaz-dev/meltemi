# Tareas — aviso-de-espera-opt-in

Vía completa. Un commit atómico por tarea, con referencia
`(aviso-de-espera-opt-in N.M)` y **sin trailers de co-autoría**. Gates del repo
en cada tarea: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, la suite del crate tocado. **Ninguna tarea añade una
dependencia**; el daemon no se toca y no hay RPC nuevo.

## 1. El emisor

- [x] 1.1 `core/meltemi-client/src/notices.rs`: el contrato del contenido como
  un tipo —clase de espera, sesión, máquina, instante, cuenta— y su traducción a
  las cinco variables de entorno; el lector de ámbito usuario de
  `[notices] on-waiting` con el mismo lector que `via-ssh-del-cliente` añadió,
  para que la frontera exista una vez; tests de mesa del contenido y de que
  ninguna variable lleva ruta ni proyecto (design D1, D3) — escenarios «El aviso
  lleva el hecho y nada del proyecto» y «Un repositorio no configura tu aviso» —
  gates: suite de `meltemi-client`
  <!-- 2026-09-26: el test del contenido asserta **por ausencia**, que es lo
  único que sirve aquí: lo interesante no es que los cinco campos estén, es que
  un sexto no se cuele mañana. Así que comprueba que ni una ruta, ni un nombre de
  proyecto, ni un texto de instrucción aparecen en ningún valor.
  El nombre de la máquina no tenía de dónde salir sin dependencia —ningún crate
  del workspace lo computaba— así que es una cascada con sus razones:
  `COMPUTERNAME` siempre está en Windows, `HOSTNAME` se prueba y no se confía
  porque suele no exportarse, `/etc/hostname` es fiable en Linux y no existe en
  macOS, y si ninguno responde el aviso dice `unknown` en vez de inventar: un
  nombre de máquina equivocado es peor que uno admitido, porque el trabajo entero
  del aviso es decir qué máquina te quiere. -->
- [x] 1.2 El lanzamiento: sin esperar, sin heredar stdio, dentro de un ámbito de
  proceso y con tope de diez segundos; sin comando configurado no se ejecuta
  nada; test con un comando que no termina que comprueba que el emisor vuelve de
  inmediato y que el proceso muere al agotarse el tope (design D4) — escenarios
  «Sin configurar, no se emite nada» y «Un comando que no responde no bloquea al
  cliente» — gates: suite de `meltemi-client`
  <!-- 2026-09-26: el vigilante del tope corre en un **hilo**, no en una tarea:
  esto se llama desde superficies que no todas tienen runtime, y el hilo posee el
  ámbito, así que el proceso queda acotado tanto si termina solo como si hay que
  terminarlo. Un comando que no se puede partir cuenta como apagado, no como
  medio ejecutado; la prueba manual es donde una persona se entera de eso.
  Dos trampas del fixture, medidas: anidar comillas en TOML dentro de un shell es
  un fixture que prueba al fixture, así que el comando es un **script** —y en
  Windows un `.cmd`, que pone `cmd.exe` en medio y de paso ejercita el ámbito—; y
  `cfg!(windows)` es un valor, no un atributo, así que las dos ramas compilan y la
  de Unix le pidió permisos de archivo a un build de Windows. Se parte con
  `#[cfg]` de verdad. -->
- [x] 1.3 La coalescencia: un aviso por evento de espera con su cuenta, un piso
  de treinta segundos entre avisos, y nada al resolverse la espera; el registro
  de cuándo disparó vive en el emisor (design D5, D7) — escenarios «Una ráfaga de
  esperas produce un aviso con su cuenta» y «Resolver la espera no avisa» —
  gates: suite de `meltemi-client`
  <!-- 2026-09-26: dos reglas que el design no había separado y que los tests
  obligaron a decidir. (1) El aviso de prueba **ignora el piso y no lo gasta**:
  quien pulsa el botón dos veces lo quiere dos veces, y un ensayo que consumiera
  el piso dejaría muda la primera espera real que llegue después. (2) `resolved`
  no manda nada **y tampoco toca el piso**, por lo mismo.
  El instante lo pone el cliente, no el daemon, porque el daemon no participa en
  un aviso y pedirle la hora lo convertiría en participante. La aritmética de la
  fecha se parte en `format_rfc3339(secs)` para poder comprobarla contra fechas
  conocidas —época, frontera de día, 29 de febrero, el día siguiente, y un siglo
  que no es bisiesto—, que es lo único que atrapa un reloj que se ve bien y está
  desplazado un día. De paso: mi valor esperado para una de esas fechas estaba mal
  y el test lo dijo; 51.200 segundos son 14:13:20, no 09:33:20. -->

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
