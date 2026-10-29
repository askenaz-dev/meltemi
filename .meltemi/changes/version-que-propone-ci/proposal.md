# version-que-propone-ci

> Vía completa (proposal → design → specs → tasks). Es la 2º de la tanda
> `entrega-sin-huecos`. La versión siguiente la calcula la integración continua
> a partir de lo archivado sin release, y llega al mantenedor como una PR
> «Release X.Y.Z» que dice por qué ese número. **Mergearla es publicar.**
> Decisiones del mantenedor del 2026-10-29: todo automatizado hasta el merge,
> un hito probado sube el minor, CI propone 1.0.0 cuando la lista de
> preparación está en verde, y una GitHub App abre la PR.

## Why

La release sale sola desde `release-con-el-push`: un commit con versión nueva
en `main` dispara los gates y la publicación. Pero ese commit lo escribe una
persona. Corre `meltemi-release changelog X.Y.Z`, luego `set-version X.Y.Z`,
y elige X.Y.Z leyendo la política de `docs/versionado.md` contra lo que se
archivó desde la última release. Es el único paso del camino que nadie
verifica, y ya falló una vez: la recomendación de 0.3.0 de
`release-para-un-desarrollador` contradecía la política, que daba 0.2.1. El
error se vio porque alguien lo leyó, no porque algo lo comprobara.

Todo lo que esa elección necesita ya está en el repositorio:

- lo pendiente: las changes de `.meltemi/changes/archive/` que ninguna
  sección de `CHANGELOG.md` lista (la herramienta ya lo calcula para escribir
  la sección);
- la ruptura en el contrato `proto/` y en la gramática de la CLI, que se puede
  medir con un diff contra el último tag: los JSON Schemas y
  `docs/referencia-cli.md`, que se genera de la gramática;
- el hito, en el informe de `docs/prueba-de-uno.md` que el mantenedor escribe
  en `docs/qa/`.

Lo que no se puede medir, una ruptura del formato de `.meltemi/`, lo declara la
propuesta de la change que la trae.

Además, 1.0.0 no tiene fecha, pero sí criterios. Calcularlos en cada PR de
release convierte «¿estamos listos?» en una lista que se lee, en vez de una
conversación que se repite.

## What Changes

- **`meltemi-release next`**: calcula la versión siguiente aplicando la
  política a lo pendiente y dice por qué. Las señales de ruptura son tres:
  - el diff de los JSON Schemas contra el último tag (un método que
    desaparece, una propiedad que desaparece, una propiedad que pasa a
    requerida, un valor de enum que desaparece, `PROTOCOL_VERSION` que cambia);
  - el diff de la gramática de `docs/referencia-cli.md` (un subcomando o una
    bandera que desaparece) y de su tabla de códigos de salida;
  - la línea `> **Ruptura**: <qué>` en la propuesta de una change pendiente.

  El hito sale de un informe de la prueba de un desarrollador, añadido desde
  el último tag, cuyo veredicto publica la última versión: sube el minor y la
  sección enlaza el informe.
- **Las reglas después de 1.0**, escritas y calculadas: una ruptura sube el
  major, lo aditivo sube el minor, y solo correcciones suben el patch. Una
  corrección se declara con `> **Corrección**: <qué>` en la propuesta.
- **La lista de preparación para 1.0**, calculada de los hechos con dos
  grupos, como fija `entrega-sin-huecos` D3. Los gates, todos en verde para
  proponer 1.0.0:
  - los tres criterios del Hito v1.0;
  - MSI, DMG y APK firmados en la última release;
  - un informe que publica la versión anterior;
  - ninguna ruptura en las dos últimas releases.

  Los informativos son los entregables de fase 2 que el hito no nombra. Un
  gate cuyo dato falta está en rojo, nunca en verde.
- **`meltemi-release prepare`**: escribe en el árbol la sección del changelog
  (con la línea del informe cuando hay hito y una línea `**Breaking**` por
  ruptura) y la versión. También escribe el cuerpo de la PR: el número, el
  porqué, la vista previa de las notas y la lista para 1.0.
- **El workflow `release-pr.yml`**: tras cada push a `main`, con un token de la
  GitHub App del mantenedor desde el entorno `release-pr`, regenera la rama
  `release/next` desde `main` con `prepare` y abre o actualiza la PR
  «Release X.Y.Z». Si nada está pendiente, cierra la que haya. Nunca empuja a
  `main`, nunca crea un tag, nunca mergea.
- **El aviso a la web con la App**: `notify-web` acuña su token de la App,
  limitado a `meltemi-web` y a lanzar workflows, en lugar de leer
  `WEB_DISPATCH_TOKEN`.
- **Los documentos**: `docs/release.md` (el camino por PR, y el manual como
  respaldo), `docs/versionado.md` (las reglas después de 1.0 y las dos líneas
  de la propuesta) y la frase de `AGENTS.md` sobre quién escribe la versión.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisitos «La versión siguiente la propone la
  integración continua», «Un hito probado de punta a punta sube el minor», «La
  preparación para 1.0 se calcula» y «Reglas de versión después de 1.0».

## Impact

- Archivos: `core/meltemi-release/src/{lib.rs,main.rs}` y sus tests,
  `.github/workflows/release-pr.yml` (nuevo), el job `notify-web` de
  `.github/workflows/release.yml`, `core/meltemid/tests/release.rs`,
  `docs/release.md`,
  `docs/versionado.md`, `AGENTS.md`, `CHANGELOG.md` (lo escribe la
  herramienta, en la rama de la PR).
- Una dependencia nueva de CI: `actions/create-github-app-token`, pineada por
  commit y justificada en el design (D7). Ningún crate nuevo: los diffs se leen
  con `git show <tag>:<ruta>` y `serde_json`, que ya están.
- Lo que el mantenedor pone: la App instalada en este repositorio (contenido
  y PRs) y en meltemi-web (Actions), y su id y su clave en los entornos
  `release-pr` y `release`, que declara el Terraform de meltemi-web
  (`ide-delivery-infra`). Hasta entonces el
  workflow lo dice en su resumen y no falla.
- Riesgo: una PR que propone un número equivocado. Por eso el porqué de cada
  número va en el cuerpo de la PR, cada señal se prueba con fixtures, y el
  merge sigue siendo del mantenedor.

## Fuera de alcance

- Mergear la PR por su cuenta, o un auto-merge con CI en verde: publicar sigue
  siendo un gesto humano (constitución §3).
- Publicar crates: `meltemi-proto` y `meltemi-client` siguen con
  `publish = false` (lo decide `contrato-publicado`).
- Escribir las notas a mano en la PR: cada línea sale de la propuesta de su
  change, y una nota mal escrita se arregla en la propuesta.
- Mover el número de versión fuera de los portadores que ya conoce
  `meltemi-release`.
