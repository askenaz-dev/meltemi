<!-- SPDX-License-Identifier: Apache-2.0 -->

# Instrucciones compartidas con el harness

[English](../harness.md)

Usa las reglas del harness para las convenciones que quieras que lean los agentes antes de
trabajar: estilo del lenguaje, estructura del proyecto o expectativas de revisión. Escribe un
archivo `RULE.md`, comprueba qué versión se aplica y después proyecta el contexto en los archivos
de instrucciones del agente. Las reglas son instrucciones para el agente; no conceden permisos
de herramientas ni sustituyen al proxy de permisos.

## Los cuatro ámbitos

`<config-dir>` es la carpeta de configuración de Meltemi, que muestra `meltemi doctor` y que se
enumera en [plataformas](plataformas.md#rutas-de-datos). `<repo>` es la raíz del proyecto:

```
<config-dir>/harness/rules/<name>/RULE.md                 (1) user
<config-dir>/harness/per-agent/<id>/rules/<name>/RULE.md   (2) user × agent
<repo>/.meltemi/harness/rules/<name>/RULE.md               (3) project
<repo>/.meltemi/harness/per-agent/<id>/rules/<name>/RULE.md (4) project × agent
```

Usa los identificadores que imprime `meltemi fleet`, como `claude-code` o `codex-cli`. Un directorio
por agente desconocido se informa y no se proyecta. El directorio por agente determina quién
recibe la regla; un campo de metadatos como `agents_supported` no selecciona un agente.

## Escribir una regla

Crea un directorio llamado `no-any-cast` y guarda esto como su archivo `RULE.md`:

```yaml
---
name: no-any-cast
scope: ["**/*.{ts,tsx}"]
severity: error
description: "Avoid TypeScript escape hatches"
---

Keep types explicit. Explain any unavoidable unsafe cast in the review.
```

El campo `name` debe coincidir con el nombre del directorio. Usa listas en línea
(`scope: [a, b]`); no se admiten listas en bloque. Una regla inválida o ilegible se diagnostica y
se excluye de la proyección.

La vista reconoce `name`, `scope`, `severity` y `description`. Los demás metadatos se conservan y
se muestran. Un `scope` describe la regla en el texto proyectado; no controla el acceso a archivos
ni filtra las operaciones del propio agente.

## Qué regla se aplica

La precedencia es **1 < 2 < 3 < 4**. La regla del mismo nombre que está en el ámbito más
específico sustituye por completo a la otra; no se mezclan campos de versiones diferentes. La
vista efectiva también muestra las reglas sustituidas o inválidas con sus motivos.

```bash
meltemi harness --agent claude-code
```

- CLI: `meltemi harness [--agent <id>]`, opcionalmente con `--json`.
- TUI: `:harness` o `:harness <id>` en la paleta de comandos.
- Escritorio: selecciona un agente en la Flota y abre su sección del harness.

Estas vistas son de sólo lectura. Edita la regla en su archivo y vuelve a consultar la vista.

## Destinos del proyecto y del usuario

Las reglas del proyecto se añaden a la constitución, el rumbo y el cambio activo en el bloque
gestionado de los archivos de instrucciones del repositorio, como `AGENTS.md` y `CLAUDE.md`.

```bash
meltemi project
```

Sólo las reglas del proyecto se incluyen en los archivos del repositorio. Las reglas de usuario
se proyectan por separado en el archivo global de instrucciones del agente compatible después
de que des tu consentimiento. Meltemi sólo modifica su bloque gestionado; el contenido fuera de
ese bloque sigue siendo tuyo.

Los destinos de usuario admitidos son `~/.claude/CLAUDE.md`, `~/.codex/AGENTS.md` y
`~/.config/opencode/AGENTS.md`. La carpeta base de Codex sigue `CODEX_HOME`; si existe
`AGENTS.override.md`, el destino habitual `AGENTS.md` se informa como ignorado y no se modifica.
Si un agente no tiene un destino global configurado, se informa en lugar de asignarle una ruta
inventada.

## Consentimiento para instrucciones globales

Para habilitar un destino de usuario, usa el formulario `context/project` de la paleta de
comandos del escritorio e incluye los identificadores del catálogo correspondientes en
`consentUserScope`. El consentimiento se guarda en `harness-consent.toml`, dentro de la carpeta
de configuración de Meltemi. `meltemi project` no concede implícitamente ningún consentimiento
nuevo: usa el que ya está guardado.

Para retirarlo, elimina el identificador del agente de ese archivo de consentimiento. Las
proyecciones posteriores dejan de actualizar el bloque global gestionado de ese agente; retirar
el consentimiento no borra el contenido existente del archivo.

## Límites actuales

La gestión del harness abarca actualmente las reglas. Meltemi no instala skills, hooks ni
subagentes desde esta vista. Tampoco crea formatos nativos de reglas por ámbito para otras
herramientas. Configúralos en la herramienta correspondiente y consulta [agentes](agentes.md)
para entender cómo la inicia Meltemi.
