<!-- SPDX-License-Identifier: Apache-2.0 -->

# Agentes: qué instalar, cómo se detectan y cómo configurarlos

[English](../agentes.md)

Meltemi no distribuye agentes ni lo hará: controla los binarios oficiales que ya tienes,
con la autenticación que cada uno gestiona por su cuenta (constitución §2, «juego limpio»).
Esta página explica, para cada agente, **qué instalar**, **qué busca Meltemi** y **qué hacer
cuando la vista Flota indica que falta algo**.

Empieza con `meltemi fleet`. Un resultado `ready` significa que se encontraron las capas
ejecutables necesarias; no inicia sesión en el agente ni demuestra que el modelo seleccionado
esté disponible.

## Las dos capas de un agente de nivel 2

Meltemi controla los agentes mediante Agent Client Protocol (ACP). Algunas CLI hablan ACP
por sí mismas (**nivel 1**); otras se controlan mediante un **adaptador** ACP (**nivel 2**).
Una entrada de nivel 2 tiene dos capas, y ambas deben estar presentes:

| Capa      | Qué es                                             | Quién se autentica           |
| --------- | -------------------------------------------------- | ---------------------------- |
| `cli`     | La CLI oficial del proveedor, tal como la instalas | La CLI, con tu propia cuenta |
| `adapter` | El puente ACP que Meltemi inicia                   | Nadie: delega en la CLI      |

Por eso, que la CLI del proveedor esté instalada pero la Flota diga que no se detecta no es
un error: el punto de control es el adaptador. La vista Flota indica qué capa falta y qué
hacer; **Meltemi nunca ejecuta un instalador por ti** (constitución §3: sin efectos externos
silenciosos).

### La capa del adaptador viene con Meltemi

En las entradas de nivel 2 del catálogo, el adaptador es **propio de Meltemi** y viene en
sus instaladores, junto al daemon. No tienes que instalar nada más para esa capa ni interviene
un paquete de terceros: instala Meltemi, instala la CLI oficial del proveedor y la entrada
queda `ready`.

Estos adaptadores controlan el binario oficial del proveedor como subproceso y nada más.
No enlazan un cliente HTTP ni una pila TLS, no escuchan en ningún puerto y nunca leen,
guardan ni reenvían tus credenciales: la CLI oficial se autentica sola, tal como cuando la
ejecutas manualmente (constitución §2).

Si la Flota informa de que falta esa capa, la instalación de Meltemi está incompleta:
**reinstala o repara Meltemi**. No se indica un comando de gestor de paquetes para ella
porque no hay un paquete que instalar, y Meltemi no inventará uno.

La detección informa de dónde encontró cada capa, para que puedas saber de dónde salió un
binario que no instalaste. En las interfaces verás `empaquetado con Meltemi` junto a un
binario de control encontrado al lado del daemon.

Estos son los estados combinados que puedes ver:

| Estado            | Significado                                                                                                    |
| ----------------- | -------------------------------------------------------------------------------------------------------------- |
| `ready`           | El punto de control está presente, y también la CLI oficial cuando la entrada declara una                      |
| `adapter_missing` | La CLI oficial está instalada, pero el adaptador ACP no. Si viene con Meltemi, reinstala o repara Meltemi      |
| `cli_missing`     | El adaptador está instalado, pero la CLI oficial no                                                            |
| `not_detected`    | No se encontró ninguna de las dos capas                                                                        |
| `not_launchable`  | Hay algo instalado, pero sólo como un script intermediario que no se puede iniciar; consulta Windows más abajo |

## Niveles de integración

| Nivel | Mecanismo                                 | Qué ofrece                                                                                       |
| ----- | ----------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 1     | ACP nativo por stdio                      | Sesiones, eventos y solicitudes de permiso que el agente expone mediante ACP                     |
| 2     | Adaptador ACP                             | Funciones del agente expuestas mediante un adaptador; consulta las restricciones de cada entrada |
| 3     | Modo sin interfaz con salida estructurada | Figura en el catálogo, pero esta versión no lo inicia (`level_not_pilotable`)                    |
| 4     | Sólo artefactos                           | Proyección de contexto: el agente lee tus especificaciones del repositorio                       |

El nivel describe la integración configurada. No garantiza que cada herramienta que usa un
agente pida permiso a Meltemi. Lee las restricciones de la entrada y revisa la sesión efectiva.

## El catálogo

### gemini-cli — Gemini CLI

- Nivel 1 (ACP nativo). Paso de MCP: sí.
- Meltemi busca: `gemini`
- Instalación: consulta la documentación del proveedor.

### copilot-cli — GitHub Copilot CLI

- Nivel 1 (ACP nativo).
- Meltemi busca: `copilot`

### cursor-cli — Cursor CLI

- Nivel 1 (ACP nativo).
- Meltemi busca: `agent`

### kiro-cli — Kiro CLI

- Nivel 1 (ACP nativo).
- Meltemi busca: `kiro-cli`

### kilo-code — Kilo Code

- Nivel 1 (ACP nativo).
- Meltemi busca: `kilo`

### opencode — OpenCode

- Nivel 1 (ACP nativo), iniciado como `opencode acp`.
- Meltemi busca: `opencode`
- **Se inicia configurado para preguntar.** Por sí solo, OpenCode edita archivos, ejecuta
  comandos y accede a la red sin preguntar; lo que no pregunta nunca llega al proxy de
  permisos. Meltemi lo inicia con `OPENCODE_CONFIG_CONTENT` establecido en una configuración
  en línea que pone `edit`, `bash`, `webfetch`, `websearch`, `task` y `external_directory`
  en `ask`, para que cada una de esas acciones te llegue como solicitud de permiso.
  Leer, buscar y listar dentro del proyecto sigue sin preguntar: no cambia nada ni sale
  del equipo.
- Si ya estableces `OPENCODE_CONFIG_CONTENT` en tu entorno o en el `env` de un perfil,
  tu valor tiene prioridad y Meltemi no le añade nada. Para mantener OpenCode bajo control,
  añade esto a tu propio valor:

  ```json
  {"permission":{"edit":"ask","bash":"ask","webfetch":"ask","websearch":"ask","task":"ask","external_directory":"ask"}}
  ```

  Un `agent.command` literal tampoco recibe esta configuración: se inicia exactamente
  lo que escribiste.

- Tu propio modelo, ya esté en tu equipo, tras un único endpoint para varios proveedores
  o a través de un agregador, se configura en el propio OpenCode: las recetas verificadas
  y sus perfiles están en [la guía de modelos locales](local-models.md).

### claude-code — Claude Code

- Nivel 2 (adaptador ACP propio de Meltemi). Paso de MCP: sí.
- Capas: CLI oficial `claude`, adaptador `meltemi-claude-acp`
- Instalar la CLI: `npm i -g @anthropic-ai/claude-code`
- El adaptador no requiere instalación adicional: `meltemi-claude-acp` viene en los
  instaladores de Meltemi, junto al daemon. Si falta, reinstala o repara Meltemi.
- **Estado legal declarado: zona gris.** El adaptador de Meltemi controla la CLI oficial
  con la sesión que ya iniciaste: la vía segura, nunca el SDK de agentes del proveedor.
  Sigue en zona gris porque el proveedor no ha publicado una postura sobre la orquestación
  de esa CLI por terceros; si aparece, la nota la citará. Meltemi muestra esta nota en la
  vista Flota.
- Reglas de permisos: cada solicitud lleva la entrada de la herramienta y el archivo
  que nombra, por lo que una regla por comando o por ruta la resuelve sin preguntarte.
  Una regla que permite un comando no permite otro comando encadenado, conectado por
  una tubería o redirigido después de él; ese otro sigue preguntando.
- Controles por sesión mediante argumento de inicio: `model`. El adaptador no tiene una
  vía verificada para establecer el esfuerzo, por lo que solicitarlo se rechaza antes de
  abrir la sesión; consulta [modelo y esfuerzo por sesión](#modelo-y-esfuerzo-por-sesión).

### codex-cli — Codex CLI

- Nivel 2 (adaptador ACP propio de Meltemi).
- Capas: CLI oficial `codex`, adaptador `meltemi-codex-acp`
- Instalar la CLI: `npm i -g @openai/codex`
- El adaptador no requiere instalación adicional: `meltemi-codex-acp` viene en los
  instaladores de Meltemi, junto al daemon. Si falta, reinstala o repara Meltemi.
- Estado legal declarado: tolerado. El adaptador controla el modo app-server documentado
  de la propia CLI, publicado por el proveedor para clientes de terceros, y la CLI se
  autentica sola.
- Versión mínima de la CLI: **0.156.1**, la versión con la que se verificó el adaptador.
  Una CLI anterior se rechaza y se indica actualizar a 0.156.1 o posterior
  (`npm i -g @openai/codex`). Con una versión más nueva, inferior a 1.0.0, la sesión se abre
  e informa antes del primer turno de que usa una versión posterior a la verificada; la
  Flota muestra la versión con la que se comprobó el nivel verificado. Desde 1.0.0 en
  adelante, la sesión se rechaza hasta actualizar Meltemi.
- La condición que pide Meltemi: en cada hilo, política de aprobación `untrusted`,
  entorno aislado `read-only` sin red y todas las aprobaciones enviadas al proxy de
  permisos de Meltemi. Si Codex responde con cualquier otra configuración, porque su
  propio `config.toml`, un perfil o un archivo de requisitos administrados la sustituye,
  la sesión se rechaza y el mensaje de solución nombra esos ajustes. También se rechaza
  un Codex con el control remoto activado: otro cliente podría responder a sus aprobaciones.
- Lo que esa condición no cubre, y que la sesión explica al abrirse: los comandos que
  Codex considera de sólo lectura, que ejecuta en su entorno aislado de sólo lectura;
  la búsqueda web del propio proveedor; y las llamadas a servidores MCP de la configuración
  de Codex que éste no somete a aprobación. La sesión nombra cada uno de esos servidores
  cuando Codex lo inicia.
- Reglas de permisos: una aprobación lleva el comando o el archivo de un cambio que
  sólo afecta a uno, tal como lo mostró la sesión, para que una regla por comando o por
  ruta la resuelva. Un cambio de varios archivos no lleva una única ruta, y un acceso
  a la red o una entrada a una terminal en ejecución no lleva el comando: esas acciones
  te preguntan.
- El nombre que usa Meltemi: el adaptador se presenta ante la CLI como `meltemi-codex-acp`,
  y la CLI informa de ese nombre al backend del proveedor como `originator` de sus
  solicitudes. Meltemi nunca se presenta como uno de los clientes del proveedor. La CLI
  etiqueta los hilos que abre con un `source` que ella misma elige; Meltemi no lo establece.
- Controles por sesión mediante argumentos de inicio: `model`, `effort`; consulta
  [modelo y esfuerzo por sesión](#modelo-y-esfuerzo-por-sesión).

### aider — Aider

- Nivel 3 (modo sin interfaz con salida estructurada), no iniciado por esta versión
  (`level_not_pilotable`).
- Meltemi busca: `aider`

### antigravity — Antigravity

- Nivel 4 (sólo artefactos): Meltemi proyecta tu contexto en `AGENTS.md` y el agente lo
  lee. No se controla ningún proceso.
- Meltemi busca: `agy`

## Límites de la verificación

El catálogo describe la integración que inicia Meltemi. No es una prueba en vivo del servicio,
la cuenta ni el modelo actuales del proveedor. Ejecuta la CLI oficial en el mismo proyecto si
fallan la autenticación, la cuota o la selección de modelo; después vuelve a abrir la Flota y
prueba una sesión pequeña.

Las pruebas de Meltemi cubren sus propios adaptadores y el comportamiento del protocolo. Las
ejecuciones registradas con agentes reales son evidencia histórica, no una garantía sobre una
CLI más nueva. La configuración de permisos que Meltemi añade a OpenCode aún no se ha comprobado
en una ejecución manual en vivo; verifica sus permisos efectivos antes de confiarle cambios.
Un agente que no envía una solicitud de permiso no puede ser detenido en ese punto por el
proxy de aprobaciones de Meltemi.

## Modelo y esfuerzo por sesión

Una sesión puede pedir un modelo y un esfuerzo: con `--model` y `--effort` en `meltemi session`,
los controles de modelo y esfuerzo del compositor de escritorio, o `model` y `effort` en un
perfil de inicio. Meltemi no mantiene un catálogo de modelos (constitución §5): los nombres
son los del proveedor y cada uno llega al agente por la vía que éste admite.

1. **Un argumento de inicio que declara su entrada.** El campo `levers` del catálogo enumera
   lo que el adaptador de una entrada lee del entorno que Meltemi prepara para él:
   `meltemi-codex-acp` acepta `model` y `effort`; `meltemi-claude-acp` acepta `model`. El valor
   se transmite sin modificar y no se valida aquí: el agente lo acepta o lo rechaza. Un
   control que la entrada no declara se rechaza antes de crear nada (error 2006), indicando
   el agente y el control.
2. **La opción que anuncia el agente.** Una entrada que no declara controles, es decir,
   todo agente ACP nativo y cualquier `agent.command` literal, se llame como se llame su
   binario, los recibe mediante las opciones que anuncia al abrir la sesión: la única
   opción de categoría `model` para el modelo y la única de categoría `thought_level`
   para el esfuerzo. Meltemi establece primero el modelo y después el esfuerzo entre las
   opciones que deja disponibles ese modelo, todo **antes del primer prompt**. Sólo envía
   el prompt si la última respuesta del agente mantiene exactamente lo solicitado y no
   cambió su modo.
3. **En cualquier otro caso se rechaza y no se envía ningún prompt.** Si no hay ninguna
   opción de esa categoría o hay más de una; si el valor no figura entre los anunciados
   (un interruptor sólo acepta `true` o `false`, los valores del propio agente); si la
   respuesta conserva otro valor o cambia el modo; o si no hay respuesta en el plazo de
   `[sessions] lever-wait` segundos (30 por defecto). La sesión termina como `lever_refused`,
   el comando devuelve el error 2007 con el control, lo solicitado, el motivo y lo que el
   agente anunció o dejó seleccionado, hasta veinte valores e indicando cuántos más hay.
   Todas las interfaces lo explican con palabras. Un prompt contiene tu repositorio, por
   lo que sólo se envía al modelo que elegiste.

Cancelar mientras se establece un control termina la sesión sin enviar un prompt. El historial
enumera lo que gobernó la sesión, y un control rechazado nunca figura como el modelo con el
que se ejecutó. `propose`, `explore`, `plan`, `constitution`, `dispatch` e `implement` aplican
los controles de un perfil del mismo modo, antes de crear nada.

La categoría es la clave de forma deliberada: ACP la presenta como dato de interfaz, pero
ningún otro dato permite distinguir «el modelo» sin adivinar por identificador o por valor.
Un agente que anuncia su selector sin categoría, o bajo dos categorías, se rechaza y se
muestran las opciones que anunció; establece la que quieras por su identificador en una
sesión activa con `meltemi set-option <session> <option-id> <value>`.

El tiempo que puede tardar el agente en responder a cada control se configura con `lever-wait`,
en segundos, dentro de `[sessions]` en el `.meltemi/config.toml` del proyecto o en tu
configuración de usuario; el mínimo que acepta es un segundo.

## Dónde busca la detección

Para cada capa, en este orden:

1. Cada directorio de `PATH`, por nombre de binario.
2. Las rutas candidatas conocidas de la entrada (`~/` se expande a tu directorio personal).
3. Para una capa que viene con Meltemi, el directorio del daemon en ejecución.

El orden es relevante: una copia que instalaste tiene prioridad sobre la que venía incluida.
Si compilas un adaptador de este repositorio y lo pones en tu `PATH`, Meltemi ejecuta ése y
la Flota te lo indica, para que no tengas que adivinar entre dos archivos con el mismo nombre.

No se ejecuta nada para detectarlo: la detección sólo consulta el sistema de archivos y se
repite en cada `fleet/list`, por lo que basta con instalar un agente y actualizar la vista.

### Windows

Los instaladores incluyen archivos `.exe`; los scripts intermediarios de npm y nvm son
`.cmd`, a veces `.bat`, y ésas son las extensiones que se pueden iniciar. A menudo también
existen como `.ps1`, que el sistema operativo **no puede iniciar directamente**: Meltemi
informa de un hallazgo que sólo contiene `.ps1` como evidencia de instalación con estado
`not_launchable`, en lugar de enviar al lanzador una ruta que fallará.

Si tu agente muestra `not_launchable`, reinstálalo para que esté presente su script
intermediario ejecutable; una instalación global de npm normalmente incluye ambos.

#### Finalizar un agente

Meltemi inicia los agentes dentro de un ámbito de procesos. Cancelar una sesión termina el
agente iniciado y sus procesos hijos. En Windows esto incluye el script `.cmd` y el proceso
que inicia; si no se puede establecer el ámbito de procesos, se rechaza el inicio.

Usa el control de cancelación de sesiones de Meltemi en lugar de cerrar un proceso hijo
arbitrario. Una conexión perdida y una sesión cancelada son estados diferentes; revisa el
resultado y el registro de la sesión.

### macOS y Linux

Una capa cuenta como encontrada cuando el archivo existe y tiene un bit de ejecución.

Si un agente que instalaste aparece como no detectado:

1. Comprueba que su directorio esté en el `PATH` del proceso que inició el daemon. Una
   configuración de shell que sólo exporta `PATH` en shells interactivos es la causa más
   habitual. `meltemi status` informa del endpoint del daemon; éste hereda el entorno de
   quien lo inició.
2. Comprueba el bit de ejecución: `ls -l $(command -v <binary>)`. Un archivo sin él se
   informa como no detectado, no como averiado.
3. Si la herramienta está fuera de `PATH`, por ejemplo en un gestor de versiones o en una
   instalación por proyecto, indícala explícitamente con `agent.command` en la configuración
   del proyecto en lugar de mover archivos.
4. Los enlaces simbólicos se siguen, por lo que un enlace al directorio de scripts
   intermediarios de un gestor de versiones funciona igual que un binario real.

## Varias cuentas y suscripciones

### Mediante vinculación (recomendado)

Cuando el catálogo declara la **variable de contexto de autenticación** de una entrada,
vincular una suscripción es una sola acción desde cualquier interfaz:

```
meltemi link claude-code personal
```

También puedes usar el formulario «Vincular suscripción» de la tarjeta del agente en la Flota
o el comando `link` del shell interactivo. La vinculación escribe un perfil de inicio cuyo
entorno fija esa variable en un directorio nuevo y vacío dentro del directorio de datos de
Meltemi, y responde con la **acción de inicio de sesión** que debes ejecutar una vez. El binario
oficial del proveedor se autentica allí por su cuenta; Meltemi nunca ejecuta el inicio de
sesión, nunca lee el directorio y nunca lo borra: desvincular lo deja allí, con su nombre.

Las variables son datos del catálogo, verificados con las CLI reales:

| Entrada       | Variable            | Acción de inicio de sesión          | Verificación                                                                                             |
| ------------- | ------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `claude-code` | `CLAUDE_CONFIG_DIR` | `claude` y ejecutar `/login` dentro | 2026-08-08: un contexto vacío informa de que no hay sesión, mientras el predeterminado sigue autenticado |
| `codex-cli`   | `CODEX_HOME`        | `codex login`                       | 2026-08-08, misma comprobación                                                                           |

Los perfiles vinculados viven en `subscriptions.toml`, junto a tu `config.toml`. Se gestionan
automáticamente y se cargan primero, por lo que un perfil escrito a mano con el mismo nombre
**tiene prioridad**, y `unlink` se niega a modificar los escritos a mano. Si dos perfiles de
un agente resuelven al mismo contexto, se informa explícitamente: es una suscripción con
dos nombres.

### Mediante configuración manual

Los perfiles de inicio permiten ejecutar varias cuentas del mismo proveedor, como dos
suscripciones de Claude, una laboral y otra personal, redirigiendo el **contexto de
autenticación** del binario oficial. Meltemi nunca lee, guarda ni reenvía credenciales;
el binario se autentica con el contexto que recibe.

En `.meltemi/config.toml`:

```toml
[[fleet.profile]]
name = "claude-personal"
agent = "claude-code"
env = { CLAUDE_CONFIG_DIR = "${CLAUDE_PERSONAL_DIR}" }

[[fleet.profile]]
name = "claude-work"
agent = "claude-code"
env = { CLAUDE_CONFIG_DIR = "${CLAUDE_WORK_DIR}" }
```

Antes de iniciar el daemon, establece `CLAUDE_PERSONAL_DIR` y `CLAUDE_WORK_DIR` con las rutas
completas de los directorios separados de configuración del agente. Los valores aceptan
referencias `${VAR}` que ocupen **todo el valor**, resueltas a partir del entorno del daemon
al iniciar el agente. Una referencia incluida dentro de una ruta más larga no se expande.
Se rechaza cualquier valor que parezca un secreto en texto plano.

Los perfiles aparecen como filas propias en la Flota, con origen `profile` y el agente que
inician, y seleccionas uno por sesión. Así, un proyecto puede ejecutar `claude-personal` y
`claude-work` al mismo tiempo en worktrees separados.

## Declarar un agente que Meltemi no conoce

```toml
[[fleet.custom]]
id = "my-agent"
name = "My Agent"
command = ["/usr/local/bin/my-agent", "acp"]
```

Una entrada personalizada declara que habla ACP: ése es el protocolo que controla el daemon.

## Usar un adaptador ACP de terceros

El catálogo incluye los adaptadores propios de Meltemi y ya no apunta a los de otros autores.
Es un cambio de recomendación, no una prohibición: existen adaptadores ACP de terceros, son
de código abierto y Meltemi controla uno como cualquier otra entrada en cuanto lo declaras.
El daemon no lo trata de forma distinta por ser de otro autor.

Instala el adaptador según las instrucciones de su publicador, decláralo y selecciónalo:

```toml
[[fleet.custom]]
id = "vendor-acp-community"
name = "Vendor ACP (third-party adapter)"
command = ["vendor-acp", "--acp"]

[agent]
id = "vendor-acp-community"
```

Para un uso puntual, `agent.command = ["vendor-acp", "--acp"]` en la configuración del proyecto
fija el comando literal y omite el catálogo por completo.

Antes de hacerlo, considera estos puntos:

- **El estado y la nota pertenecen a la entrada, no a cualquier adaptador.** El estado legal
  que muestra Meltemi para `claude-code` y `codex-cli` describe _la vía que Meltemi distribuye_.
  Un adaptador de terceros es otra vía con sus propios términos, y Meltemi no tiene una nota
  sobre ella: lee su licencia y los términos del proveedor.
- **Algunos adaptadores controlan un SDK de agentes del proveedor en lugar de su binario
  oficial.** Si los términos del proveedor restringen qué clientes pueden usar una
  suscripción de consumo, ésa es la distinción que debes revisar. Los adaptadores propios
  de Meltemi controlan el binario oficial con la sesión que ya iniciaste; por eso el catálogo
  sigue esa vía.
- **Esa capa deja de venir con Meltemi.** Debes seguir sus versiones, actualizaciones y
  cadena de suministro por tu cuenta.

Meltemi tampoco afirma que ningún proveedor apruebe su propia vía: la Flota muestra el estado
y la nota de cada entrada tal como los declara el catálogo. Si un proveedor publica una
postura, la nota se actualizará **con su fuente**, sin suponer una aprobación.

## Cuando algo sigue sin funcionar

- La vista Flota actualiza la detección en cada consulta: instala y vuelve a abrirla.
- El comando exacto que muestra Meltemi es un dato del catálogo; ejecútalo tú mismo.
- Un identificador cuya capa de control falta se rechaza al iniciar, con un diagnóstico
  que nombra la capa, sin cambiar silenciosamente a otro proveedor.
- Los problemas de autenticación pertenecen al binario del agente: Meltemi muestra su
  error literalmente, sin interpretarlo.
