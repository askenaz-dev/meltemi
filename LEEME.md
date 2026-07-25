<!-- SPDX-License-Identifier: Apache-2.0 -->
# Meltemi

**El plano de control spec-driven para el desarrollo agéntico.** Open source
(Apache-2.0), gratuito, de la comunidad. Meltemi orquesta los agentes de
codificación que ya usas —mediante estándares abiertos— bajo una disciplina
donde ninguna línea de código se escribe sin una especificación revisada.

> **Un rumbo, muchas velas.** Una spec clara impulsa cualquier número de agentes,
> de cualquier fabricante, sin atarte a ninguno.

**[meltemi.dev](https://meltemi.dev)** es el sitio del producto: qué es, el
método, los agentes que orquesta y todas las descargas.

_Read me in English: [README.md](README.md)._

## Qué es

Meltemi es un producto con dos superficies sobre tres sistemas operativos:

- **Una aplicación de escritorio** (Windows, macOS, Linux) — el plano de control
  visual: sesiones con transcripción en vivo, la bandeja de permisos, la flota de
  agentes, la vista de proyecto, la contabilidad local de consumo, revisión de
  diffs línea a línea y un editor de código utilitario.
- **Una aplicación de terminal** (`meltemi`) — la misma potencia en TUI más una
  CLI completamente scriptable, así que funciona por SSH en una máquina sin
  entorno gráfico.

Ambas son clientes finos sobre un daemon headless (`meltemid`) que concentra toda
la lógica. La regla es absoluta: **toda capacidad del daemon se alcanza desde
cualquier superficie** —esa es la paridad de núcleo, y la CI rompe el build si una
capacidad aterriza en un solo sitio.

Habla estándares abiertos: el Agent Client Protocol para pilotar agentes, MCP
para herramientas, LSP para inteligencia de código y JSON-RPC sobre un **socket
local únicamente** (sin puerto de red, jamás). Trae tu agente, tu clave y tu
modelo.

El flujo es spec-first: se propone un cambio, se revisan sus escenarios y solo
entonces se implementa —tarea a tarea, en worktrees de git aislados, con
checkpoints automáticos pre-tarea y un commit atómico por tarea que rastrea cada
línea hasta el requisito que la originó.

## Qué no es

Ni un editor de propósito general (la edición de código es utilitaria, al
servicio del bucle agéntico), ni otro agente, ni un servicio en la nube, ni
CI/CD, ni un marketplace. Sin cuentas, sin créditos, sin tarifas, sin lock-in y
sin telemetría.

## Instalación

### Opción A — los instaladores de la release (recomendado)

Los nombres de artefacto son estables y libres de versión, así que cada enlace
resuelve siempre a la **última release firmada**: nunca tienes que conocer un
número de versión.

| Plataforma | App de escritorio | Núcleo (daemon + terminal) |
|---|---|---|
| Windows 10 1809+ / 11 | [`meltemi-desktop-Windows.msi`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Windows.msi) | [`meltemi-Windows.zip`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Windows.zip) |
| macOS 13+ | [`meltemi-desktop-macOS.dmg`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-macOS.dmg) | [`meltemi-macOS.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-macOS.tar.gz) |
| Linux (glibc) | [`meltemi-desktop-Linux.AppImage`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Linux.AppImage) · [`meltemi-desktop-Linux.deb`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-desktop-Linux.deb) | [`meltemi-Linux.tar.gz`](https://github.com/askenaz-dev/meltemi/releases/latest/download/meltemi-Linux.tar.gz) |

El archivo del núcleo trae ambos binarios: el daemon `meltemid` y el cliente de
terminal `meltemi`. El instalador de escritorio trae la app de escritorio y usa
la vista web de tu propio sistema (en Windows arranca el runtime del sistema si
falta), así que se mantiene por debajo de 15 MB.

Cada release publica `SHA256SUMS` con firma separada. Verifica antes de instalar:

```bash
sha256sum --check SHA256SUMS
```

(`shasum -a 256` en macOS; `Get-FileHash` en Windows.)

Los scripts instaladores colocan `meltemi`, `meltemid` y el alias corto `mel` en
tu `PATH`. Son cortos y auditables, publican su hash dentro del `SHA256SUMS`
firmado, verifican lo que descargan y rehúsan seguir si algo no coincide: **aquí
no hay `curl | sh` a ciegas —descarga, lee y entonces ejecuta.**

```bash
# macOS y Linux
curl -fsSLO https://github.com/askenaz-dev/meltemi/releases/latest/download/install.sh
less install.sh
sh install.sh
```

```powershell
# Windows
irm -OutFile install.ps1 https://github.com/askenaz-dev/meltemi/releases/latest/download/install.ps1
notepad install.ps1
./install.ps1
```

El equivalente manual está escrito al inicio de cada script y en
[`docs/release.md`](docs/release.md). En el navegador, todo lo anterior está en
[meltemi.dev/es/downloads.html](https://meltemi.dev/es/downloads.html).

### Opción B — desde el código

Necesitas el toolchain de Rust pineado (`rust-toolchain.toml` lo selecciona) y,
para la app de escritorio, Node 24+.

```bash
git clone <este repositorio>
cd meltemi
cargo build --release -p meltemid -p meltemi
```

La app de escritorio la construye la CLI de Tauri, **no** un `cargo build`
pelado: el frontend se embebe en tiempo de compilación y solo `tauri build` lo
mete ahí (un cargo build a secas produce un binario cuya ventana no puede cargar
su interfaz).

```bash
npm ci --prefix desktop/ui
cd desktop && ui/node_modules/.bin/tauri build             # instaladores + binario
cd desktop && ui/node_modules/.bin/tauri build --no-bundle # solo el binario
```

`tauri build` corre la compilación del frontend por su cuenta
(`beforeBuildCommand`), así que no hace falta un `npm run build` aparte.

En Linux la app de escritorio necesita los paquetes de desarrollo del motor web
del sistema:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### Comprueba que funcionó

```bash
meltemid --version        # el daemon
meltemi status            # arranca el daemon si hace falta y reporta su estado
meltemi fleet             # qué agentes ve Meltemi, y qué instalar si falta alguno
```

Si `meltemi fleet` muestra un agente como no detectado, nombra la capa que falta
y el comando exacto que la instala. Empieza por ahí, o lee
[la guía de agentes](docs/agentes.md).

## Primer paso

```bash
meltemi propose "añade un interruptor de modo oscuro a ajustes"
```

Luego sigue el [quickstart](docs/quickstart.md).

## Estado

Fase 1 completa: el daemon, el motor de specs, el proxy de permisos, la
orquestación por worktrees, los checkpoints, los commits por tarea y el ciclo SDD
(`propose → plan → review → verify → archive → implement`) están implementados y
probados en Windows, macOS y Linux —**Windows es de primera clase**—. La fase 2
está en curso: el cliente de escritorio se está puliendo y sus instaladores salen
en cada release. Ver [`docs/plan-de-cambios.md`](docs/plan-de-cambios.md).

## Documentación

- [Quickstart](docs/quickstart.md), [Arquitectura](docs/arquitectura.md),
  [Método SDD](docs/metodo-sdd.md), [Referencia CLI](docs/referencia-cli.md),
  [Agentes](docs/agentes.md), [Paridad de núcleo](docs/paridad-nucleo.md),
  [Accesibilidad](docs/accesibilidad.md), [Plataformas](docs/plataformas.md),
  [Releases](docs/release.md).
- [Contribuir](CONTRIBUTING.md) · [Gobernanza](GOVERNANCE.md) · [Seguridad](SECURITY.md)

## Licencia

Apache-2.0, para siempre (constitución §12).
