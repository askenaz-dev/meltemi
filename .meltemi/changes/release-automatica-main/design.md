# Design

## D1 — Autorización y SHA

`workflow_run` escucha CI completado, filtra success/push/main/mismo repo y un
job con checkout de control en github.workflow_sha valida también la API de Actions contra el id de ci.yml.
`workflow_dispatch` desde main exige `ci_run_id` y el mismo control. Solo después
se hace checkout del head_sha validado. No se descargan artefactos de PR.

## D2 — Cola y reserva

CI y Release usan concurrency queue: max (100 pendientes; exceso cancelado
por GitHub). Release tiene grupo único. El orden de llegada a la cola no promete
orden de commits; latest se protege además por ancestría y versión.
Referencia: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#concurrency

El tag anotado contiene JSON con schema, sourceSha, version, ciRunId y baseTag.
Se crea mediante GitHub API (objeto + ref, nunca update/force). Antes de reservar,
se busca una reserva del mismo SHA. Se toma el tag first-parent más cercano
más distancia. Si la base declarada se introdujo después de ese tag, se toma el
mayor entre base más distancia y tag más distancia; si el tag es posterior a la
introducción, es autoritativo y no se recuentan commits previos a esa release.
La introducción de una base mayor recibe esa base exacta. Tags descendientes no
participan; huecos son aceptables. Una línea minor/major se declara explícitamente. Un tag
histórico sin metadata en el mismo SHA impide duplicar una publicación.

## D3 — Empaquetado y procedencia

Cada job hace checkout del SHA aprobado y estampa la misma versión con
meltemi-release set-version; el publish no compila. release-manifest.json viaja
bajo SHA256SUMS/atestación y declara la versión fuente, versión entregada,
sourceSha, tag y CI. El archivo fuente descargado de GitHub conserva la versión
base; para reproducir se aplica el estampado del manifiesto. La atestación del
workflow identifica la ejecución publicadora; el manifiesto ata esa ejecución
a la fuente validada, sin fingir que GITHUB_SHA de workflow_run es esa fuente.

## D4 — Reintentos y latest

El tag persiste si fallan gates o publicación. Se reconstruye desde el mismo
SHA y número, reutiliza el borrador y nunca modifica una release publicada.
latest avanza solo si la versión candidata es mayor y su fuente desciende de la
fuente de latest. Un commit antiguo puede completarse sin volverse latest.
Antes de omitir una release publicada se verifican once assets no vacíos, el
manifiesto contra la reserva y los digests API contra SHA256SUMS cuando están
disponibles. Un publicado incompleto falla sin mutación. Los artefactos internos
de Actions usan overwrite para rerun-all; failed-only conserva los de jobs ya
exitosos. Ningún asset público se reemplaza después de publicar.
Errores de API distintos de 404 fallan: no se interpretan como ausencia.

## D5 — Validación

Pruebas Node con repos git temporales y API falsa cubren CI falso, reservas,
reintentos tras fallos, tags históricos, notas de rango y latest antiguo.
Los controles Rust mantienen gates y artefactos. No requiere dependencias nuevas.
