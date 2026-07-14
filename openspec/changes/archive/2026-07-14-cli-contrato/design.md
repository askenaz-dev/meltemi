## Context

`meltemid` expone un contrato JSON-RPC v1 sobre socket local. Hoy lo consume solo
`meltemi-devclient`, un binario de desarrollo sin gramática ni disciplina de
salida estables. El documento fundacional fija que la interfaz de terminal es un
**único binario `meltemi`** (alias `mel`) que es a la vez CLI scriptable y TUI
interactiva (§0, §7.2), con **paridad de núcleo** respecto de la futura GUI
(principio constitucional 4). El rumbo sitúa este binario en el crate `tui/`.

Esta change fija el **contrato** de esa superficie —lo verificable, lo que otras
superficies deben respetar para que la paridad sea real— y crea el crate `tui/`
con el modo scriptable. La TUI interactiva (paneles, navegación) es #6; aquí solo
se fija la regla que decide cuándo se entra en ese modo.

Restricciones heredadas: sin dependencias que no sean mínimas, pineadas y
justificadas (constitución); Windows es plataforma de primera clase; el contrato
`proto/` no se altera; cabecera SPDX en cada archivo.

## Goals / Non-Goals

**Goals:**
- Una gramática de subcomandos estable, con subcomandos del ciclo SDD **reservados**.
- Una regla de despacho CLI↔TUI determinista y verificable.
- Una taxonomía de códigos de salida estable y extensible.
- Disciplina estricta stdout/stderr, con `--json` apto para consumo por máquina.
- Un mapeo normativo comando↔método RPC, anclado a los métodos existentes.
- El crate `tui/` como miembro del workspace, compilando en las tres plataformas.

**Non-Goals:**
- La TUI interactiva real (paneles, navegación, onboarding): #6.
- Los cuerpos de los subcomandos del ciclo SDD (más allá de cablear `propose` al
  RPC ya existente): #14 y siguientes.
- Autocompletado de shell, manpages, empaquetado y alias a nivel de instalación: #22/#23.
- Cualquier transporte que no sea el socket local / túnel SSH ya existentes.

## Decisions

### D1 — Parser de argumentos hecho a mano, sin dependencia
La gramática es pequeña y estable (un puñado de subcomandos, tres flags globales).
Se implementa un parser propio en `tui/`, sin crate de terceros.
- **Por qué**: coherencia con la disciplina de dependencias mínimas de la
  constitución y con el precedente de `meltemi-spec` (cero dependencias);
  mantiene la auditoría `cargo-deny` trivial y el binario pequeño (objetivo §10:
  binario TUI < 25 MB). La detección de TTY usa `std::io::IsTerminal` (estable en
  la stdlib, sin dependencia).
- **Alternativas**: un crate de parsing rico ofrece ayuda y validación
  automáticas, pero arrastra un árbol de dependencias desproporcionado para una
  gramática de este tamaño; un crate de parsing minimalista sigue siendo una
  dependencia evitable. Si la gramática creciera hasta justificarlo, se
  reconsidera en una change dedicada. La superficie interactiva (#6) sí traerá su
  propia dependencia de framework de TUI, justificada allí.

### D2 — Regla de despacho por subcomando y por TTY
El primer token no-flag decide el modo:
- Con **subcomando** (`status`, `propose`, …) → **modo scriptable de un disparo**,
  con independencia de si hay TTY.
- **Sin subcomando** y con **stdout conectado a un TTY** → **modo interactivo**.
- **Sin subcomando** y **sin TTY** (p. ej. en un *pipe* o en CI) → error de uso
  (código 2) con un mensaje que remite a `meltemi help`; nunca se cuelga
  esperando entrada.

En esta change el modo interactivo es un **arranque diferido**: emite por stderr
un aviso de que la interfaz interactiva llega en una entrega posterior y termina
con éxito (0). #6 sustituye ese cuerpo por la TUI real sin tocar la regla.
- **Por qué el TTY**: es la señal estándar y sin dependencias para distinguir uso
  humano de uso programático; garantiza que un script que redirige la salida
  nunca caiga por accidente en un modo interactivo.

### D3 — Taxonomía de códigos de salida
Contrato estable, con los códigos convencionales de shell reservados y los de
dominio a partir de 10 para no colisionar:

| Código | Significado |
|--------|-------------|
| 0  | Éxito |
| 1  | Error interno inesperado |
| 2  | Error de uso (subcomando desconocido, flags inválidos, falta argumento) |
| 10 | Daemon inalcanzable (fallo de arranque bajo demanda o de transporte) |
| 11 | Error de contrato (respuesta de error RPC, versión de protocolo no soportada) |
| 12 | Operación rechazada por política (denegada) |
| 13 | Operación cancelada |

- **Por qué**: 0/1/2 siguen la convención POSIX/shell; empezar el dominio en 10
  evita colisión con los reservados por el shell (126–128+n). El conjunto es
  extensible sin renumerar. Cualquier cambio a esta tabla exige un delta de spec.

### D4 — Disciplina de flujos
- **stdout**: exclusivamente la salida útil del comando —texto humano *o*, con
  `--json`, exactamente un objeto JSON—, apta para *pipe*. Nada de progreso ni
  diagnósticos.
- **stderr**: progreso, avisos, diagnósticos, errores y el aviso de modo
  interactivo diferido.
- En modo `--json`, **tanto el éxito como el error** emiten exactamente un objeto
  JSON en stdout (el error con un discriminante y el mismo código de la tabla D3);
  stderr queda libre de JSON. En modo humano, los errores van a stderr como texto.
- **Por qué**: un consumidor por máquina siempre puede parsear un único objeto en
  stdout y leer la categoría del código de salida, sin heurísticas.

### D5 — Mapeo comando↔RPC (normativo)
Todo subcomando respaldado por RPC envía primero `initialize` (el contrato lo
exige como primer mensaje) y luego su método:

| Subcomando | Método(s) RPC | Naturaleza |
|------------|---------------|------------|
| `status`        | `initialize` → `status`   | Consulta |
| `propose <idea>`| `initialize` → `propose`  | Delega en el agente |
| `stop`          | `initialize` → `shutdown` | Termina el daemon |
| `version`       | — (local)                 | Versión del cliente |
| `help`          | — (local)                 | Ayuda de la gramática |

Los subcomandos del ciclo SDD (`explore`, `review`, `plan`, `implement`,
`verify`, `archive`) se **reservan** en la gramática: se reconocen y responden
con un aviso de "no implementado todavía" (código 2 no; se usa un mensaje claro y
código 0 no — ver spec) sin inventar comportamiento. `propose` se cablea al RPC
existente, sin la disciplina de gates de #14.

### D6 — Ubicación y forma del binario
El crate `tui/` produce un binario llamado `meltemi`. El alias `mel` es materia de
empaquetado (#23); en el repositorio se construye un único binario y se documenta
el alias. El crate se añade como miembro del workspace raíz. Reutiliza el
transporte y el arranque bajo demanda ya implementados por/para `meltemid`
(bootstrap `connect_or_start`), sin duplicar la lógica de socket.

## Risks / Trade-offs

- **Parser propio con expectativas GNU** (flags combinados, `=`): un usuario
  podría esperar comportamiento de un parser rico. → Gramática pequeña y
  documentada; soporte de `--json`, `--help`/`-h`, `--version`/`-V` y `--` como
  fin de flags; tests que fijan la gramática.
- **Churn de la taxonomía de salida rompe scripts**: → los códigos son parte del
  contrato (spec); cambiarlos exige un delta y es un cambio observable.
- **Detección de TTY errónea bajo pipes/CI**: → `std::io::IsTerminal` sobre
  stdout; sin TTY siempre implica modo scriptable; caso cubierto por spec.
- **Modo interactivo diferido confunde**: → aviso claro por stderr que remite a
  `meltemi help`, y salida con éxito.
- **Divergencia con el devclient**: el devclient conserva gramática propia. → Se
  documenta que `meltemi` es la superficie contractual y el devclient es interno;
  no se unifican en esta change.

## Migration Plan

Cambio puramente **aditivo**: nuevo crate `tui/`, nuevo miembro del workspace,
nueva capacidad `cli-contract`. No hay estado que migrar ni ruptura de contrato
RPC. Reversión: retirar el crate del workspace; nada más depende de él todavía.
El `meltemi-devclient` permanece intacto como red de seguridad de depuración.

## Open Questions

- Ninguna que bloquee la implementación. El alias `mel` y el autocompletado se
  resuelven en la fase de distribución (#23); la TUI interactiva, en #6.
