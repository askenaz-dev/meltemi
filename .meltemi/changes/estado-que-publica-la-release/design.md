# Design — estado-que-publica-la-release

## Context

Hay cuatro lugares que dicen el estado de una capacidad: la tabla de `/ide/`
en meltemi-web, `docs/plan-de-cambios.md`, el inventario de `entrega-sin-huecos`
y la memoria del mantenedor. Ninguno tiene test. La release, que es lo único
que un usuario puede comprobar, no dice nada en un formato legible por una
máquina: publica binarios, `SHA256SUMS`, la atestación y las notas.

`release.yml` tiene un job `version` sin secretos que ya compila y corre
`meltemi-release` (`gate`, `notes`) y sube un artefacto (`release-notes`) que
recoge `publish`. Ese es el sitio natural para un archivo más.

## Decisions

### D1 — Un TOML de texto en `docs/`, y un JSON derivado en la release

`docs/capacidades.toml` porque lo edita una persona, se revisa en una PR y
admite comentarios. El JSON es para quien lo lee desde fuera: lo genera la
herramienta y nunca se escribe a mano.

```toml
[[capability]]
id = "mobile"
state = "in-development"
assets = []                 # nombres estables de release.yml
changes = ["apk-en-la-release", "firma-de-plataforma"]

[capability.name]
en = "Mobile companion"
es = "Compañero móvil"

[capability.note]
en = "Android, built from source and tested on an emulator; no APK is published yet."
es = "Android, se compila desde el código y se probó en un emulador; todavía no se publica un APK."
```

```json
{
  "version": "0.3.0",
  "capabilities": [
    {
      "id": "mobile",
      "state": "in-development",
      "name": { "en": "…", "es": "…" },
      "note": { "en": "…", "es": "…" },
      "assets": [ { "name": "…", "platformSigned": false } ]
    }
  ]
}
```

`meltemi-release` todavía no depende de `toml`. El workspace ya lo tiene, así
que sumarlo al crate no agrega nada a `Cargo.lock` ni a `deny.toml`; la tarea
1.1 lo confirma. Si no fuera así, el formato pasa a JSON a mano, sin
dependencia nueva.

*Alternativa descartada*: escribir el estado en el frontmatter de cada change.
El estado es de la capacidad, no de una change, y varias changes la cierran.

### D2 — Tres reglas que atan el archivo a los hechos

`capabilities --check` (y un test de la suite que lo corre sobre el árbol) lee
el TOML, `release.yml`, `.meltemi/changes/` y `.meltemi/changes/archive/`:

| Estado | Exige | Prohíbe |
| --- | --- | --- |
| `published` | cada asset suyo aparece en `release.yml` por su nombre estable (o por su patrón con `${{ runner.os }}` resuelto a Windows, macOS y Linux) | — |
| `in-development` | al menos una change de `changes` abierta en `.meltemi/changes/` | — |
| `not-yet` | — | una change de `changes` archivada; un asset suyo producido por `release.yml` |

Además, toda capacidad tiene nombre y nota en los dos idiomas (constitución
§11) y un id único. Lo que no se puede atar, que es si la nota es justa, se
revisa en la PR.

La regla de `not-yet` es la que habría evitado el error del 2026-10-29:
`mobile` con `companero-movil` en sus changes y archivada no puede decir «todavía
no».

### D3 — El estado de hoy

| id | Estado | Assets | Changes | Nota (en) |
| --- | --- | --- | --- | --- |
| `core` | published | los tres archivos del núcleo, `install.sh`, `install.ps1` | — | Releases signed by the workflow that publishes them, with per-platform archives. |
| `desktop` | in-development | MSI, DMG, deb | `herramientas-a-la-vista`, `firma-de-plataforma`, `distribucion-y-actualizacion` | Installers ship with each release; Windows and macOS warn on first run until they carry platform signatures, and tool calls show only their title. |
| `mobile` | in-development | — (`apk-en-la-release` lo suma) | `apk-en-la-release`, `firma-de-plataforma` | Android, built from source and tested on an emulator; no APK is published yet. |
| `sdk` | not-yet | — | `plugins-skills-sdk` (solo nombrada) | There is no packaged SDK yet; the contract is public as JSON Schemas, and the command line is scriptable with `--json`. |

Una change «solo nombrada» (sin directorio) cuenta como no archivada. Las notas,
en inglés y en español, son las que `ide-state-as-it-is` pone a mano en el
sitio (su design D1), palabra por palabra: `ide-state-from-release` prueba con
un fixture que pintar desde la release no cambia ninguna. Si una nota cambia
aquí, cambia allá en la misma tanda.

### D4 — Lo genera `version`, lo publica `publish`

En `release.yml`, dentro del job `version` y solo cuando el gate decide
publicar:

```
cargo run --locked -p meltemi-release -- capabilities --version "$VERSION" --out capabilities.json
upload-artifact name: meltemi-capabilities, path: capabilities.json
```

El `pattern: meltemi-*` de `publish` lo recoge con los demás, entra al
`SHA256SUMS` y a la atestación, y se adjunta antes de publicar (las releases
son inmutables). Ningún job con credenciales compila nada nuevo.

`firma-de-plataforma` necesita escribir `platformSigned: true` de lo que
verificó. Para eso moverá la generación a su job de verificación de firmas, que
corre después de firmar y antes de publicar, pasando `--signed <asset>,…`. La
bandera existe desde esta change, y sin ella todo es `false`. Es un hecho: hoy
nada lleva firma de plataforma.

### D5 — El nombre del asset es estable

`capabilities.json`, sin versión en el nombre, como el resto (requisito
«Nombres de artefacto estables por plataforma»). Así
`releases/latest/download/capabilities.json` siempre resuelve, y es lo que lee
meltemi-web.

## Risks / Trade-offs

- **El patrón de nombres de `release.yml` cambia** y el test deja de encontrar
  un asset: falla en CI con el nombre, que es lo que debe pasar.
- **Una release sin `capabilities.json`** (todas hasta la primera tras esta
  change): el sitio conserva sus filas escritas. Lo resuelve
  `ide-state-from-release`, no esta change.
- **Una nota injusta pasa el test.** El test ata estados a hechos, no
  adjetivos. La nota se revisa como texto.
