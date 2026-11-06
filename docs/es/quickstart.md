<!-- SPDX-License-Identifier: Apache-2.0 -->

# Primeros pasos

[English](../quickstart.md)

De los archivos de la release a una sesión gobernada con el agente que ya tienes, y de ahí a tu
primera propuesta revisada, sin compilador ni configuración escrita a mano. Usa la aplicación de
escritorio para conversar y revisar, o la CLI para ejecutar comandos individuales y utilizar la
interfaz de terminal.

## 1. Instalar desde la release

Todos los enlaces resuelven a la última release publicada; no necesitas conocer un número de
versión.

| Plataforma            | Aplicación de escritorio                                                                                                     | Núcleo: daemon y cliente de terminal                                                                                                                                                                        |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows 10 1809+ / 11 | [`meltemi-desktop-Windows.msi`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Windows.msi) | [`install.ps1`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.ps1) o [`meltemi-Windows.zip`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Windows.zip) |
| macOS 13+             | [`meltemi-desktop-macOS.dmg`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-macOS.dmg)     | [`install.sh`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.sh) o [`meltemi-macOS.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-macOS.tar.gz) |
| Linux (Debian/Ubuntu) | [`meltemi-desktop-Linux.deb`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Linux.deb)     | [`install.sh`](https://github.com/askenaz-dev/meltemi/releases/latest/download/install.sh) o [`meltemi-Linux.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Linux.tar.gz) |

Los scripts de instalación son breves: descárgalos, léelos y después ejecútalos. Añaden `meltemi`,
`meltemid` y el alias `mel` a tu `PATH`, y rechazan una descarga cuya suma de comprobación no
coincida. Para comprobar un archivo por tu cuenta:

```bash
sha256sum --check --ignore-missing SHA256SUMS      # 1. it arrived intact
gh attestation verify meltemi-Linux.tar.gz --repo askenaz-dev/meltemi \
  --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml   # 2. it is the release's
```

En macOS usa `shasum -a 256`; en Windows, `Get-FileHash`. La guía de
[releases y verificación](release.md) explica qué demuestra cada comprobación y cómo verificar
sin conexión.

**El primer inicio de la aplicación de escritorio puede mostrar una advertencia.** El MSI y el
DMG todavía no tienen firma de plataforma, por lo que Windows SmartScreen y macOS Gatekeeper
piden confirmación antes de abrirlos. Las comprobaciones anteriores permiten saber si el archivo
es el que publicó la release. Las
[preguntas frecuentes](faq.md#el-instalador-avisa-de-que-no-tiene-firma) indican los pasos exactos.

## 2. Comprobar que funciona

```
meltemi version      # prints the client version, no daemon needed
meltemi help         # the full scriptable surface
meltemi status       # starts the daemon on demand and reports it
```

`version` y `help` son locales: nunca contactan con el daemon. `status` lo inicia bajo demanda
mediante un socket local; no se abre ningún puerto de red.

## 3. Tus agentes

Meltemi ejecuta el binario oficial del agente de programación que ya tienes, con la autenticación
que gestiona el propio agente: nunca lee ni guarda las credenciales de un agente.

```
meltemi fleet        # the agent catalog crossed with what is installed here
```

Cada fila indica si el agente está preparado y, si no lo está, qué capa falta y el comando exacto
que la instala. No hace falta configurar nada: el agente se indica con `--agent <id>` de esa
lista o se elige con su control en la aplicación de escritorio. Puedes vincular varias
suscripciones de un proveedor con `meltemi link <agent> <name>`. Los detalles de cada agente están
en la [guía de agentes](agentes.md).

## 4. Una sesión gobernada

```
cd your-project
meltemi session "summarise what this project does" --agent <id>
```

La sesión utiliza el proxy de permisos: cada herramienta que el agente solicita usar se consulta
aquí, en tu terminal, con las opciones del propio agente. Un dígito elige una, `d` deniega, `r`
aprueba y recuerda la regla, y Ctrl+C detiene la sesión. Todo queda en un registro al que sólo se
añaden entradas y se toma un punto de restauración antes del primer turno. La
[guía de sesiones libres](sesion-libre.md) explica qué gobierna una sesión y qué permite una
regla.

En la aplicación de escritorio, usa el compositor de inicio: selecciona el proyecto y el agente,
escribe la instrucción y envíala. Las solicitudes de permisos aparecen dentro de la conversación,
y la bandeja reúne todas las que siguen pendientes.

## 5. Tu primera propuesta

```
meltemi propose "add a dark-mode toggle to the settings page" --agent <id>
```

En este momento el proyecto recibe su carpeta `.meltemi/`: Meltemi crea la estructura del cambio
en `.meltemi/changes/` y delega la propuesta en tu agente, bajo el mismo proxy de permisos. Revisa
los cambios de especificación como una lista de comprobación:

```
meltemi review add-a-dark-mode-toggle-to-the-settings-page
```

## 6. El resto del ciclo

```
meltemi plan add-a-dark-mode-toggle-to-the-settings-page
meltemi implement add-a-dark-mode-toggle-to-the-settings-page <agent>
meltemi verify add-a-dark-mode-toggle-to-the-settings-page
meltemi archive add-a-dark-mode-toggle-to-the-settings-page
meltemi archive add-a-dark-mode-toggle-to-the-settings-page confirm
```

El primer `archive` muestra una vista previa; el segundo confirma aplicar los cambios a las
especificaciones vivas. Revisa antes la evidencia de los escenarios y las tareas.

Cada tarea se ejecuta en un árbol de trabajo aislado, con un punto de control previo y un commit
atómico que remite a su requisito.

## Accesibilidad y uso remoto

- `--json` produce salida legible por máquinas en los comandos que llaman al daemon.
- Se respetan `NO_COLOR` y la alternativa ASCII; consulta [accesibilidad](accesibilidad.md).
- Para el uso remoto, crea un túnel SSH del **socket local**: el daemon nunca escucha en la red.
  Consulta [acceso remoto](../acceso-remoto.md) y las [notas de plataformas](plataformas.md).

## Nota para Windows

En git-bash / MSYS, las variables de entorno que parecen rutas, como `MELTEMI_ENDPOINT`, pueden
alterarse. Consulta la solución en las [notas de plataformas](plataformas.md#windows-git-bash)
antes de configurarlas.

## Si algo falla

Empieza por [preguntas y resolución de problemas](faq.md).
