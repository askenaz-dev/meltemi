# Tasks — pulido-pre-anuncio

## 1. Skin global de botones (GUI)

- [x] 1.1 Añadir `display: inline-flex; align-items: center; gap: var(--sp-2)`
  a la regla global `button` de `desktop/ui/src/app.css`, fijar
  `align-items: center` en `.actions` de `EmptyState.svelte`, y cubrir
  «Icono y etiqueta en una línea» y «Par de acciones del estado vacío a
  altura pareja» en `desktop/tests/scenarios_shell.rs`
- [x] 1.2 Retirar las re-declaraciones locales que dupliquen exactamente la
  regla global (TopBar, App, SessionDetail, Sidebar, Fleet, Sessions,
  NewSession, Editor — solo duplicados exactos; los overrides deliberados de
  `gap` se conservan, design D1), con `npm run check` limpio

## 2. Etiqueta sin falso contador (GUI)

- [x] 2.1 Dejar `sessions.empty.fleet` en «Ver la flota» / «See the fleet» en
  `desktop/ui/src/lib/messages.ts` (ES y EN) y cubrir «La acción de flota sin
  falso contador» en `desktop/tests/scenarios_shell.rs`; «El atajo conserva
  su afordancia» queda cubierto por el `kbd` existente del sidebar y el smoke
  de 4.1

## 3. Instantánea del registro y guía (daemon + docs)

- [ ] 3.1 Actualizar los `adapter-install` de las dos entradas de nivel 2 en
  `core/meltemid/data/fleet-registry.toml` al scope vigente
  `@agentclientprotocol` (design D3), subir `version` a `2026-07-27`, y
  actualizar las secciones correspondientes de `docs/agentes.md` en el mismo
  commit — la coherencia la vigila `core/meltemid/tests/agents_guide.rs`
  (`the_guide_states_the_level_and_binaries_the_registry_declares`); marcar
  «Comando de instalación verificado contra la distribución vigente» y
  «Distribución archivada reemplazada por su sucesora» como verificación
  documentada con fuente y fecha (`sdd/verify-mark`)

## 4. Verificación

- [ ] 4.1 Gates locales — `cargo fmt --check`, `cargo clippy -- -D warnings`,
  tests del workspace (incluidos `meltemi-desktop::scenarios_shell` y
  `meltemid::agents_guide`), `npm run check` y `npm run lint:i18n` — y smoke
  visual conducido por CDP sobre el binario reconstruido: los siete botones
  del inventario en una línea, alturas del par del estado vacío parejas
  también al envolver, etiqueta sin «(4)», sin regresión en los botones de
  solo texto; publicado en `docs/qa/<fecha>-pulido-pre-anuncio-smoke.md`
