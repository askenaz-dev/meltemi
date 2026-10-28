<!-- SPDX-License-Identifier: Apache-2.0 -->
# La prueba de un desarrollador

Una tarde, una persona, sus agentes. Este guion lleva a quien lo sigue desde los
assets de una release publicada hasta el ciclo entero de una change sobre un
repositorio de prueba, con los agentes que ya usa y paga, y termina en un
informe que acompaña a la release. No hace falta leer el repositorio ni
compilar nada.

Cada paso dice **qué haces**, **qué esperas ver**, **dónde queda** registrado y
la **desviación conocida** que puede aparecer. Una desviación conocida no es un
fallo de la prueba: es un hueco que ya tiene dueño, y se anota como tal. Lo que
no esté previsto aquí es un hallazgo, y es para lo que existe la prueba.

Está escrito para **Windows** primero. Lo que cambia en macOS y Linux está en
[su sección](#macos-y-linux). La documentación CI comprueba que cada verbo de
`meltemi` que aparece aquí existe en el binario, y corre los pasos que no
necesitan agentes.

## Antes de empezar

- Una máquina tuya, con los agentes que ya usas instalados y con su propia
  sesión iniciada: por ejemplo Claude Code (`claude`), Codex (`codex`) u
  OpenCode (`opencode`). Meltemi nunca lee ni guarda sus credenciales.
- Un **repositorio de prueba desechable**, nunca uno de trabajo: una carpeta
  nueva con `git init`, un archivo de código pequeño y un primer commit.
- `gh`, la CLI de GitHub, si quieres verificar la firma de la descarga.
- Anota antes de empezar: la versión de la release, el sistema, y la versión de
  cada agente (`claude --version`, `codex --version`, `opencode --version`).

## 1. Instalar desde la release

**Qué haces.** Descarga de la [última release](https://github.com/askenaz-dev/meltemi/releases/latest)
`meltemi-desktop-Windows.msi`, `install.ps1` y `SHA256SUMS`. Comprueba la
descarga y luego instala: el MSI con doble clic, y el núcleo con el script.

```
Get-FileHash .\meltemi-desktop-Windows.msi -Algorithm SHA256
gh attestation verify meltemi-desktop-Windows.msi --repo askenaz-dev/meltemi --signer-workflow askenaz-dev/meltemi/.github/workflows/release.yml
.\install.ps1
```

**Qué esperas ver.** El hash coincide con su línea de `SHA256SUMS`; `gh`
confirma que lo publicó el workflow de release de este repositorio. El script
instala `meltemi`, `meltemid`, `mel` y los dos adaptadores propios en
`%LOCALAPPDATA%\Programs\meltemi` y lo pone en el `PATH`.

**Dónde queda.** En nada de Meltemi todavía: anota los dos resultados.

**Desviación conocida.** Al abrir el MSI, **SmartScreen avisa** de que protegió
el equipo: el instalador no lleva firma de plataforma porque eso exige
certificados comprados. *Más información* → *Ejecutar de todas formas*. Es
esperado y está escrito en la [FAQ](faq.md#the-installer-warns-that-it-is-not-signed).

## 2. Lo primero: el diagnóstico

**Qué haces.** Abre una terminal nueva, para que tome el `PATH`, y:

```
meltemi version
meltemi doctor
```

**Qué esperas ver.** La versión de la release. `doctor` da una fila por cosa:
el cliente, el daemon, el endpoint, cada agente; cada una con su estado, sus
datos y lo que hay que hacer si falla. Nunca arranca un daemon ni ejecuta un
agente, y sale con `14` si alguna fila falla.

**Dónde queda.** En pantalla. Si algo falla, `meltemi doctor --json` es lo que
se copia en el informe.

**Desviación conocida.** Antes del primer `status` el daemon no está corriendo y
`doctor` lo dice: no es un fallo, `meltemi status` lo arranca.

## 3. Tu flota

**Qué haces.**

```
meltemi status
meltemi fleet
```

**Qué esperas ver.** `status` arranca el daemon por un named pipe local —ningún
puerto de red— y lo reporta. `fleet` cruza el catálogo con lo instalado: tus
agentes como *ready*, con su nivel, y los que faltan con la capa que falta y el
comando exacto que la instala.

**Dónde queda.** El daemon escribe su propio log en la carpeta de datos
(`%APPDATA%\meltemi\data\`).

**Desviación conocida.** Ninguna.

## 4. Una suscripción vinculada (opcional)

**Qué haces.** Si tienes dos cuentas de un mismo proveedor:

```
meltemi link claude-code trabajo
```

**Qué esperas ver.** Crea el perfil de lanzamiento `trabajo` e **imprime** el
gesto de inicio de sesión que tienes que correr tú, con el binario oficial.
Meltemi no inicia sesión por ti.

**Dónde queda.** En tu `config.toml` global, como `[[fleet.profile]]`.

**Desviación conocida.** Ninguna; si no tienes dos cuentas, el informe dice «no
corrido».

## 5. Una sesión gobernada en la terminal

**Qué haces.** En el repositorio de prueba:

```
meltemi session "resume qué hace este proyecto" --agent claude-code
meltemi session "crea NOTAS.md con una línea que diga hola" --agent claude-code
```

En la segunda, cuando el agente pida escribir, pulsa `r`: aprueba y recuerda la
regla más precisa, que es esa herramienta con esa ruta exacta. Confírmala con
`y`. Luego borra `NOTAS.md` y repite la segunda sesión: es la misma ruta.

**Qué esperas ver.** Cada permiso se pregunta en la terminal con lo que el
agente mandó —herramienta, título, comando o ruta— y sus propias opciones. La
regla que guardas dice a qué se aplica. En la repetición, la regla decide y no
se pregunta. Una regla por comando nunca cubre otro comando encadenado detrás
(`;`, `&&`, `|`): ese vuelve a preguntar, y es lo diseñado.

**Dónde queda.**

```
meltemi sessions
meltemi log <sesión>
```

`log` muestra el registro apend-only: `permission_requested` y
`permission_decided` con quién decidió (tú o la regla).

**Desviación conocida.** Ninguna para Claude Code ni Codex desde
`adaptadores-con-rawinput`: si una regla por comando o por ruta no casa, es un
hallazgo.

## 6. OpenCode pregunta antes de editar

Solo si usas OpenCode.

**Qué haces.**

```
meltemi session "crea OPENCODE.md con una línea" --agent opencode
```

**Qué esperas ver.** El aprobador pregunta **antes** de que exista el archivo.
Meltemi lanza OpenCode con una configuración que pone en `ask` editar,
ejecutar comandos, ir a la red y tocar rutas fuera del proyecto.

**Dónde queda.** En el `log` de la sesión, como `permission_decided`.

**Desviación conocida.** Es una verificación que CI no puede hacer: si OpenCode
edita sin preguntar, **anótalo como fallo de este paso**, con su versión. Si ya
tienes `OPENCODE_CONFIG_CONTENT` en tu entorno, gana el tuyo y este paso mide
otra cosa ([guía de agentes](agentes.md)).

## 7. El escritorio

**Qué haces.** Abre Meltemi desde el menú Inicio. Elige el repositorio de prueba
con el chip de proyecto, escribe una instrucción que pida editar un archivo y
envíala.

**Qué esperas ver.** La primera vez, el chip de agente dice cuántos agentes
detectó («N detectados»), y no lo vuelve a decir. Las peticiones de permiso
aparecen dentro de la conversación, y la bandeja guarda las que esperan; la
regla del paso 5 decide sola lo que cubre. La miga dice «Nueva sesión» mientras
compones.

**Dónde queda.** La sesión aparece también en `meltemi sessions`: la terminal y
el escritorio hablan con el mismo daemon.

**Desviación conocida.** El transcript pinta las llamadas a herramienta como
píldoras «título · estado», sin el resultado ni el diff
(`herramientas-a-la-vista`, propuesta).

## 8. El ciclo de una change

**Qué haces.** En el repositorio de prueba, una idea pequeña y real:

```
meltemi propose "añade una función farewell junto a greet" --agent claude-code
meltemi plan <change>
meltemi review <change>
```

La decisión de revisión no tiene verbo en la CLI: ábrela en la vista
**Proyecto** del escritorio, o en `meltemi` sin argumentos, comenta un requisito
para forzar una reelaboración y apruébala cuando esté resuelta. Después, dos agentes sobre la misma tarea:

```
meltemi assign <change> 1.1 claude-code,codex-cli
meltemi dispatch <change> 1.1 claude-code
meltemi dispatch <change> 1.1 codex-cli
meltemi race <change> 1.1
meltemi worktrees
```

Y para cerrar:

```
meltemi verify <change>
meltemi archive <change> confirm
```

**Qué esperas ver.** `propose` crea `.meltemi/changes/<change>/` y el agente
escribe la propuesta bajo el mismo proxy de permisos. `review` lista los deltas
como checklist. Cada `dispatch` corre en su propio worktree desde una base
común, con un checkpoint antes y un commit con trazabilidad después; `race`
muestra el diff de cada uno contra esa base. `verify` dice qué escenario está
cubierto y cuál no; `archive` pliega los deltas en `.meltemi/specs/`.

**Dónde queda.** `meltemi log <sesión>` de cada despacho, con la raíz que da
`meltemi sessions`. `git status` en el árbol principal **no** muestra los
worktrees de las calles.

**Desviación conocida.** Codex no se reanuda con `meltemi direct`
(`codex-reanudar-y-mcp`). Una regla que no le concedas a un despacho termina en
denegación: es lo diseñado, y el commit lo refleja.

## 9. El consumo

**Qué haces.**

```
meltemi usage week --all
```

**Qué esperas ver.** Las sesiones de la tarde, por agente.

**Dónde queda.** En ninguna parte nueva: se pliega de los registros de sesión,
en local. Nada sale de la máquina.

**Desviación conocida.** Claude Code y Codex **no reportan uso** por el
protocolo (`extension-acp-usage`): sus filas salen sin tokens ni costo, y
Meltemi no los estima.

## 10. Desde otra máquina (opcional)

**Qué haces.** Si tienes otra máquina con acceso SSH a esta, sigue
[acceso remoto](acceso-remoto.md). Lo esencial:

```
meltemi tunnel usuario@otra-maquina
```

**Qué esperas ver.** El comando `ssh` que reenvía el endpoint local de este
daemon a la otra máquina; con `--exec` lo corre. El daemon nunca escucha en la
red: lo remoto llega siempre por el túnel.

**Dónde queda.** Las sesiones que abras desde la otra máquina quedan en el
daemon de esta, como cualquier otra.

**Desviación conocida.** Ninguna; si no lo corres, el informe dice «no
corrido».

## 11. Desinstalar

**Qué haces.** Desinstala *Meltemi* desde *Configuración → Aplicaciones*, borra
`%LOCALAPPDATA%\Programs\meltemi` y quítalo del `PATH` de tu usuario. Antes,
`meltemi stop`.

**Qué esperas ver.** No queda ningún servicio ni tarea programada. **Quedan, a
propósito**, tus datos y tu configuración en `%APPDATA%\meltemi\` (registros de
sesión, reglas, perfiles) y la carpeta `.meltemi/` del repositorio de prueba:
son tuyos. Borra `%APPDATA%\meltemi\` si no los quieres.

**Dónde queda.** Anota lo que quedó en disco.

**Desviación conocida.** Ninguna.

## macOS y Linux

- **macOS**: `meltemi-desktop-macOS.dmg` y `install.sh`; el hash con
  `shasum -a 256`. **Gatekeeper** rehúsa abrir la app la primera vez:
  Control-clic → *Abrir*, o *Ajustes del Sistema → Privacidad y seguridad →
  Abrir igualmente*. Los datos y la configuración, en
  `~/Library/Application Support/meltemi/`.
- **Linux (Debian/Ubuntu)**: `meltemi-desktop-Linux.deb` con
  `sudo apt install ./meltemi-desktop-Linux.deb`, y `install.sh`; el hash con
  `sha256sum --check --ignore-missing SHA256SUMS`. El `.deb` no avisa de firma.
  Datos en `~/.local/share/meltemi/` y configuración en `~/.config/meltemi/`.
- En las dos, `install.sh` instala en `~/.local/bin`, y el daemon escucha en un
  socket Unix con permisos `0700`.

## El informe

Copia esta plantilla a `docs/qa/<fecha>-prueba-de-uno.md` y rellénala con lo que
viste. **Una fila que no se pudo ejercer no se escribe como aprobada**: se
escribe «no corrido» y por qué.

```text
# Prueba de un desarrollador — <fecha>

Release: <versión> · Commit: <sha del tag>
Plataforma: <sistema y versión>
Agentes: <agente y versión>, <agente y versión>, ...

| Paso | Resultado | Notas |
| --- | --- | --- |
| 1. Instalar desde la release | <ok / falla / no corrido> | <hash, attestation, aviso de firma> |
| 2. meltemi doctor | | <filas en warn o fail> |
| 3. Flota | | <agentes ready> |
| 4. Suscripción vinculada | | |
| 5. Sesión gobernada y regla recordada | | <la regla decidió la repetición> |
| 6. OpenCode pregunta | | <versión de OpenCode> |
| 7. Escritorio | | <saludo una vez, bandeja, regla> |
| 8. Ciclo de una change | | <change, despachos, race, verify, archive> |
| 9. Consumo | | |
| 10. Otra máquina | | |
| 11. Desinstalar | | <qué quedó en disco> |

## Desviaciones

<cada desviación conocida que apareció, con su change>

## Hallazgos

<lo que no estaba previsto, con lo que se vio y dónde quedó registrado>

## Veredicto

<se publica / se publica con hallazgos declarados / no se publica>
```

Con el informe escrito, la release lo enlaza en sus notas: ver
[releases](release.md#making-a-release-maintainer).
