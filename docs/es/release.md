<!-- SPDX-License-Identifier: Apache-2.0 -->

# Descargas, instalación y verificación

[English](../release.md)

Instala Meltemi desde una release publicada en GitHub. Elige el instalador de escritorio para la
aplicación gráfica y el archivo del núcleo o el script de instalación para el daemon, el cliente
de terminal y los adaptadores incluidos. Consulta [primeros pasos](quickstart.md) para iniciar tu
primera sesión y [versiones](versionado.md) para conocer la información de origen registrada en
cada release.

## Dirección base de descarga

La dirección base de descarga es:

```
CANONICAL_DOWNLOAD_BASE=https://github.com/askenaz-dev/meltemi/releases
```

Los enlaces que usan `releases/latest/download/<stable-name>` resuelven a la última release
publicada. Usa la página de la propia release cuando necesites una versión concreta.

## Archivos distribuidos

| Plataforma | Archivo del núcleo     | Instalador de escritorio      |
| ---------- | ---------------------- | ----------------------------- |
| Windows    | `meltemi-Windows.zip`  | `meltemi-desktop-Windows.msi` |
| macOS      | `meltemi-macOS.tar.gz` | `meltemi-desktop-macOS.dmg`   |
| Linux      | `meltemi-Linux.tar.gz` | `meltemi-desktop-Linux.deb`   |

El archivo del núcleo contiene `meltemi`, `meltemid`, `meltemi-claude-acp` y `meltemi-codex-acp`.
Conserva los adaptadores en el mismo directorio que el daemon. Instala por separado la CLI
oficial del proveedor; la [guía de agentes](agentes.md) explica qué necesita cada integración.

El escritorio usa la vista web del sistema operativo. Windows instala WebView2 si hace falta;
macOS usa su vista web del sistema; el paquete Debian declara `libwebkit2gtk-4.1-0` y
`libgtk-3-0`. No se publica RPM ni AppImage. Consulta [plataformas](plataformas.md) para conocer
los sistemas compatibles.

Cada release incluye también:

- `install.sh` e `install.ps1`, los scripts de instalación del núcleo.
- `SHA256SUMS`, con las huellas usadas para verificar los archivos de la versión.
- `attestation.sigstore.json`, el archivo de atestación de la release.
- `release-manifest.json`, que registra la versión distribuida, el SHA exacto del código fuente
  y la CI aprobada.

## Verificar una descarga

Descarga el archivo y `SHA256SUMS` desde la misma release. Comprueba primero la suma y después
verifica el origen de la publicación si tienes instalada la CLI de GitHub:

```bash
# 1. Check the checksum: the file arrived intact
sha256sum --check --ignore-missing SHA256SUMS   # (shasum -a 256 on macOS; Get-FileHash on Windows)

# 2. Verify the signature: this repository's release workflow published this very file
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml
```

En el segundo comando, usa el nombre del archivo que descargaste. La atestación se refiere a los
archivos enumerados en `SHA256SUMS`; el propio archivo de sumas no es uno de los archivos
atestados. Los scripts de instalación comprueban la suma por ti, pero no verifican la atestación.

El punto de confianza es la identidad del workflow:
`askenaz-dev/meltemi/.github/workflows/release.yml`. No tomes ese punto de confianza ni ninguna
clave del texto de la página de una release: verifica contra el repositorio y el workflow que
esperas.

### Sin conexión a la red

Descarga una vez la raíz de confianza desde un equipo conectado y transfiérela, junto con el
archivo descargado y la atestación de la release, al equipo donde vas a verificar:

```bash
gh attestation trusted-root > trusted_root.jsonl

gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml \
  --bundle attestation.sigstore.json --custom-trusted-root trusted_root.jsonl
```

### Qué demuestra la atestación

La atestación es la firma de la release. El job `publish` firma el conjunto de archivos con la
identidad de su workflow; atesta la agrupación, no cada paso de compilación, y no atesta la
compilación de cada archivo individual. `release-manifest.json` identifica el código fuente
aprobado por CI, que puede diferir de la revisión del workflow registrada por la atestación de
publicación.

Una firma válida demuestra el origen y la integridad, no que el programa sea seguro o carezca de
defectos. Una cuenta con acceso de escritura al repositorio puede publicar una release cuya
verificación sea válida. Lee las notas de la release y comprueba el origen esperado además de la
firma.

El registro público de transparencia contiene los nombres y las huellas de los archivos y la
identidad del workflow que los publicó. Son metadatos de compilación, nunca datos de usuario ni
un registro de descargas.

## Instaladores

Descarga un script de instalación, léelo y después ejecútalo. Ambos scripts colocan juntos en
`PATH` el daemon, el cliente de terminal y los adaptadores incluidos, y crean el alias corto
`mel`:

- Unix: [install.sh](../../scripts/install.sh).
- PowerShell: [install.ps1](../../scripts/install.ps1).

Para una instalación manual, extrae el archivo verificado del núcleo, conserva sus binarios
juntos y añade ese directorio a `PATH`. Las cabeceras de los scripts describen los pasos manuales
equivalentes.

Después de instalar:

```bash
meltemi version
meltemi status
meltemi fleet
```

`status` inicia el daemon local cuando hace falta. Después de actualizar, detén el daemon
anterior con `meltemi stop`; el siguiente comando iniciará la versión instalada. El escritorio
y el daemon deben usar la misma release. `meltemi doctor` informa si las versiones difieren.

## Firma de plataforma

El MSI y el DMG todavía no tienen firmas de Windows Authenticode ni de la plataforma de Apple.
SmartScreen o Gatekeeper pueden mostrar una advertencia antes de abrirlos. Las sumas y la
atestación Sigstore verifican los archivos publicados; no eliminan esas advertencias de
plataforma. Las [preguntas frecuentes](faq.md#el-instalador-avisa-de-que-no-tiene-firma) explican
los controles disponibles para abrirlos después de verificar la descarga.

## Releases publicadas y compilaciones temporales

Instala desde las [releases publicadas](https://github.com/askenaz-dev/meltemi/releases), no desde
un artefacto de `.github/workflows/build.yml`. Un artefacto de integración no es una distribución:
caduca, no tiene una URL estable de última release e incluye `UNSIGNED-BUILD.txt` para indicar
que carece de firma de release. Su suma de comprobación sólo describe esa compilación temporal.

La CI aprobada de un push a `main` inicia la publicación automática de ese código fuente
exacto. Un tag puede existir antes de que termine el empaquetado; usa una release publicada con
sus archivos requeridos. Las notas automáticas describen cambios del código fuente y no afirman
que se haya ejecutado una prueba de aceptación manual.

## Releases con problemas

Una release publicada es inmutable. La recuperación consiste en publicar la siguiente versión
y señalar la problemática, con su fecha, en las notas de la release. Consulta esas notas antes
de instalar una versión anterior.

Para conocer las releases desaconsejadas, consulta los avisos vigentes en las notas publicadas.
