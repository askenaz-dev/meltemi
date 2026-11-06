<!-- SPDX-License-Identifier: Apache-2.0 -->
# Sesiones libres y permisos

[English](../sesion-libre.md)

Inicia una sesión libre cuando quieras trabajar con un agente en tu proyecto sin crear antes un
cambio ni una especificación. Elige el proyecto y el agente, envía una instrucción y sigue el
trabajo. La sesión usa la misma política de permisos y el mismo registro al que sólo se añaden
entradas que los demás flujos de agentes.

## Empezar desde el escritorio o la terminal

La pantalla inicial del escritorio contiene el compositor. Elige el proyecto y el agente o
perfil de ejecución, conserva el flujo en **Libre**, escribe la instrucción y envíala.
La conversación muestra la sesión, los eventos del agente y las solicitudes de permiso.
Una nueva instrucción se envía a la misma sesión mientras siga disponible; enviarla no cancela
el turno en curso.

```bash
meltemi session "tidy up the error handling in the parser"
meltemi session "same, but on another checkout" /path/to/project
meltemi session "..." --agent codex-cli
```

La CLI espera ese turno y termina con su resultado. Imprime el id de sesión, el punto de
restauración o la razón por la que no está disponible, y un aviso si se denegó algún permiso.
Las sesiones del escritorio pueden esperar más turnos; el comando de CLI no mantiene abierta
esa conversación después de su turno.

En la TUI usa `session` desde la paleta. `direct` envía otra instrucción desde el detalle de la
sesión seleccionada. También puedes usar:

```bash
meltemi direct <session> "add a test for the error case"
meltemi watch <session>
meltemi log <session>
```

Una instrucción puede quedar en cola detrás de un turno activo o reanudar una sesión terminada
si el agente admite reanudación. Si no puede hacerlo, Meltemi explica por qué. `watch` sigue el
registro; Ctrl+C deja de seguirlo sin cancelar la sesión. Usa `meltemi cancel <session>` para
cancelarla.

## Dónde trabaja el agente

Una sesión libre trabaja en la **raíz de tu proyecto**, incluidos tus cambios de trabajo.
No crea un árbol de trabajo aislado. Usa [el flujo de tareas SDD](metodo-sdd.md) si necesitas
árboles de trabajo gestionados. Revisa los cambios del agente antes de confirmarlos o descartarlos.

La sesión aparece junto a las demás. El registro conserva prompts, llamadas a herramientas que
el agente comunica, decisiones de permiso y resultados de turnos. Importa también la política del
propio agente: Meltemi sólo decide las peticiones que llegan a su proxy. Consulta [agentes](agentes.md).

## El punto de restauración

Antes del primer turno se toma una instantánea de un repositorio Git que ya tiene historia.
Meltemi muestra su referencia bajo `refs/meltemi/checkpoints/free/` y la lista con
`meltemi checkpoints` en el grupo `free`. Tomarla no mueve tu rama ni modifica el índice de Git.

Un punto de control de sesión libre **no ofrece revertir el árbol completo**. La reversión
habitual se limita a árboles de trabajo gestionados por Meltemi y rechaza la raíz de tu proyecto.
Para recuperar un archivo concreto, inspecciona la instantánea y usa deliberadamente la
restauración por ruta de Git:

```
git restore --source refs/meltemi/checkpoints/free/<session>-<agent> -- <path>
```

Esto reemplaza la ruta elegida. Conserva antes los cambios posteriores que quieras mantener.

La sesión puede iniciarse aunque no haya una instantánea disponible:

| Causa | Qué muestra | Siguiente paso |
|---|---|---|
| La carpeta no es un repositorio Git | `no restore point: this root is not a git repository` | Inicializa Git si quieres un historial de versiones |
| El repositorio no tiene commits | `no restore point: this repository has no history yet` | Crea el primer commit cuando corresponda |

## Elegir agente, modelo y esfuerzo

Usa `--agent <id|profile>` o el control de agente del compositor de escritorio. Sin selección,
Meltemi usa el agente configurado en el proyecto; si tampoco hay uno, informa los candidatos
disponibles para que la interfaz permita elegir.

```bash
meltemi session "..." --agent opencode --model anthropic/claude-haiku-4-5
meltemi session "..." --agent codex-cli --model <model> --effort high
```

Los nombres de modelo y esfuerzo pertenecen al proveedor. Meltemi los transmite mediante el
control de ejecución declarado del agente o la opción que éste anuncia al iniciar la sesión.
Un valor no admitido se rechaza antes del primer prompt en lugar de sustituirse en silencio.
Consulta [los controles del agente](agentes.md).

La opción de CLI `--mode manual|semi|autonomous` elige la política de autonomía de la sesión.
Es independiente de elegir Libre, Proponer o Explorar. Las reglas explícitas de denegación siguen
prevaleciendo y las acciones irreversibles necesitan aprobación: un modo no concede acceso
indiscriminado a las herramientas.

## Responder una petición de permiso

Con stdin y stderr conectados a una terminal, la CLI pregunta en stderr y deja stdout disponible
para `--json`. Muestra la herramienta, el título y los datos que informó el agente, además de
las opciones de ese agente:

```
permission requested by session 3f2c9a
  tool     execute
  title    Bash: cargo test
  content  {"command":"cargo test","description":"Run the tests"}
  1 Allow once  (allow once)
  2 Reject  (reject once)
  d deny   Ctrl+C stop the session
```

- Un dígito elige esa opción. Enter no elige nada. Una opción que evita nuevas preguntas del
  agente para peticiones parecidas exige confirmar otra vez con `y`.
- `d` o Esc deniega mediante el rechazo del agente, o cancela si no ofreció un rechazo.
- `r`, cuando se ofrece, aprueba una vez y recuerda una regla para la herramienta y el comando
  o ruta concretos. `y` confirma escribirla. Rige desde la siguiente sesión iniciada sin `--mode`.
- Ctrl+C cancela la sesión; el comando termina con `13`.

No se ofrece recordar cuando faltan un comando o ruta utilizables, para cambios de `.meltemi/`,
ni desde flujos de tareas como `implement` y `dispatch`. Una regla que permite un comando no
permite otro encadenado, canalizado, redirigido o sustituido después: esos casos siguen preguntando.

Las teclas escritas antes de poder leer la pregunta, o pegadas juntas, no la responden. Una
petición vencida o resuelta desde otra superficie se retira. En un script o tubería se deniegan
las peticiones sin resolver en vez de esperar entrada. Configura reglas explícitas en
`.meltemi/permissions.toml` del proyecto o en tu configuración de usuario para trabajo desatendido.

## Continuar con un cambio

El compositor de escritorio también ofrece **Proponer** y **Explorar**:

| Flujo | Comando CLI | Resultado |
|---|---|---|
| Libre | `meltemi session` | Trabajo en la raíz con los controles de sesión anteriores |
| Proponer | `meltemi propose` | Crear un cambio y pedir al agente que redacte su propuesta |
| Explorar | `meltemi explore` | Explorar el tema sin que el método escriba en el árbol |

Elegir el flujo no aprueba una propuesta ni modifica la constitución del proyecto. Consulta
[el método SDD](metodo-sdd.md) para tareas revisadas y [la referencia CLI](referencia-cli.md)
para los argumentos y códigos de salida.
