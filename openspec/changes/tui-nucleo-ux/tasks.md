## 1. Fundamentos del shell y framework

- [x] 1.1 Añadir el framework de TUI y su backend de terminal (perfil `ratatui` + `crossterm`) pineados al workspace, con features mínimas, cabecera SPDX y auditoría cargo-deny (design D6)
- [x] 1.2 Bucle de eventos del shell: entrada/salida de raw mode y alternate screen con restauración garantizada del terminal (RAII) en salida y en pánico, manejo de SIGWINCH, y salida limpia
- [x] 1.3 Reemplazar la rama `Interactive` de `dispatch` para lanzar el shell en vez del aviso diferido _(Modified: cli-contract — Regla de despacho CLI↔TUI)_

## 2. Chrome, keymap y navegación

- [x] 2.1 Chrome persistente (header/footer) con estado de conexión e indicador de bandeja de permisos irrenunciables, breadcrumb de drill-in _(Req: Modelo de vistas y chrome persistente)_
- [x] 2.2 Keymap como dato único con contrato de consistencia y split dígitos-global/letras-local; lint de consistencia y de reserva global de teclas transversales _(Req: Navegación por teclado y contrato de consistencia; Reserva global de teclas de acción transversal)_
- [x] 2.3 Conjunto de teclas robusto (sin Alt/Meta, F1–F12, Ctrl del TTY) y desambiguación de Esc por timeout con alternativa `q`/Backspace _(Req: Conjunto de teclas robusto y desambiguación de Esc)_
- [x] 2.4 Overlays: ayuda `?` (mapa de teclas) y paleta `:` con captura total de teclado, "Esc para salir" y registro de capacidades del daemon _(Req: Captura de teclado en contextos de entrada de texto; Paridad de núcleo por la paleta de comandos)_
- [x] 2.5 Diálogos de confirmación como superficie modal de primera clase (foco atrapado, Esc/`q`/Backspace cancela seguro, opción no destructiva por defecto) _(Req: Diálogos de confirmación como superficie modal de primera clase)_

## 3. Línea base de accesibilidad (transversal)

- [x] 3.1 Tabla única de glifos con gemelo ASCII, detección de capacidad Unicode y overrides (`--ascii`, `MELTEMI_ASCII=1`, config); lint "ningún glifo Unicode sin gemelo" _(Req: Accesibilidad — fallback ASCII)_
- [x] 3.2 Soporte de `NO_COLOR`/`--no-color`/`TERM=dumb` (render sin color, sin pintar fondo) y codificación redundante símbolo+etiqueta; foco/selección distinguibles sin color y entre sí _(Req: Accesibilidad — nunca solo color; Accesibilidad — NO_COLOR)_
- [x] 3.3 Tabla de mensajes ES/EN con lint de hardcodeo; reduce-motion por defecto sobre SSH; reconocer `--json` como ruta accesible garantizada _(Req: Ruta accesible garantizada e internacionalización)_

## 4. Vistas cableadas al daemon

- [x] 4.1 Conexión asíncrona: chrome inmediato y conexión/arranque en segundo plano, distinguiendo "conectando…" (transitorio) de "inalcanzable" (código 10) con diagnóstico _(Req: Estado vacío sin daemon)_
- [x] 4.2 Vista Sesiones (tabla desde `status`) y drill-in Sesión con región append-only del stream `session/event` y seguimiento de cola suspendible al desplazar _(Req: Modelo de vistas; Reflow, streaming y seguimiento de cola)_
- [x] 4.3 Vistas Proyecto (artefactos `.meltemi/`, verbos SDD reservados no-error), Permisos (casa) y Flota (casa) con sus estados vacíos y el launchpad de Sesiones _(Req: Estado vacío sin sesiones; Estado vacío sin proyecto y desacople de ámbito)_
- [x] 4.4 Reflow en SIGWINCH: colapso a modo lineal bajo ancho mínimo, descarte ordenado de columnas y scroll horizontal en vez de truncar-y-ocultar _(Req: Reflow, streaming y seguimiento de cola sobre SSH)_

## 5. Endurecimiento (paridad y seguridad)

- [x] 5.1 Prioridad de señales del shell + desconexión ruidosa con aviso de deny-by-default y reconexión con backoff + reconciliación de vencimientos de permiso _(Req: Indicador de bandeja y prioridad de señales; Desconexión ruidosa y reconexión con backoff; Reconciliación de vencimientos de permisos)_
- [x] 5.2 Control de ciclo de vida: cancelar la sesión activa con `x` (`session/cancel`) y apagar el daemon (`shutdown`), ambos con confirmación y superficiados al menos por la paleta _(Req: Control de ciclo de vida de sesión y daemon)_
- [x] 5.3 Pérdida de daemon en vivo (congelar transcript con marca de corte, honestidad al reconectar) y suelo duro de tamaño de terminal con degradación automática que preserva los indicadores críticos _(Req: Pérdida de daemon durante una sesión en vivo; Suelo duro de tamaño de terminal)_
- [x] 5.4 Onboarding de primer uso: overlay saltable y persistente que enseña navegación, cómo salir (`q`) y cómo escapar de la captura (Esc), con checklist contextual, sin red ni telemetría _(Req: Onboarding de primer uso)_

## 6. Tests y calidad

- [x] 6.1 Tests de navegación (reductores puros, sin terminal): consistencia del keymap, split dígitos/letras, equivalencia Esc/`q`, la captura de texto no filtra dígitos, reserva global respetada
- [x] 6.2 Tests de accesibilidad: lint "glifo sin gemelo ASCII" falla, render con `NO_COLOR` sin ANSI de color, snapshots en 80x24 / ASCII / monocromo (por plataforma)
- [x] 6.3 Tests de estados contra un daemon efímero en proceso donde aplique: sin daemon (transitorio vs inalcanzable), sin sesiones, sin proyecto, suelo de tamaño, pérdida de daemon en vivo
- [x] 6.4 `cargo clippy -- -D warnings`, `cargo fmt --check` y `cargo test` verdes en el workspace; verificar que el terminal se restaura en salida y en pánico
