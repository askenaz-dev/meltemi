# QA — Costo de tamaño de los adaptadores ACP propios (2026-07-28)

Primera medición del costo real de empaquetar los dos adaptadores propios
(`meltemi-claude-acp`, `meltemi-codex-acp`) junto al daemon en el archivo por
plataforma, hecha al cerrar la tarea 4.6 de `adaptadores-propios-acp`. La
proposal decía «sin rustls el costo es moderado, pero se mide, no se supone»;
esto es la medición.

**Metodología**: build `release` del workspace en Windows 11 x86_64 (hardware de
referencia del mantenedor), tamaños en bytes de los binarios de
`target/release/`, y tamaño del archivo comprimido con `Compress-Archive
-CompressionLevel Optimal` (equivalente práctico del `7z a` que usa el job de
empaquetado) sobre el conjunto anterior y el nuevo.

## Resultado global: PASA

| Presupuesto (§12) | Valor | Veredicto |
|---|---|---|
| Adaptador ACP < 6 MB (nuevo gate) | `meltemi-claude-acp.exe` **3.15 MB** (3 299 840 B) | ✅ 48 % del techo |
| Adaptador ACP < 6 MB (nuevo gate) | `meltemi-codex-acp.exe` **3.16 MB** (3 313 152 B) | ✅ 48 % del techo |
| TUI < 25 MB | `meltemi.exe` 3.43 MB (3 594 752 B) | ✅ sin cambio atribuible |
| Instalador GUI < 15 MB | no afectado: el bundle de escritorio no transporta el daemon ni los adaptadores | ✅ |

## Costo del archivo por plataforma

| Archivo | Contenido | Comprimido | Δ |
|---|---|---|---|
| Antes | `meltemi` + `meltemid` | 4 709 599 B (4.49 MiB) | — |
| Después | + `meltemi-claude-acp` + `meltemi-codex-acp` | 7 154 918 B (6.82 MiB) | **+2 445 319 B (+2.33 MiB, +51.9 %)** |

Sin comprimir el conjunto pasa de 13.13 MiB a 19.44 MiB. Los dos binarios pesan
casi lo mismo entre sí porque comparten la librería de puente: lo que difiere es
el dialecto, no el esqueleto.

Un archivo que crece un 52 % merece decirse en voz alta y no esconderse en un
changelog. Lo que se compra a cambio: instalar Meltemi más el CLI oficial del
proveedor deja la entrada `ready`, sin `npm i -g` de terceros, que era el muro
de onboarding que `flota-deteccion-guia` diagnosticó.

## El gate nuevo y por qué en 6 MB

`MELTEMI_ADAPTER_BUDGET_BYTES = 6 291 456` (6 MiB) en `.github/workflows/release.yml`,
bloqueante en las tres plataformas como los demás presupuestos. El techo es
aproximadamente el doble del costo de hoy: suficientemente holgado para no
fallar ante crecimiento ordinario, suficientemente estrecho para que un binario
que de pronto enlace una pila que tiene prohibida no llegue a todos los
instaladores sin que nadie lo note. La prohibición categórica de esas pilas vive
en `deny.toml` (`[bans]`: rustls, native-tls, openssl, ureq, curl, isahc); este
gate es la medición que mantiene honesta esa afirmación — dos guardas
independientes sobre la misma propiedad, que es como se descubre que una de las
dos se rompió.

## Notas de honestidad de la medición

- `meltemi.exe` y `meltemid.exe` se midieron sobre los artefactos `release` del
  2026-07-26 presentes en el árbol: el proceso del daemon en ejecución tenía
  ambos archivos bloqueados durante la medición y no pudieron relinkearse. Los
  cambios de esta change sobre esos dos binarios son un puñado de funciones de
  detección y de composición de remedio; el delta no es material y no se afirma
  como cero, se declara como no medido. Los dos adaptadores —que son la cifra
  que esta medición existe para establecer— sí son del build de hoy de esta
  rama.
- Las cifras de macOS y Linux se publicarán con la primera release que corra el
  gate nuevo en sus runners, como se hizo con los instaladores de la GUI en
  `2026-07-25-gui-presupuestos.md`. Nada aquí está estimado para esas dos
  plataformas: simplemente todavía no está medido, y por eso no aparece.
- El instalador de escritorio (MSI/DMG/deb) no incluye ni el daemon ni los
  adaptadores hoy; su presupuesto de 15 MB queda intacto. Que la GUI dependa de
  un `meltemid` instalado aparte es una propiedad previa a esta change, no algo
  que esta change introduzca.
