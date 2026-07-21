## 1. Fundaciones

- [x] 1.1 `docs/ux/design-system.md`: tokens desde brand V2 (tipografía, color, espaciado, densidad) + regla transversal símbolo+palabra _(design D6)_
- [x] 1.2 Extraer el cliente JSON-RPC al crate compartido `core/meltemi-client` (módulos `rpc`/`transport`/`bootstrap`/`paths` de `meltemid`, re-exportados) sin cambio de contrato observable (TUI sigue verde) _(design D1)_
- [x] 1.3 Miembro `desktop/` del workspace: Tauri 2 pineado + Svelte 5/TS pineados por lockfile; capacidades mínimas deny-by-default; CSP sin orígenes remotos _(Req: Cliente fino sobre el socket local; design D1, D2)_
- [x] 1.4 Puente IPC: comandos/eventos Tauri ↔ `meltemi-client` (conexión, backoff, arranque bajo demanda, ErrorData con remedy) _(Req: Cliente fino; Desconexión ruidosa)_
- [x] 1.5 CI: build + tests de `desktop/` en las tres plataformas _(constitución §7)_

## 2. Shell y paridad

- [x] 2.1 Chrome + vistas Sesiones/Proyecto/Permisos/Flota + drill-in con breadcrumb + estados vacíos honestos (incl. sin proyecto) _(Req: Paridad de vistas y modelo de navegación)_
- [x] 2.2 Paleta de comandos con registro tipado de métodos RPC y autocompletado _(Req: Paleta y registro obligatorio)_
- [x] 2.3 `docs/paridad-nucleo.md` + check de CI contra `proto/schemas/v1/` que cubre los registros de TUI y GUI _(Req: Matriz de paridad; design D3)_
- [x] 2.4 i18n: catálogo ES/EN con override de idioma + lint anti-hardcodeo en la webview _(Req: Internacionalización ES/EN; design D6)_
- [x] 2.5 Accesibilidad base: operación 100% teclado, roles/etiquetas del árbol accesible, símbolo+palabra, alto contraste y movimiento reducido _(Req: Accesibilidad)_
- [x] 2.6 Onboarding de primer uso con flag persistente, sin cuenta/red/telemetría _(Req: Onboarding)_

## 3. Permisos, sesiones y flota

- [x] 3.1 Bandeja de permisos + indicador símbolo/contador/palabra + prioridad de señales + avisos de vencimiento persistentes _(Req: Bandeja de permisos y prioridad de señales)_
- [x] 3.2 Transcript de sesión en streaming con congelado y marca de corte en caída + reconexión honesta _(Req: Desconexión ruidosa y streaming honesto)_
- [x] 3.3 Panel de flota con perfiles, detección y niveles de integración _(Req: Paridad de vistas)_

## 4. Specs y diffs

- [ ] 4.1 Editor de specs enriquecido con findings de `validate` en vivo y guardado trazable _(Req: Editor de specs enriquecido)_
- [ ] 4.2 Revisión de diffs línea a línea por archivo/hunk para asignaciones y carreras, con comparación contra la base común _(Req: Revisión de diffs)_

## 5. Edición in situ y concurrencia

- [x] 5.1 `proto/`: método aditivo `worktree/apply-edit` + tipos + schema _(Req edit-surface: Trazabilidad de ediciones humanas; design D5)_
- [x] 5.2 Daemon: validación de ruta dentro del worktree + escritura + evento `human_edit` en el JSONL + estado turno-en-vuelo observable por worktree _(Req edit-surface: política de concurrencia; design D4)_
- [x] 5.3 Nota de ediciones humanas antepuesta al siguiente turno del agente, evidenciada en el log _(Req edit-surface: Nota al siguiente turno)_
- [x] 5.4 Paridad del método nuevo: subcomando CLI + registro en la paleta de la TUI _(constitución §4; design D5)_
- [ ] 5.5 GUI: árbol/pestañas/búsqueda/resaltado + guardado vía daemon aplicando la política (reforzada/simple/directa) _(Req: Edición utilitaria in situ; edit-surface)_
- [ ] 5.6 LSP BYO: detección/config de servidores del usuario + degradación honesta sin servidor _(Req: Edición utilitaria con LSP BYO)_
- [ ] 5.7 "Abrir con…" hacia el editor del usuario con archivo:línea desde diff y árbol _(Req edit-surface: Deep-link; Req: Revisión de diffs)_

## 6. Distribución y presupuestos

- [ ] 6.1 Bundler Tauri en el pipeline: MSI/DMG/AppImage+deb firmados + gate de tamaño < 15 MB _(Req release-distribution: Instaladores de la GUI)_
- [ ] 6.2 Presupuestos: arranque < 1 s y RAM en reposo < 80 MB medidos y publicados en QA por release _(Req: Presupuestos de huella)_

## 7. Calidad

- [ ] 7.1 Unit/contract: puente IPC, registro de paleta, política de concurrencia en sus tres estados, sobre fixtures temporales con `mock-agent` _(constitución: fixtures, mock, sin red)_
- [ ] 7.2 Smoke e2e con tauri-driver donde exista driver; verificación manual documentada por escenario donde no _(design: riesgos)_
- [ ] 7.3 `cargo clippy -- -D warnings`, `cargo fmt --check` y tests verdes en las tres plataformas _(constitución §7)_
