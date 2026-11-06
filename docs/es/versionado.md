<!-- SPDX-License-Identifier: Apache-2.0 -->

# Versiones e identidad del código fuente

[English](../versionado.md)

Meltemi usa **SemVer, anterior a 1.0**, con una única versión distribuida para el daemon, el
cliente de terminal, el escritorio y los adaptadores incluidos. Ejecuta `meltemi version` para
ver la versión del cliente de terminal instalado. `meltemi status` y `meltemi doctor` ayudan a
identificar un daemon de una instalación anterior.

## Qué significa el número de una release

Una ejecución de CI aprobada para un push a `main` inicia una nueva release de ese commit de
origen exacto. Los cambios habituales incrementan automáticamente la versión patch. El orden de
las versiones sigue la historia del código fuente, no el orden en que terminan las
compilaciones.

Un tag puede estar reservado mientras una compilación sigue en curso o ha fallado. Un salto en
la numeración no significa que falte un archivo en tu instalación. Descarga una **release
publicada**, con su manifiesto y sumas de comprobación, en vez de confiar sólo en un tag.

## Código fuente y versión distribuida

`release-manifest.json` registra la versión distribuida en `version`, el SHA de origen en
`sourceSha`, la versión declarada en `sourceVersion`, la ejecución de CI y el tag. El archivo de
código fuente puede conservar una versión base declarada anterior: el empaquetado estampa la
versión asignada a la release en su copia de compilación aislada. Esto no significa que los
binarios publicados procedan de otro código fuente.

Para identificar el código de una descarga, inspecciona ese manifiesto y verifica la atestación
del archivo. La [guía de descargas](release.md) ofrece los comandos de comprobación de sumas y
firmas.

## Compatibilidad

Antes de 1.0, un cambio incompatible requiere incrementar la versión **minor**; después de 1.0,
requiere incrementar la versión major. La compatibilidad abarca:

- `proto/`: métodos, campos y esquemas públicos entre cliente y daemon.
- **Gramática CLI**: comandos, opciones, argumentos y códigos de salida.
- **Formato de artefactos**: especificaciones del proyecto, cambios y registros en disco.

Lee las notas de la release antes de una actualización que cambie la versión minor. Compartir
un número de versión no significa que todos los agentes o proveedores externos admitan las
mismas funciones; consulta [agentes](agentes.md).

## Inmutabilidad y reintentos

Los archivos y tags publicados no se sustituyen. Una publicación fallida se reintenta con la
misma versión reservada y el mismo código fuente. Recuperar una compilación anterior no hace
retroceder `latest`. Una corrección de una release ya publicada se distribuye como otra versión.
