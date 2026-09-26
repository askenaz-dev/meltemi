# extension-acp-usage

> Vía completa (proposal → design → specs → tasks): define una extensión ACP
> con espacio de nombres propio, la emiten los dos adaptadores y la pliega
> el daemon; el design escribe además la propuesta upstream. Depende de
> `usage-desde-acp`. Estaba nombrada sin abrir en el backlog; nace ahora del
> comparativo con Orca del 2026-09-24
> (`docs/research/comparativo-orca-2026-09-24.md`, brechas LM-7 y AI-6).

## Why

Orca muestra «58 % 5h · 41 % wk» por proveedor y permite cambiar de cuenta
en caliente. Lo consigue llamando a las API de los proveedores con los
tokens de los CLI del usuario — exactamente lo que la constitución §2
prohíbe («leer, almacenar o reutilizar credenciales de agentes»). La regla
de `local-analytics` es la contraria y es la correcta: tokens solo donde la
salida oficial del agente los reporta.

`usage-desde-acp` pliega lo que llega por `usage_update` (OpenCode ya lo
envía). Los dos adaptadores propios no lo envían, y **sí lo saben**: Claude
Code reporta uso en su salida estructurada y el app-server de Codex lo
reporta en sus eventos de turno. Hoy esa información muere en el adaptador
y `meltemi usage` dice «no reportado» para las sesiones de los dos agentes
más usados. No hay que pedir nada a nadie: el adaptador lee la salida
oficial que ya lee y la transporta por ACP.

## What Changes

- **Una extensión con espacio de nombres**: `_meta["meltemi.dev/usage"]`
  en el `usage_update` (si el crate pineado lo tipa) o en un `session update`
  de extensión, con tokens de entrada y salida, contexto usado y tamaño, y
  costo cuando el CLI lo reporta; solo lo que la salida oficial trae, nada
  estimado. El design fija el esquema JSON y lo versiona.
- **Los dos adaptadores la emiten** desde la salida oficial del CLI que ya
  parsean; la conformidad manual mide que el número coincide con lo que el
  CLI muestra por su cuenta.
- **El daemon la pliega** con el mismo camino de `usage-desde-acp`, y las
  tres superficies la muestran con la frontera honesta: «reportado por el
  CLI» frente a «no reportado».
- **Propuesta upstream**: un documento breve en `docs/research/` con la
  forma de la extensión y por qué ACP debería tener un campo estándar de
  uso, para llevarlo al proyecto del protocolo (constitución §6: estándares
  abiertos primero; una extensión documentada que cualquier agente pueda
  implementar, nunca un canal privilegiado).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `own-adapters`: + requisito «Uso reportado por el CLI viaja como extensión
  ACP con espacio de nombres».
- `local-analytics`: + fuente «adaptador propio» en la cobertura, con su
  motivo.
- `integration-levels`: + comprobación de coincidencia con la salida del
  CLI.

## Impact

- Archivos: `core/meltemi-adapters/src/{claude,codex}/`,
  `core/meltemid/src/analytics.rs`, `proto/schemas/v1/analytics.schema.json`
  (aditivo), `docs/conformidad-manual.md`, `docs/research/` (propuesta
  upstream).
- Cero dependencias nuevas.
- Riesgo: la salida oficial de un CLI cambia de forma con la versión; el
  adaptador ya vive con esa realidad (rango soportado) y el uso entra bajo
  la misma disciplina: leer o declarar «no reportado», nunca adivinar.

## Fuera de alcance

- Límites de cuota, «resets» de rate limit o saldos leídos de una API con el
  token del CLI: §2, y `local-analytics` lo declara
  (`no-quota-balance-or-billing`). Solo si el CLI los imprime.
- Cambiar de cuenta en caliente: los perfiles de `vincular-suscripciones`
  eligen la cuenta al abrir sesión.
