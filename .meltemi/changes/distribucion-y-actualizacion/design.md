# Design — distribucion-y-actualizacion

## Context

La release publica, por plataforma, el archivo del núcleo (`meltemi-Windows.zip`,
`meltemi-macOS.tar.gz`, `meltemi-Linux.tar.gz`), el instalador de escritorio
(MSI, DMG, deb), los scripts de instalación, un `SHA256SUMS` y la atestación.
Los archivos de macOS y Linux son de una sola arquitectura: `macos-latest`
(arm64) y `ubuntu-latest` (x86_64). Después de `publish` corre `notify-web` en
el entorno `release`, y su fallo no deshace nada. El patrón está probado.

El núcleo necesita que los adaptadores (`meltemi-claude-acp`,
`meltemi-codex-acp`) estén en el mismo directorio que `meltemid`, porque es el
último lugar donde su detección mira (`docs/agentes.md`). Cada manifiesto tiene
que respetarlo.

Meltemi no tiene red: el daemon no abre sockets de red (§3), el webview no
alcanza orígenes remotos, y `deny.toml` prohíbe `reqwest`, `hyper`, `rustls`,
`native-tls`, `openssl` y `ureq`.

## Decisions

### D1 — Un job `distribute` tras `publish`, como `notify-web`

`distribute` depende de `publish`, corre en el entorno `release` y hace lo
siguiente:

1. Descarga el `SHA256SUMS` publicado y lo verifica con `gh attestation verify`
   antes de leer un solo hash. Los manifiestos salen de lo publicado y
   atestado, nunca de los artefactos del run.
2. Genera los manifiestos desde plantillas versionadas en `packaging/`
   (`packaging/homebrew/meltemi.rb.in`, `packaging/homebrew/meltemi-desktop.rb.in`,
   `packaging/scoop/meltemi.json.in`, `packaging/winget/*.yaml.in`), con la
   versión, las URLs por tag y los hashes. Plantillas y no código, para que un
   test las renderice con un `SHA256SUMS` de fixture y compare.
3. Publica cada manifiesto en su destino (D2, D3), cada uno en su propio paso
   con `continue-on-error: false`. Un gestor que falla no impide intentar los
   demás, y el job termina rojo nombrando cuál falló.

Un fallo no deshace la release. `distribute` se puede re-ejecutar solo, y cada
publicación es idempotente: si el destino ya tiene esa versión, no hace nada.
Sin la credencial de un destino, ese paso escribe un aviso en el resumen y se
salta.

### D2 — Homebrew y Scoop, con la App

- **El tap**. La decisión es del mantenedor:
  - `askenaz-dev/homebrew-meltemi` (`brew tap askenaz-dev/meltemi`), propio y
    limpio;
  - reutilizar `askenaz-dev/homebrew-tap` (`brew tap askenaz-dev/tap`), que ya
    existe con `forge.rb`, de otro proyecto suyo.

  **Recomendación**: el propio. Un tap compartido mezcla historiales y
  permisos de dos productos, y la App tendría escritura sobre la fórmula del
  otro.
- **La fórmula `meltemi`** (macOS arm64 y Linux x86_64, los archivos que
  existen) instala todos los binarios del archivo en `bin`, adaptadores
  incluidos. Así quedan junto a `meltemid` sin depender de cómo se resuelva un
  enlace simbólico. `mel` es un alias de la fórmula. En Intel macOS no se
  ofrece y la fórmula lo dice.
- **El cask `meltemi-desktop`** apunta al DMG. Homebrew está dejando de
  aceptar casks que Gatekeeper rechaza, así que el cask se genera solo si el
  `capabilities.json` de la release dice que el DMG lleva firma de plataforma
  (`firma-de-plataforma`). Hasta entonces el paso lo dice y se salta.
- **El bucket de Scoop**: `askenaz-dev/scoop-meltemi`, con `meltemi.json`
  (`bin` con `meltemi.exe`, `meltemid.exe` y `mel`; el archivo entero en el
  directorio de la app, con los adaptadores dentro).
- **La credencial** es un token de la GitHub App (`entrega-sin-huecos` D4),
  instalada en el tap y en el bucket con `contents: write`. El job empuja un
  commit directo: los repositorios son del proyecto y no hay a quién pedir
  revisión. El commit dice la versión y el `SHA256SUMS` del que salió.

### D3 — winget, con su propio token

`winget` se publica con una PR a `microsoft/winget-pkgs` desde un fork. La App
no está instalada ahí y no puede abrir esa PR, así que hace falta un token de
usuario con permiso para empujar al fork y abrir PRs en repositorios públicos.
Hay dos salidas:

| Opción | Qué es | Coste |
| --- | --- | --- |
| **Cuenta de máquina** (recomendada) | una cuenta de GitHub dedicada (por ejemplo, `meltemi-bot`), dueña del fork, con un token clásico de alcance `public_repo` y nada más | una cuenta que custodiar; GitHub permite una cuenta de máquina por persona |
| Token del mantenedor | el mismo alcance, sobre su cuenta personal | un token con su identidad en un secreto de CI |

La herramienta es `komac` (Rust, open source), pineada por versión y verificada
por checksum en el paso. Genera los manifiestos multi-archivo y abre la PR.
`wingetcreate`, la oficial, hace lo mismo y queda como alternativa si `komac`
no sirve.

Los paquetes son dos:

- **`askenaz-dev.Meltemi`**: el zip del núcleo como `zip` con instaladores
  `portable` anidados (`meltemi.exe` y `meltemid.exe`, con `mel` como alias).
  Que winget extraiga el archivo entero, con los adaptadores junto al daemon,
  se verifica en la tarea 2.3. Si no lo hace, el paquete del núcleo se aplaza y
  el de escritorio sale solo.
- **`askenaz-dev.MeltemiDesktop`**: el MSI.

La primera PR de cada paquete la revisa un moderador de Microsoft y tarda días.
Las siguientes suelen pasar con la validación automática. La página de
descargas lo dice.

### D4 — El cliente sabe cómo se instaló, por su ruta

Una función pura `install_channel(exe_path, env) -> Channel` en un módulo del
crate cliente, con tests por plataforma sobre rutas de fixture:

| Canal | Señal | Comando de actualización |
| --- | --- | --- |
| Homebrew | la ruta pasa por `Cellar/meltemi/` | `brew upgrade meltemi` |
| Scoop | la ruta pasa por `scoop\apps\meltemi\` | `scoop update meltemi` |
| winget | la ruta pasa por `WinGet\Packages\askenaz-dev.Meltemi_` | `winget upgrade askenaz-dev.Meltemi` |
| script | `~/.local/bin` o el directorio que escribe `install.ps1` | volver a correr el instalador de una línea |
| paquete del sistema | `/usr/bin` en Linux (el deb del escritorio) | `sudo apt install ./meltemi-desktop-Linux.deb` con la nueva |
| desconocido | lo demás (compilado desde el código, otra ruta) | «mira la página de releases» |

Para el escritorio, la misma función mira la ruta de la app: el cask en
`/Applications` con su marca en `Caskroom`, el MSI en `Program Files`. Un MSI
no distingue si lo instaló winget o una persona, así que se muestran los dos
comandos.

- **CLI**: `meltemi version --verbose` (y `--json`) imprime la versión, el
  canal y el comando. `version` sin la bandera no cambia.
- **GUI**: la línea «Acerca de» de Ajustes gana el canal y el comando,
  copiable, y un botón «Ver releases» que abre la página de releases con el
  abridor del sistema (el mismo mecanismo de «Abrir con…»), sin que la app
  toque la red.

Es lógica del cliente sobre su propio ejecutable: el daemon no gana capacidad,
y la paridad de núcleo no aplica. Las dos superficies la tienen igual por
cortesía, como en `abrir-con-lo-tuyo`.

### D5 — Ninguna comprobación de versión por red, por escrito

El requisito lo dice, y `deny.toml` ya lo hace cumplir. La propuesta lo escribe
para que nadie lo «arregle» con un cliente HTTP. La pregunta «¿hay versión
nueva?» la contesta el gestor del usuario (`brew outdated`, `scoop status`,
`winget upgrade`), y el cliente le dice cuál.

### D6 — Lo que va a meltemi-web

Son changes de allá, después de esta:

- el Terraform de los repositorios del tap y del bucket, la instalación de la
  App y el secreto de winget;
- la página de descargas con los comandos, cuando el primer manifiesto esté
  publicado.

`ide-delivery-infra` ya declara la App. Esos repositorios se suman ahí o en
una change propia, según el orden en que lleguen.

## Risks / Trade-offs

- **Un manifiesto roto** en el tap afecta a todos los usuarios de `brew`. El
  test de plantillas lo renderiza y corre `ruby -c` (la sintaxis) en CI. Un
  `brew audit` contra el tap real lo corre la tarea de verificación.
- **El token de winget** es la única credencial de usuario de larga vida del
  camino. Tiene un solo alcance, es de una cuenta que no es la del mantenedor y
  entra al inventario de `firma-de-plataforma` D7.
- **La heurística de canal se equivoca** con una ruta rara. El peor caso es
  «desconocido», que remite a la página de releases.
