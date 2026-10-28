# opencode-que-pregunta

> Vía rápida. 5º de la tanda de `release-para-un-desarrollador` (2026-10-28).
> Nace del hallazgo 1 de la corrida integrada del 2026-09-26
> (`docs/qa/2026-09-26-corrida-integrada.md`): «OpenCode edita sin pedir
> permiso … Es la primera cosa que arreglar».

## Why

En la corrida integrada, OpenCode 1.18.32 no hizo una sola petición de permiso:
escribió un archivo sin regla y sin humano, y su despacho hizo commit. Con su
configuración por omisión, casi todos sus permisos son `allow`
(<https://opencode.ai/docs/permissions/>), así que no pregunta y el proxy de
Meltemi no tiene nada que decidir. La sesión queda registrada pero **no
gobernada**, y eso contradice la promesa del producto: toda sesión corre
gobernada.

OpenCode documenta dos cosas que bastan para cambiarlo sin tocar sus archivos:
cada permiso admite `ask`, y la variable `OPENCODE_CONFIG_CONTENT` lleva una
configuración en línea que se aplica por encima de la global y de la del
proyecto (<https://opencode.ai/docs/config/>). Su página de ACP dice que el
sistema de permisos funciona igual por ACP (<https://opencode.ai/docs/acp/>).

## What Changes

- **El registro puede declarar el entorno con que se lanza un agente**
  (`launch-env`), como dato, igual que ya declara sus argumentos ACP. El daemon
  lo aplica en toda resolución de ese agente —por perfil, por id o como agente
  del proyecto—, nunca a un comando literal que el usuario escribió, y nunca
  por encima de una variable del mismo nombre que el usuario ya fijó en su
  entorno o en su perfil.
- **La entrada de OpenCode lo declara**: `OPENCODE_CONFIG_CONTENT` con
  `permission` en `ask` para editar, ejecutar comandos, ir a la red, lanzar
  subagentes y tocar rutas fuera del proyecto. Las lecturas dentro del proyecto
  siguen sin preguntar.
- **La guía de agentes lo dice**, y la verificación con un OpenCode real queda
  escrita en `docs/conformidad-manual.md` como pierna pendiente: CI no corre
  agentes reales (constitución §5).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `fleet-catalog`: + entorno de lanzamiento declarado por el registro.

## Impact

- `core/meltemid/data/fleet-registry.toml`, `core/meltemid/src/fleet.rs`,
  `core/meltemid/src/levels.rs` y sus tests; `docs/agentes.md`,
  `docs/conformidad-manual.md`.
- Cero dependencias, cero RPC, cero cambios en el contrato.

## Fuera de alcance

- Reescribir la configuración del usuario de OpenCode, o mezclar JSON con un
  `OPENCODE_CONFIG_CONTENT` que el usuario ya tenga: si lo tiene, manda el suyo
  y la guía dice qué añadirle.
- Las reglas por comando o por ruta para OpenCode: dependen de lo que mande en
  su petición, y eso lo dirá la pierna manual.
