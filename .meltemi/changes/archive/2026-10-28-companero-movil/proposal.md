# companero-movil

> Vía completa (proposal → design → specs → tasks). Es la change de fase 3
> que las specs `mobile-companion` y `remote-access` gobiernan desde julio;
> estaba nombrada en el backlog y se abre ahora con lo que el comparativo con
> Orca del 2026-09-24 dejó decidido
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas MOB-1, MOB-2,
> MOB-4 y MOB-9). **No empieza hasta que sus prerrequisitos aterricen**:
> `identidad-propia`, `via-ssh-del-cliente`, `maquinas-remotas`,
> `estado-en-vivo`, `aviso-de-espera-opt-in` y `verbos-del-agent-boss`.

## Why

«Keep agents moving from your phone» es la primera línea de la sección
móvil de Orca, y su app —iOS en la App Store, Android como APK— es lo que
más pesa en sus testimonios. Meltemi tiene la spec (cuatro verbos:
monitorear, aprobar, revisar, dirigir; nunca autoría; solo túnel SSH; regla
de subconjunto), la maqueta (`design-system/Meltemi Mobile.dc.html`:
conexión, aprobar, flota y ajustes) y los tres prerrequisitos de daemon
archivados (`espera-humana`, `sesion-esperando`, `eventos-para-tardios`).
No tiene una línea de código: `desktop/tauri.conf.json` solo declara
msi/dmg/deb y no existe `mobile/`.

La forma de Orca —relé propio en la nube y push sin túnel— está fuera por
constitución §3 y por la spec. La forma de Meltemi ya está escrita en
`docs/acceso-remoto.md`: el teléfono marca hacia afuera, por el punto de
encuentro del usuario, hasta `ssh <pc> meltemi bridge`. Lo que falta es un
cliente que quepa en una mano y hable ese canal.

## What Changes

- **Android primero**, con Tauri 2 móvil reutilizando `desktop/ui` (los
  tokens y el design system ya prevén el compañero: `--touch-min`, la
  maqueta) en un crate `mobile/` con las vistas del subconjunto: sesiones
  por estado, la petición que espera con sus opciones, la revisión de una
  carrera (gates, checklist, adoptar o revertir **con confirmación**, sin
  editar), y dirigir con el compositor mínimo. iOS después, con el mismo
  código y el trámite de la tienda como tarea del mantenedor.
- **Canal SSH dentro de la app**: un cliente SSH en Rust confinado al crate
  móvil (candidato `russh`, licencia y dependencias criptográficas
  verificadas en el design), con una **excepción acotada** en `deny.toml`
  que deje escrito que el workspace de escritorio sigue sin TLS ni HTTP y
  que la suite de las tres plataformas sigue verde (§7, §10). El canal es
  `exec: meltemi bridge` sobre SSH; la clave del usuario vive en el
  Keystore del sistema, generada allí o importada por él, y Meltemi jamás
  lee claves de otras apps (§2).
- **Reconexión al frente y suspensión honesta**: al volver a primer plano
  la app reconecta y reconcilia con `session/list` y `estado-en-vivo`;
  mientras está suspendida no es un cliente conectado, y la gracia
  `no-client-grace` del daemon (configurable, con techo) decide la
  denegación constitucional. El aviso de espera opt-in es lo que trae al
  usuario de vuelta; la app nunca mantiene el socket vivo en segundo plano.
- **Máquinas e identidad**: la lista de `maquinas-remotas` y la presencia de
  `identidad-propia`, con el mismo formato y sin cuentas de Meltemi.
- **Regla de subconjunto verificada**: cada RPC que la app consume tiene
  fila en `docs/paridad-nucleo.md` con casa en TUI y GUI; el gate de paridad
  lo comprueba.

## Capabilities

### New Capabilities

- Ninguna: la capability `mobile-companion` ya existe; esta change la
  implementa.

### Modified Capabilities

- `mobile-companion`: + los escenarios de implementación (Android primero,
  canal SSH confinado, suspensión y reconexión, revisión con confirmación).
- `remote-access`: + escenario «el compañero móvil habla por `exec:` sobre
  el SSH embebido».
- `release-distribution`: + el APK firmado como artefacto de release y el
  presupuesto de tamaño del compañero.

## Impact

- Archivos: `mobile/` (nuevo crate y configuración Tauri móvil),
  `desktop/ui` (vistas del subconjunto, condicionales por superficie),
  `deny.toml` (excepción acotada), `.github/workflows/` (build Android),
  `docs/acceso-remoto.md`, `docs/plataformas.md`, `rumbo/structure.md`
  (`mobile/` en el monorepo).
- Dependencia nueva de peso (cliente SSH) justificada y confinada; el
  presupuesto del APK se declara antes de compilar.
- Riesgo real: el toolchain Android en CI y el tiempo de build; la
  suspensión en iOS/Android frente a la gracia; el ancho de banda de
  reconstruir por `session/log` tras cada reconexión. Todo se mide en el
  design antes de fijar promesas.

## Fuera de alcance

- Relé en la nube, push sin túnel, cuentas de Meltemi: constitución §3 y
  rumbo; exigen enmienda, y esta propuesta no la pide.
- Autoría de código o de specs desde el móvil: la spec lo prohíbe salvo
  enmienda fundacional.
- Un cliente SSH en el crate de escritorio: allí el `ssh` es el del usuario.
