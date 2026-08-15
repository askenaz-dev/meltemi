# acceso-remoto-en-dos-vias

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Elegible por criterio — deltas solo ADDED sobre una capability existente
> (`remote-access`), ninguna capability nueva, ningún MODIFIED ni REMOVED, y
> alcance de un día: un verbo nuevo del cliente, sus tests, y la documentación
> del patrón. **Deliberadamente sin colisión**: ninguna change activa toca
> `remote-access`, y este delta no entra a `cli-contract`, donde
> `lanzador-conversacional` tiene un delta sin archivar.

## Why

El mantenedor definió el deseable en una conversación de exploración
(2026-08-09) y cabe en tres frases suyas: «necesito que la combinación sea en
dos vías — si estoy fuera de la casa o en otra red con el PC o con el CEL
debería poder seguir avanzando»; «podría tener más de un meltemi en varias PCs
y varios CELs»; y el ideal «Meltemi PC ↔ Meltemi Mobile directo, pero si no se
puede, prefiero PC → Server ← Mobile».

Hoy el acceso remoto tiene un agujero con nombre y un patrón sin escribir.

**El agujero: Windows no puede ser el extremo servido.** `meltemi tunnel`
compone `ssh -R` del endpoint local — y en Windows ese endpoint es un named
pipe, que OpenSSH no sabe reenviar. El helper lo rehúsa con honestidad (está
pineado por test y por escenario), así que la máquina principal del mantenedor
—Windows, plataforma de primera clase— no puede recibir un cliente remoto.
La observación que desatasca: **ninguna forma de reenvío alcanza un named
pipe desde fuera; toda ruta termina en algo que corre dentro del PC**. Ese
último metro es un verbo que falta: `meltemi bridge`, que conecta el endpoint
local con su propio stdio. Con él, `ssh pc "meltemi bridge"` es un canal
JSON-RPC completo — y SSH siempre sabe ejecutar un comando remoto con stdio,
en las tres plataformas. La limitación de Windows queda arrinconada en el
único lugar donde no molesta.

**El patrón sin escribir: el punto de encuentro.** «En dos vías» se resuelve
con una sola regla — nadie recibe conexiones; todos marcan hacia afuera, a un
punto de encuentro del usuario (su bastión SSH, o su malla WireGuard con plano
de control autohospedado). Con esa regla, los cuatro cuadrantes (PC en casa o
fuera × CEL en casa o fuera) usan el mismo camino, y el ideal «directo si se
puede, hub si no» es exactamente lo que una malla WireGuard hace por conexión.
Nada de eso es código de Meltemi: es infraestructura del usuario (BYO-network,
como BYO-agent), y por eso lo que corresponde es **documentarlo como frontera
y patrón**, no implementarlo. La constitución queda intacta: el daemon sigue
ciego, el transporte final sigue siendo el SSH del usuario hacia el socket
local, y la malla es solo el enrutamiento IP por el que ese SSH viaja.

Lo que **no** entra aquí y queda anotado para fase 3 (`companero-movil`): el
login con la cuenta del dominio del usuario (BYO-identity con certificados
SSH), el selector de máquinas del compañero móvil (el multi-daemon es un
concepto de la app: cada `meltemid` sigue siendo de su máquina), y el aviso de
espera autohospedado. Esta change les pavimenta el camino; no los construye.

## What Changes

- **Verbo nuevo del cliente: `meltemi bridge`.** Conecta con el endpoint local
  del daemon de esta máquina —named pipe en Windows, socket Unix en el resto—
  y bombea stdio↔endpoint en ambas direcciones hasta que un extremo cierra.
  Sin TTY, sin sockets propios, sin tocar material de claves. Si el daemon no
  está, rehúsa con diagnóstico y remedio en vez de colgarse. Es el último
  metro de todo acceso remoto, y el primero que funciona en Windows.
- **El remedio del rehúso de Windows en `meltemi tunnel` ahora nombra el
  puente**: en vez de «no se puede», dice cómo sí (`ssh <pc> "meltemi
  bridge"`). El escenario del rehúso no cambia — Windows sigue sin admitir el
  reenvío estándar; lo que mejora es el remedio.
- **`docs/acceso-remoto.md` gana el patrón completo**: el puente; el punto de
  encuentro en dos vías (todos marcan hacia afuera, la matriz de los cuatro
  cuadrantes, el túnel inverso permanente del PC); la variante BYO-network
  (malla WireGuard con plano de control autohospedado, licencias verificadas:
  protocolo abierto, implementaciones MIT/BSD, plano de control BSD-3) con su
  frontera — nada de esa infraestructura es código ni dependencia de Meltemi;
  y las notas de fase 3 (BYO-identity, selector multi-máquina, aviso de
  espera) para que el design de `companero-movil` arranque con esto decidido.

## Capabilities

### Modified Capabilities

- `remote-access`: + dos requisitos ADDED — el puente stdio del último metro
  (el verbo, sus prohibiciones y su rehúso honesto) y el patrón del punto de
  encuentro documentado como frontera (dos vías, BYO-network, y qué es del
  usuario y qué de Meltemi). Ningún requisito existente se modifica: el
  rehúso del helper de túnel sigue siendo verdad tal como está escrito.

### New Capabilities

- Ninguna.

## Impact

- Archivos: `tui/src/` (el verbo `bridge` y el remedio del túnel),
  `core/meltemid/tests/` o `tui/tests/` (los tests del puente),
  `docs/acceso-remoto.md`, `docs/plan-de-cambios.md`.
- **Cero dependencias nuevas**: el cliente ya sabe conectar con el endpoint
  (`meltemi-client`); el puente es copiar bytes en dos direcciones con lo que
  `tokio` ya trae.
- **Cero métodos del contrato y cero cambios del daemon**: el puente es un
  verbo del cliente, como `tunnel`. No nace deber de paridad §4 — no hay
  capacidad nueva del daemon que consumir; la GUI y la TUI remotas se
  benefician por igual porque ambas hablan por el mismo endpoint.
- **Lo que solo una corrida real puede confirmar, declarado ahora**: el
  camino completo `ssh` (sshd de Windows) → `meltemi bridge` → pipe exige un
  sshd real que CI no tiene; los e2e cubren el puente contra el daemon local
  y el tramo ssh se verifica a mano y se documenta como smoke, no se finge.

## Fuera de alcance

- **Desplegar bastión, malla, IdP o notificador**: infraestructura del
  usuario. Meltemi documenta el patrón; no lo instala ni lo empaqueta.
- **El compañero móvil entero** (`companero-movil`, fase 3): la app, su
  login, su selector de máquinas. Esta change deja las notas de design que esa
  change consumirá.
- **BYO-identity (certificados SSH + IdP)**: patrón anotado en la doc; su
  materialización es del usuario hoy y de fase 3 si la app la necesita.
- **Cualquier transporte de red en el daemon**: prohibido constitucionalmente
  y esta change lo refuerza — el puente corre como proceso del usuario, el
  daemon no distingue nada.
- **Enmienda a `mobile-companion` o `remote-access` más allá de lo ADDED**: el
  transporte final sigue siendo SSH del usuario; la malla es enrutamiento IP y
  no exige bendición nueva.
