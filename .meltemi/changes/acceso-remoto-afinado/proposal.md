# acceso-remoto-afinado

> Vía rápida (fast-forward): dos correcciones de una línea cada una, una
> sección de documentación que faltaba y un smoke que nunca se corrió. Deltas
> solo ADDED sobre `remote-access` y `daemon-lifecycle`. Nace del comparativo
> con Orca y de la prueba integrada del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas R3, R9, R4, MOB-6
> y R-V1). Absorbe las propuestas nombradas allí como
> `arranque-solo-en-endpoint-propio`, `tunel-resiliente` y
> `acceso-remoto-documentado`.

## Why

Tres cosas pequeñas que juntas hacen que el acceso remoto se sienta frágil,
y una que nunca se midió:

1. **El cliente arranca un daemon donde no debe.** `connect_or_start`
   (`core/meltemi-client/src/bootstrap.rs:33-53`) intenta conectar y, si
   falla, lanza `meltemid` y reintenta cinco segundos — sin mirar si el
   endpoint era el de esta máquina. Con `MELTEMI_ENDPOINT` apuntando a un
   socket reenviado por un túnel caído, el remedio correcto es «revisa el
   túnel»; el actual es arrancar un daemon local sobre ese nombre. El
   2026-09-24 ocurrió literalmente: un cliente sin las variables fijadas
   arrancó un daemon en el endpoint por defecto del usuario.
2. **El túnel no sabe que murió.** El helper compone `ssh -N -R …`
   (`tui/src/tunnel.rs:77`) sin `ServerAliveInterval` ni
   `ExitOnForwardFailure`: un túnel que perdió la red sigue vivo como
   proceso y muerto como camino, y un reenvío que falló al arrancar no avisa.
3. **La documentación dice dos cosas.** `docs/plataformas.md` recomienda
   `-L` mientras el helper imprime `-R`; no hay sección para Windows como PC
   servido (OpenSSH Server + `meltemi bridge`, el tramo que el smoke del
   2026-08-15 dejó declarado sin medir); nadie cuenta que desde un teléfono
   **ya** se puede hoy (Termius/Blink/Termux + `ssh -t pc meltemi`, 80×24, y
   `MELTEMI_BELL=1` que esas apps convierten en notificación local); y los
   ajustes de espera (`[permissions] wait`, `no-client-grace`) existen sin
   página.
4. **GUI y TUI sobre un socket reenviado nunca se han medido**: `docs/qa/` no
   tiene un solo smoke con túnel. `docs/acceso-remoto.md:12-15` promete que
   «con el túnel establecido, funciona todo»; es una expectativa.

## What Changes

- **Arranque solo sobre el endpoint propio**: `connect_or_start` lanza el
  daemon únicamente cuando el endpoint es el por defecto de este usuario en
  esta máquina; con un `MELTEMI_ENDPOINT` explícito que no responde, rehúsa
  de inmediato con diagnóstico y remedio («`<endpoint>` no acepta conexiones;
  si es un túnel, revísalo; si es local, arráncalo con `meltemi status` sin
  la variable»). El e2e que hoy fija `MELTEMI_ENDPOINT` para aislarse gana una
  bandera explícita de «arranca aquí» en vez de depender del accidente.
- **Túnel con latido**: el comando y el snippet de configuración del helper
  ganan `-o ServerAliveInterval=30 -o ServerAliveCountMax=3
  -o ExitOnForwardFailure=yes`, con la nota de qué hace cada uno; `--exec`
  reporta la salida del `ssh` cuando muere en vez de callar.
- **`docs/acceso-remoto.md`** gana tres secciones: «Windows como PC servido»
  (instalar `OpenSSH.Server`, shell por defecto, `ssh pc meltemi bridge`, lo
  medido y lo no medido), «Desde el teléfono, hoy» (TUI por SSH, tamaño
  mínimo, campana como aviso local, lo que **no** hay: push sin túnel) y
  «Cuánto se espera al humano» (los ajustes de `[permissions]` con sus
  valores). `docs/plataformas.md` deja de contradecir al helper.
- **Smoke con túnel**: `docs/qa/<fecha>-gui-tui-sobre-tunel-smoke.md`, GUI y
  TUI contra un daemon macOS/Linux reenviado con `-R`, y el tramo
  `sshd → meltemi bridge` en Windows cuando el mantenedor habilite el
  servidor; lo que no se pueda medir queda declarado, como en el smoke
  anterior.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `daemon-lifecycle`: + requisito «Un endpoint explícito jamás arranca un
  daemon» (rehúso con remedio).
- `remote-access`: + requisitos «Túnel con latido y fallo visible» y
  «Documentación del PC Windows servido, del teléfono de hoy y de la espera
  al humano»; + escenario de smoke sobre túnel.

## Impact

- Archivos: `core/meltemi-client/src/bootstrap.rs` (+ tests),
  `tui/src/tunnel.rs` (+ tests), `docs/acceso-remoto.md`,
  `docs/plataformas.md`, `docs/qa/`.
- Cero dependencias nuevas; cero RPC.
- Riesgo: algún flujo interno cuente con que un endpoint explícito arranque
  el daemon (el e2e, `capture-desktop.ps1`). Se enumeran y se les da la
  bandera explícita en la misma change.

## Fuera de alcance

- Que un cliente propio lance el `ssh` y hable por su stdio
  (`via-ssh-del-cliente`), perfiles de máquinas (`maquinas-remotas`) y el
  aviso de espera (`aviso-de-espera-opt-in`).
- Cualquier relé o puerto de red: constitución §3.
