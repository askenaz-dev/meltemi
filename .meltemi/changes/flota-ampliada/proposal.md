# flota-ampliada

> Vía rápida (fast-forward): entradas nuevas del registro embebido, cada una
> verificada contra el registro público ACP y el binario oficial antes de
> entrar, con estatus legal dicho sin maquillaje. Deltas solo ADDED sobre
> `fleet-catalog` e `initial-docs`. Nace del comparativo con Orca del
> 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`, brechas AI-3 y
> la mitad de nivel 1 de AI-9).

## Why

Orca anuncia 27 agentes; Meltemi lista 10, de los cuales seis son nivel 1 y
solo OpenCode se ha ejercido. La diferencia de número no es la brecha —Orca
mete cualquier CLI en una terminal, sin protocolo ni gobierno—; la brecha es
que **agentes que sí hablan ACP nativo por stdio no están en el registro**,
y un usuario que los tiene instalados ve una flota vacía. `[[fleet.custom]]`
ya permite declararlos a mano (`fleet-catalog`, «Agentes personalizados del
usuario»), pero un registro que obliga a escribir TOML para un agente
público no es un registro.

La regla de entrada ya está escrita por `flota-deteccion-guia`: binario
oficial, argumentos ACP verificados, comando de instalación del proveedor,
estatus legal declarado (`grey` mientras el proveedor no publique posición),
y `docs/agentes.md` verificada contra el registro por un test que rompe el
build si divergen. Constitución §2: Meltemi detecta, jamás empaqueta un
agente ajeno.

## What Changes

- **Verificación vendor por vendor** contra el registro público ACP y la
  documentación de cada CLI: qué binario, qué argumentos abren el servidor
  ACP por stdio, qué anuncia (modos, opciones, MCP), qué licencia y qué
  postura publica sobre clientes de terceros. Candidatos a mirar (no a
  copiar): Goose, Amp, Qwen Code, Kimi CLI, Mistral Vibe, Droid, Auggie, Rovo
  Dev, Pi, Hermes Agent, Cline, Charm. Entra el que **se verifica**, con la
  fecha; el que no habla ACP nativo se anota como nivel 3 o 4 candidato y no
  entra por esta vía.
- **Una entrada por agente verificado** en
  `core/meltemid/data/fleet-registry.toml`: id, nombre, nivel 1, `bin`,
  rutas candidatas por plataforma, `acp-args`, `install`, `mcp-support`,
  `legal-status` y `legal-note` con la cita o su ausencia. Versión del
  registro incrementada.
- **`docs/agentes.md`** gana una sección por entrada (el test la exige) y una
  tabla de «verificado el … con la versión …»; `docs/conformidad-manual.md`
  los lista como **no medidos** en este equipo hasta que la corrida los
  toque.
- **La flota los muestra** como muestra a los demás: detección pasiva,
  estado compuesto, remedio con el comando exacto del proveedor.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `fleet-catalog`: + entradas verificadas con fecha y estatus legal; +
  escenario «un agente candidato sin ACP nativo no entra como nivel 1».
- `initial-docs`: + secciones por agente en la página de agentes.

## Impact

- Archivos: `core/meltemid/data/fleet-registry.toml`, `docs/agentes.md`,
  `docs/conformidad-manual.md`, `docs/research/integracion-agentes.md`
  (matriz interna), el test de sincronía registro↔docs.
- Cero dependencias nuevas; cero RPC; el daemon no cambia de forma.
- Riesgo: un CLI que anuncia ACP pero lo implementa a medias; por eso entra
  con estatus y fecha, y el nivel **verificado** se otorga solo en la
  corrida (`corrida-integrada`).

## Fuera de alcance

- Declarar agentes personalizados desde la GUI y la TUI:
  `registro-agentes-en-superficie` (nombrada, sigue siendo change propia).
- Agentes sin ACP nativo por terminal pty: rechazado por rumbo (una pty
  esquiva el proxy y el registro); nivel 3 espera a `runner-nivel-3`.
- Empaquetar o instalar agentes: constitución §2.
