# Release automática tras CI — verificación local

Fecha: 2026-11-01. Change: `release-automatica-main`.

## Resultado

La reserva y recuperación se verificaron con repositorios Git temporales y API
simulada. El estampado se ejecutó además sobre un worktree real separado de la
rama: versión fuente 0.2.0, versión de build 0.2.123. No se creó tag remoto ni
release durante estas pruebas, y la versión de la rama principal quedó intacta.

| Comprobación | Resultado |
|---|---|
| `node --test scripts/automatic-release.test.mjs` | 13/13 |
| `cargo test --locked -p meltemi-release` | 24/24 |
| Controles `core/meltemid/tests/release.rs`, compilados con `rustc --test` | 32/32 |
| `cargo test --locked -p meltemi-spec --test dogfood` | 2/2; incluye árbol `.meltemi/` real |
| `cargo fmt --all --check` y `git diff --check` | Sin hallazgos |

## Estampado real

Sobre un worktree detached de la implementación:

```powershell
$env:RELEASE_VERSION = '0.2.123'
node scripts/automatic-release.mjs stamp
node scripts/automatic-release.mjs stamp
cargo test --locked -p meltemi-release
```

Ambas invocaciones de stamp terminaron bien; la segunda no volvió a modificar
portadores. Solo cambiaron `Cargo.toml`, `Cargo.lock`, ambas configuraciones
Tauri y package.json/package-lock.json del frontend. El lockfile movió únicamente
los 13 miembros del workspace, validado por la herramienta existente.
Los 24 tests de la herramienta y los 32 controles de release también pasaron
contra ese árbol estampado. El worktree temporal se retiró después de la prueba.

## Cobertura de fallos

- CI de PR/fork, workflow ajeno, rama ajena, fallo y SHA distinto rechazados.
- CI fuera de orden: números estables por historia first-parent; recuperación
  antigua no cambia latest.
- Base minor nueva, base antigua anterior al tag histórico y patch declarada
  inferior al número ya liberado.
- Tag de otro SHA y tag histórico sin reserva rechazados.
- Upload fallido conserva borrador; reintento lo completa sin nueva versión.
- Publicado incompleto, manifiesto diferente o digest discrepante fallan sin
  mutación de la release inmutable.
- Notas limitadas al rango de la reserva, sin atribuir pruebas manuales.

## Límite de esta evidencia

No se ejecutó el build completo de producto ni se publicaron binarios desde esta
máquina. Linux/macOS/Windows y la integración real con Actions quedan a los gates
de la PR y a la primera ejecución después del merge. El CLI instalado no pudo
ejecutar `meltemi validate/verify` porque no encuentra `meltemid`; la validación
estructural se realizó con el mismo motor mediante su test dogfood. La change
permanece activa hasta la validación remota.
