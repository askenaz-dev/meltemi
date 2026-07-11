# Política de cabeceras SPDX

Todo archivo fuente del proyecto lleva como primera línea (o primera línea
tras el shebang, si existe) un identificador SPDX de licencia:

```rust
// SPDX-License-Identifier: Apache-2.0
```

## Alcance

- **Sí llevan cabecera**: archivos `.rs`, scripts (`.ps1`, `.sh`), y cualquier
  otro archivo de código fuente que admita comentarios.
- **No llevan cabecera**: archivos de datos y configuración declarativa
  (`.json`, `.toml`, `.yml`, `.md`, assets de `brand/`). Los JSON Schemas de
  `proto/` declaran la licencia en el `proto/README.md` que los acompaña.

## Sintaxis por tipo de archivo

| Tipo | Cabecera |
| --- | --- |
| Rust (`.rs`) | `// SPDX-License-Identifier: Apache-2.0` |
| PowerShell (`.ps1`) | `# SPDX-License-Identifier: Apache-2.0` |
| Shell (`.sh`) | `# SPDX-License-Identifier: Apache-2.0` (tras el shebang) |

## Verificación

La cabecera se revisa en code review. No se acepta código nuevo sin ella;
si se detecta un archivo sin cabecera, se corrige en el siguiente commit
que lo toque. La licencia del proyecto es Apache-2.0 (ver `LICENSE` y
`NOTICE` en la raíz).
