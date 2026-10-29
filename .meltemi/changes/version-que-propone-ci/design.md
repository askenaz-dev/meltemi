# Design — version-que-propone-ci

## Context

`meltemi-release` ya hace casi todo lo que hace falta. `changelog` lista las
changes archivadas que ninguna sección lista; `set-version` mueve los seis
portadores y rehúsa una versión que no avanza; `gate` decide en `release.yml`
si un commit de `main` publica. Le falta decidir el número. Y al camino le
falta alguien que abra la PR, porque el token del workflow no puede: las PRs
que crea no disparan CI, y el Terraform de meltemi-web deja apagado
`can_approve_pull_request_reviews`.

La política está en `docs/versionado.md`. Antes de 1.0, una ruptura sube el
minor, lo aditivo sube el patch y un hito probado sube el minor. Ruptura es
alterar uno de los tres contratos: los JSON Schemas de `proto/` (o
`PROTOCOL_VERSION`), la gramática de la CLI (o la taxonomía de códigos de
salida) y el formato en disco de `.meltemi/`.

## Decisions

### D1 — Lo pendiente es lo que ya calcula `changelog`

Pendiente es cada change archivada que ninguna sección de `CHANGELOG.md` lista,
la misma regla que ya usa `write_changelog`. Se extrae a una función
(`pending_changes`) y la usan las dos. Sin nada pendiente no hay versión que
proponer. Es la única fuente: no se cuentan commits ni se leen mensajes de
commit, porque la unidad de una release es la change.

*Alternativa descartada*: commits convencionales (`feat:`, `fix:`). El proyecto
no los usa, y el método ya tiene una unidad mejor.

### D2 — Ruptura medida en dos contratos, declarada en el tercero

«El último tag» es el mayor `vX.Y.Z` del repositorio. Si no hay ninguno, no
hay ruptura que medir.

- **Contrato `proto/`**: para cada `proto/schemas/v1/*.schema.json` del tag,
  `git show <tag>:<ruta>` contra el árbol. Cuenta como ruptura:
  - un archivo de schema que desaparece (un método que se va);
  - una propiedad de un `$defs` que desaparece;
  - un nombre que entra a un `required` donde no estaba;
  - un valor que sale de un `enum`;
  - un `const` que cambia;
  - `PROTOCOL_VERSION` distinto en `proto/meltemi-proto/src/lib.rs`.

  Un schema nuevo, una propiedad nueva opcional o un valor nuevo de enum no
  cuentan.
- **Gramática de la CLI**: el bloque `## Grammar` de `docs/referencia-cli.md`
  del tag contra el del árbol. Se comparan los nombres de subcomando (la
  primera palabra de cada línea con sangría de cuatro espacios bajo
  `SUBCOMMANDS:`) y las banderas `--x`. Una ausencia es ruptura, y también lo
  es cualquier diferencia en la tabla de `## Exit codes`. El archivo lo genera
  `meltemi::cli::reference()` y un test ya lo ata al código, así que medir el
  documento es medir la gramática sin compilar la versión del tag.
- **Formato de `.meltemi/`**: no hay diff fiable. Lo declara la propuesta con
  una línea `> **Ruptura**: <qué rompe>`. La línea sirve también para declarar
  rupturas de los otros dos contratos que el diff no ve (un cambio de
  semántica con la misma forma).

La detección implementa la definición escrita, sin margen. Si el mantenedor
cree que algo medido como ruptura no lo es, lo que cambia es la política, no
la herramienta. Por eso no hay bandera para ignorar una señal.

*Alternativa descartada*: compilar la CLI del tag para comparar su ayuda. Es
lento y frágil; el documento generado ya es la gramática, con test.

### D3 — El hito sale del informe, y del informe correcto

Un archivo `docs/qa/*-prueba-de-uno.md` añadido desde el último tag (`git diff
--name-only --diff-filter=A <tag>..HEAD`) cuenta como hito si cumple dos
condiciones:

- su línea `Release:` nombra la última versión publicada: se prueba lo que la
  gente instala;
- la primera línea no vacía bajo `## Veredicto` empieza por «se publica»; vale
  también «se publica con hallazgos declarados», y no vale «no se publica».

Entonces sube el minor y `prepare` pasa el informe a `changelog --report`. Con
un informe cuyo veredicto no publica, no hay hito, y el cuerpo de la PR lo
dice.

Hasta que el mantenedor corra la prueba, la PR propondrá 0.2.1 con lo que hay.
El mantenedor no la mergea; cuando el informe llega a `main`, la PR se
regenera con 0.3.0 y el informe enlazado. Ese es el camino de la 0.3.0.

### D4 — Después de 1.0, el major

Con major ≥ 1:

| Pendiente | Sube |
| --- | --- |
| alguna ruptura (medida o declarada) | major |
| alguna change aditiva, o un hito | minor |
| solo correcciones | patch |

Una change es corrección si su propuesta lleva `> **Corrección**: <qué>`. Sin
línea, es aditiva: el error seguro es subir de más. Antes de 1.0 la línea de
corrección no cambia nada (lo aditivo ya sube el patch), pero se acepta desde
ya para que la costumbre exista cuando importe. Las reglas entran a
`docs/versionado.md` en esta change.

### D5 — La lista para 1.0, en código y atada al documento fundacional

`next` calcula una lista con dos grupos. Cada gate lee un hecho; los que no
están en el repositorio los trae el workflow como archivos (`--facts <dir>`).

| Gate | Hecho | De dónde |
| --- | --- | --- |
| Paridad de núcleo verificada por CI | `docs/paridad-nucleo.md` existe y `ci.yml` corre `cargo test --workspace`, que la verifica | repositorio |
| Instalable en macOS, Windows y Linux | la última release tiene el núcleo y el instalador de escritorio de las tres, por sus nombres estables | `assets.json` (`gh release view`) |
| Headless por SSH | la gramática lista `bridge` | repositorio |
| MSI, DMG y APK firmados | el `capabilities.json` de la última release los declara firmados | `capabilities.json` (`estado-que-publica-la-release`, `firma-de-plataforma`) |
| La prueba de un desarrollador | un informe cuyo veredicto publica la última versión | repositorio (D3) |
| Sin ruptura en las dos últimas releases | ninguna de las dos últimas secciones de `CHANGELOG.md` lleva `**Breaking**`; una sección anterior a esta herramienta que sube el minor sin informe cuenta como ruptura | repositorio |

Un gate cuyo hecho falta (no hay `capabilities.json`, o el archivo no tiene el
campo) está en rojo con «sin dato: <qué falta>». Nunca está en verde.

Los informativos son el motor propio, el sandbox, los hooks y los plugins
sobre el SDK. Se muestran con el estado de su change: archivada, abierta o
solo nombrada en `docs/plan-de-cambios.md`. Que no sean gate es la lectura de
`entrega-sin-huecos` D3, y la confirma el mantenedor (tarea 3.3 de esa change);
si dice que sí son gate, se mueven de grupo en una constante.

Con major 0 y todos los gates en verde, `next` propone **1.0.0**, aunque lo
pendiente diera otra cosa. Un test lee la línea «**Hito v1.0**» de `meltemi.md`
§10 y exige que cada criterio que nombra tenga su gate, de modo que el
documento y la lista no puedan divergir sin que CI lo diga.

### D6 — `prepare` y el cuerpo de la PR

`prepare [--facts <dir>] [--body <archivo>]` corre `next` y, si hay versión:

1. escribe la sección con `write_changelog`, que gana dos cosas:
   - una línea `**Breaking**: <qué>` por cada ruptura, debajo de la línea del
     informe si la hay; así el escenario «Ruptura exige minor» («el cambio
     SHALL constar en las notas») se cumple solo;
   - el `--report` cuando hay hito;
2. llama a `set_version`;
3. escribe en `--body` el cuerpo de la PR: la versión y la regla que la dio,
   cada señal con su evidencia (archivo y diff resumido), la lista para 1.0 con
   ✅/❌ y su hecho, y la sección del changelog tal como saldrá en las notas.

Sin nada pendiente, `prepare` no toca el árbol y sale con un código propio
(`3`, «nada que proponer») para que el workflow distinga ese caso de un error.

### D7 — `release-pr.yml` y la GitHub App

```
on: push (main), workflow_dispatch
permissions: contents: read
concurrency: release-pr (cancel-in-progress)
job propose — environment: release-pr
  1. actions/create-github-app-token (pineada por commit), con
     MELTEMI_APP_ID / MELTEMI_APP_KEY del entorno, limitada a este repositorio
     y a contents:write + pull-requests:write
     — sin la clave: ::warning:: y resumen «la App no está configurada
       (entrega-sin-huecos 3.1)», y el job termina en verde
  2. checkout con fetch-depth 0 y tags
  3. gh release view --json assets > facts/assets.json; descargar
     capabilities.json si la release lo tiene (si falta, sin dato)
  4. cargo run -p meltemi-release -- prepare --facts facts --body body.md
     — sale 3: cerrar la PR de release/next si existe y borrar la rama; fin
  5. commit «Release X.Y.Z» en release/next, push --force a release/next
  6. gh pr create, o gh pr edit si ya existe; título «Release X.Y.Z»,
     cuerpo body.md, base main
```

- **Por qué una App y no el token del workflow**: el token del workflow no
  dispara CI en las PRs que crea, y la PR de release tiene que pasar los
  mismos gates que cualquier otra antes de que el mantenedor la mergee. Un PAT
  sí los dispara, pero es una credencial personal de largo plazo. La App da un
  token de una hora, limitado al repositorio y a dos permisos, y el
  mantenedor ya la tiene (`entrega-sin-huecos` D4).
- **Por qué `--force` a `release/next`**: la rama es de la App. Se regenera
  desde `main` en cada push, así que la PR siempre es «main + la release», sin
  conflictos que resolver. Lo que alguien empuje a mano a esa rama se pierde en
  el siguiente push a `main`; el cuerpo de la PR lo dice.
- **Dependencia nueva** (constitución §10): `actions/create-github-app-token`,
  la acción oficial de GitHub para tokens de App, pineada por commit completo
  como exige «Controles estructurales del camino sin paso humano». La
  alternativa, firmar el JWT a mano con `openssl` y llamar a la API, es más
  código propio en el sitio menos visible.
- **Lo que nunca hace**: empujar a `main`, crear tags, mergear o aprobar. Un
  test estático lo fija leyendo el YAML, como ya se fijan los permisos y los
  pines.

### D8 — Quién escribe la versión

`AGENTS.md` dice que un agente no cambia la versión salvo que el mantenedor lo
pida con palabras, y `release-para-un-desarrollador` exige que la versión de
una release de hito la publique un gesto del mantenedor. Las dos reglas siguen
en pie. La App propone en una rama que no es `main`; el commit de versión llega
a `main` solo por el merge del mantenedor. La frase de `AGENTS.md` se reescribe
para decirlo: la versión la propone CI en la PR de release y la publica el
merge del mantenedor, y un agente sigue sin escribirla en `main`. El camino
manual (`changelog` + `set-version` + push) queda en `docs/release.md` como
respaldo, por ejemplo mientras la App no está instalada.

## Risks / Trade-offs

- **Una PR abierta que nadie mergea** es una release que no sale. La cadencia
  de `docs/release.md` (nunca más de un mes) sigue valiendo, y la PR abierta
  con su fecha es el recordatorio.
- **La heurística de la gramática** lee un documento con forma fija. Si
  `reference()` cambia de forma, el test de `next` con la referencia real del
  árbol falla antes que una release.
- **Una ruptura de semántica con la misma forma** no se mide. Para eso está la
  línea `> **Ruptura**:`, y la revisión de la propuesta.
- **El token de la App** puede escribir contenido en el repositorio. Vive en un
  entorno limitado a `main`, dura una hora y el job solo empuja a
  `release/next`. Si el mantenedor quiere más, puede proteger `main` contra la
  App en el ruleset, y el workflow no se entera.
