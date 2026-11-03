<!-- SPDX-License-Identifier: Apache-2.0 -->
# Activación de releases y recuperación de borradores — 3-nov-2026

Las PR #5 y #6 se integraron tras CI completa en Linux, macOS y Windows,
incluido cargo-deny. La primera incorpora también el parche compatible
async-executor 1.14.1: 1.14.0 había sido retirado del registry.

| Entrega | Fuente aprobada | CI push/main | Release workflow |
| --- | --- | --- | --- |
| v0.2.27 | dd83664ef47760bd495f4dd97d8c073dfdcd96ae | 41694685866 | 41697987181 |
| v0.2.28 | 44707e4b6de453edb804a686fb3c52defba17d6b | 41696658723 | 41699893823 |

Ambos pushes dispararon su workflow automáticamente, conservaron sus ejecuciones
en cola y reservaron tags distintos. Pasaron los gates del árbol estampado,
construyeron CLI e instaladores en las tres plataformas y emitieron atestaciones.

## Defecto encontrado y recuperación

La verificación previa a publicar buscaba el borrador por tag, pero ese endpoint
sólo devuelve releases publicadas. El primer diagnóstico como posible fallo
transitorio quedó descartado al listar los borradores: ambos contenían ya los
once assets. El defecto también afectaba a la búsqueda durante una recuperación.

Se verificaron por ID los borradores 433136791 y 433160936: fuente y CI, tag
anotado y manifiesto, once archivos no vacíos, todos los tamaños y SHA256 API,
nueve entradas SHA256SUMS y nueve subjects de la atestación del workflow
release.yml. Una revisión independiente confirmó la misma evidencia. La prueba
local del CLI Windows devolvió 0.2.27/0.2.28 y los MSI declararon esas versiones;
no se instalaron aplicaciones ni se probó aquí la GUI nativa.

Con autorización explícita se finalizaron esos IDs, sin reconstruir, sustituir
assets ni mover tags. GitHub confirmó immutable=true y los mismos IDs/digests de
assets. Los reintentos failed-only tomaron la ruta verifyExisting, terminaron
verdes en attempt 2 y emitieron el aviso a Web. Latest quedó en v0.2.28.

- [v0.2.27](https://github.com/askenaz-dev/meltemi/releases/tag/v0.2.27), publicada 20:04:27 UTC.
- [v0.2.28](https://github.com/askenaz-dev/meltemi/releases/tag/v0.2.28), publicada 20:06:26 UTC.

## Arreglo y límites de esta evidencia

La búsqueda ahora incluye el listado autenticado paginado; las comprobaciones
antes/después de publicar conservan el ID numérico del borrador. Rechaza
identidades duplicadas, tags/estados incorrectos y uploads incompletos. Los
diagnósticos HTTP incluyen sólo el status, sin stderr crudo ni secretos.

Verificación local: 17 pruebas Node de releases, 32 controles Rust de entrega y
2 pruebas dogfood del árbol de specs, todas verdes. Las pruebas nuevas modelan
el 404 real para drafts, paginación, publicación interrumpida y reintento de
publicación inmutable sin mutación. Revisión independiente sin nuevos blockers.

Las dos primeras releases requirieron finalización explícita por el defecto.
La CI del arreglo y una nueva publicación completamente automática siguen
pendientes al escribir esta evidencia; sus resultados se registrarán en la PR.
