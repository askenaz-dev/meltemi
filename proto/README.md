# `proto/` — Contrato daemon ↔ clientes

Los JSON Schemas de `proto/schemas/` son la **fuente de verdad, neutral al
lenguaje**, del protocolo entre `meltemid` y sus clientes (TUI, GUI, tooling).
El crate [`meltemi-proto`](meltemi-proto/) contiene los tipos serde
equivalentes; el test de conformidad del crate valida que serializan conforme
a estos schemas. Ante cualquier discrepancia, **el schema manda**.

## Transporte

JSON-RPC 2.0 con delimitación por líneas (un mensaje por línea, UTF-8, `\n`)
sobre socket local: Unix domain socket con permisos `0700` en macOS/Linux,
named pipe con ACL restringida al usuario en Windows. Sin puertos de red,
jamás. La conexión es bidireccional: el daemon también envía peticiones
(`permission/request`) y notificaciones (`session/event`) al cliente.

## Versionado (`protocolVersion`)

- El contrato lleva un **entero** de versión. Versión actual: **1**
  (`schemas/v1/`).
- El cliente declara su versión en `initialize` (primera petición de toda
  conexión). El daemon la acepta o responde el error `1000`
  (`protocol_version_unsupported`) incluyendo la versión declarada y las
  soportadas, y cierra la conexión ordenadamente.
- **Cambios aditivos** (campos opcionales nuevos, métodos nuevos, valores de
  enum nuevos donde el consumidor tolera desconocidos) **no incrementan** la
  versión. Los schemas no usan `additionalProperties: false` por esta razón;
  los consumidores deben ignorar campos desconocidos.
- **Cambios rompedores** (quitar o renombrar campos, cambiar tipos o
  semántica) incrementan la versión y crean `schemas/v<N+1>/`.
- El evento de sesión JSONL lleva su propio versionado (`v` en el envelope de
  `session-event.schema.json`), independiente del `protocolVersion`.

## Métodos y notificaciones (v1)

| Mensaje | Dirección | Tipo | Schema |
| --- | --- | --- | --- |
| `initialize` | cliente → daemon | request | `initialize.schema.json` |
| `status` | cliente → daemon | request | `status.schema.json` |
| `shutdown` | cliente → daemon | request | `shutdown.schema.json` |
| `propose` | cliente → daemon | request | `propose.schema.json` |
| `fleet/list` | cliente → daemon | request | `fleet.schema.json` |
| `context/project` | cliente → daemon | request | `context.schema.json` |
| `session/list` | cliente → daemon | request | `session-list.schema.json` |
| `session/log` | cliente → daemon | request | `session-log.schema.json` |
| `session/cancel` | cliente → daemon | notification | `session-cancel.schema.json` |
| `session/event` | daemon → cliente | notification | `session-event.schema.json` |
| `permission/request` | daemon → cliente | request | `permission.schema.json` |
| `permission/timeout` | daemon → cliente | notification | `permission.schema.json` |
| `permission/pending` | cliente → daemon | request | `permission.schema.json` |
| `permission/decide` | cliente → daemon | request | `permission.schema.json` |
| `permission/changed` | daemon → cliente | notification | `permission.schema.json` |

Los errores de aplicación (códigos 1xxx/2xxx/3xxx con `data`
`{kind, detail, remedy}`) están catalogados en `error.schema.json`.

Los objetos que Meltemi reenvía tal cual desde ACP (el `update` de
`session/event`, el `toolCall` de `permission/request`) pertenecen al
contrato de ACP en la versión pineada del crate oficial; estos schemas los
declaran como objetos abiertos a propósito.

## Convenciones

- Nombres de campos en `camelCase`; valores de enum en `snake_case`
  (el mismo estilo que ACP).
- Strings del contrato (mensajes de error, `detail`, `remedy`) en inglés.
- Timestamps RFC 3339 en UTC.

## Licencia

Apache-2.0, igual que el resto del proyecto (ver `LICENSE` y `NOTICE` en la
raíz). Los archivos JSON no llevan cabecera SPDX porque el formato no admite
comentarios; esta declaración los cubre (ver `docs/politica-spdx.md`).
