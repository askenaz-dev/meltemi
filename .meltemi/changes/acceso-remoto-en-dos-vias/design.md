# Design — acceso-remoto-en-dos-vias

## Context

Verificado el 2026-08-09, durante la exploración con el mantenedor:

- `meltemi tunnel` rehúsa en Windows con honestidad (`tunnel.rs`, test
  `a_windows_daemon_refuses_honestly`): el endpoint es un named pipe y OpenSSH
  solo reenvía puertos TCP y sockets Unix. El escenario vivo lo cubre con un
  condicional que sigue siendo verdad: «IF la plataforma no admite el reenvío
  estándar».
- El port de OpenSSH para Windows tampoco implementa el reenvío de sockets
  Unix (descarta la vía AF_UNIX), y un puerto TCP en loopback rompería la
  historia de seguridad del pipe con ACL (cualquier proceso local podría
  conectar). Quedaba una sola salida limpia.
- La infraestructura del mantenedor existe y está viva: `rancher.askenaz.dev`
  responde detrás de Cloudflare (origen encendido), `k8s.askenaz.dev` resuelve
  directo a su IP con la API cerrada a Internet (postura correcta), ddns-go
  mantiene el dominio. Kubernetes con Rancher disponible para hospedar el
  plano de control de una malla y, en fase 3, el IdP y el notificador.
- Los tres prerrequisitos de daemon del Agent Boss están archivados
  (`espera-humana`, `sesion-esperando`, `eventos-para-tardios`) y
  `session/direct` existe: el daemon ya sabe todo lo que un cliente remoto
  necesita. Lo que falta es transporte, no capacidad.
- Ninguna change activa tiene deltas sobre `remote-access`;
  `lanzador-conversacional` (bloqueada en firma) tiene un MODIFIED sin
  archivar sobre `cli-contract` — por eso este delta no entra ahí. El verbo
  `tunnel` ya se especifica en `remote-access`, así que `bridge` pertenece al
  mismo lugar.

## Goals / Non-Goals

**Goals**: que una máquina Windows pueda ser el extremo servido del acceso
remoto; que el patrón «en dos vías» (PC y CEL en cualquier red) quede
documentado como frontera con nombres y licencias verificadas; que fase 3
arranque con las decisiones de transporte ya tomadas.

**Non-Goals**: desplegar infraestructura; tocar el daemon o el contrato; la
app móvil; login o identidad; persistir nada.

## Decisions

### D1 — El último metro es un puente stdio, no un reenvío

Tres candidatos para alcanzar el named pipe desde fuera y por qué gana el
puente:

| | Mecanismo | Por qué no / por qué sí |
| --- | --- | --- |
| A | **`meltemi bridge`**: proceso que conecta el pipe con su propio stdio; el remoto hace `ssh pc "meltemi bridge"` | **Elegida.** SSH ejecuta comandos remotos con stdio en toda plataforma; JSON-RPC delimitado por líneas sobre un stream es exactamente lo que el daemon ya habla; el proceso corre con la identidad del usuario autenticado por sshd, así que la ACL del pipe se respeta |
| B | Daemon escucha AF_UNIX en Windows (reenviable en teoría) | El port de OpenSSH para Windows no implementa el reenvío streamlocal; dependeríamos de Microsoft |
| C | Forwarder TCP en 127.0.0.1 | Un puerto loopback es alcanzable por cualquier proceso local: rompe la ACL del pipe. Es el caso que la constitución evita aunque el puerto no lo abra «el daemon» |

El puente hereda las prohibiciones del helper de túnel, escritas en su
requisito: sin sockets propios, sin transporte de red en el daemon, sin tocar
material de claves. Y una más propia: **sin TTY** — nace para correr bajo
`ssh` no interactivo, y un verbo que exige terminal ahí es un verbo roto.

Ciclo de vida: bombea hasta que stdin cierra o el endpoint cierra, lo que
llegue primero, y termina con código 0 en el cierre ordenado. Si el daemon no
está, rehúsa de inmediato con diagnóstico y remedio (arrancarlo), nunca cuelga
esperando: al otro lado del ssh no hay nadie mirando un spinner.

### D2 — En dos vías = todos marcan hacia afuera

El requisito del mantenedor («fuera de casa con el PC o con el CEL, debo poder
seguir») se satisface con una regla única: **ningún extremo recibe conexiones;
ambos marcan salientes hacia un punto de encuentro del usuario**. El PC
mantiene un túnel inverso permanente hacia el bastión (`ssh -R
2222:localhost:22`, repuesto por el propio usuario con autossh o un servicio);
el cliente remoto salta por el bastión y aterriza en el sshd del PC esté donde
esté. Los cuatro cuadrantes usan el mismo camino.

Detalle que simplifica todo: el túnel inverso reenvía **el puerto TCP del sshd
del PC** — reenvío TCP, que el OpenSSH de Windows sí hace perfectamente. El
pipe ya no viaja por ningún túnel; solo lo toca el puente, ejecutado en el
último salto, dentro del PC.

Esto es documentación, no código: el bastión es del usuario (BYO-relay). El
único deber de Meltemi es que el patrón esté escrito con el comando exacto y
sus precauciones (cuenta dedicada en el bastión, `PermitListen`, solo claves).

### D3 — BYO-network: la malla es del usuario y no exige enmienda

El ideal «PC ↔ CEL directo si se puede, hub si no» es literalmente el
comportamiento de una malla WireGuard con plano de control autohospedado
(Headscale): P2P cuando el NAT lo permite, relé del usuario cuando no, y la
red elige por conexión. Presencia de máquinas incluida — la lista de
«instancias conectadas» que el mantenedor quiere ver.

Decisión de frontera: **la malla es enrutamiento IP del usuario y el
transporte final sigue siendo su SSH hacia el socket local**, así que la spec
vigente ya la cubre y no hace falta MODIFIED. Lo que sí hace falta es decirlo
por escrito para que nadie lea ambigüedad — eso es el requisito ADDED de
patrón documentado, con la tabla de licencias verificada en la exploración:
protocolo WireGuard abierto; módulo del kernel GPLv2 (vive en el kernel del
server, no se distribuye); `wireguard-go`, cliente Windows y driver MIT;
`tailscaled`/CLI/app Android BSD-3; Headscale BSD-3; Keycloak Apache-2.0 como
IdP si se quiere pureza de licencia. Nada de esto entra al workspace: cero
crates, cero cuentas para compilar o testear (§5, §10 intactos). «WireGuard»
es marca registrada: se usa, no se rebrandea.

### D4 — Lo de fase 3 se anota, no se construye

Tres decisiones de la exploración que pertenecen al design de
`companero-movil` y quedan escritas en la doc para no perderse:

- **BYO-identity**: login con la cuenta del dominio del usuario = IdP
  autohospedado (OIDC) que emite certificados SSH de vida corta; el bastión y
  los PCs confían en la CA. Meltemi no gana cuentas jamás — la lectura
  «Meltemi con login» chocaría con el rumbo («ni servicio en la nube», sin
  lock-in) y se rechaza; la identidad vive en la infraestructura del usuario.
- **Selector multi-máquina**: «varios meltemi» no toca el daemon — cada
  `meltemid` es de su máquina; el multi-daemon es un concepto de la app
  (lista de endpoints con presencia, servida por el plano de control de la
  malla del usuario).
- **Aviso de espera**: ya especificado en `remote-access` (opt-in, mínimo,
  jamás del daemon); el k8s del usuario es su casa natural.

### D5 — Sin paridad §4 que deber, y por qué

El puente no añade método al contrato ni capacidad al daemon: es un verbo del
cliente, como `tunnel`, que cualquier superficie remota aprovecha por igual
(la GUI remota y la TUI remota hablan por el mismo endpoint puenteado). El
precedente es el propio `tunnel`, que vive en `remote-access` sin fila en la
matriz de paridad.

### D6 — Verificación honesta del tramo que CI no puede correr

Los e2e cubren el puente contra un daemon real de fixture: hablar JSON-RPC por
el stdio del proceso `meltemi bridge` y recibir respuesta — en Windows eso
ejercita el named pipe de verdad. El tramo `sshd → bridge` exige un sshd que
CI no tiene: se verifica a mano sobre el binario de release, se captura en la
nota de QA, y la doc lo declara. No se finge un sshd de juguete cuya
educación esconda el bug real (lección repetida de los adaptadores).

## Risks / Trade-offs

- **Un puente por conexión**: cada cliente remoto ejecuta su `meltemi bridge`;
  N clientes = N procesos. Correcto y deseado (aislamiento por conexión), pero
  la doc lo dice para que nadie espere multiplexación.
- **El túnel inverso permanente es disponibilidad del usuario**: bastión
  apagado = sin remoto. Se documenta como el costo de no depender de ninguna
  nube ajena.
- **sshd en el PC es superficie nueva del usuario** (no de Meltemi): la doc
  recomienda solo claves y la cuenta con lo mínimo. Meltemi no lo configura.
- **El stdio de sshd en Windows** (cmd como shell por defecto, CRLF) podría
  ensuciar un stream binario; JSON-RPC por líneas es texto y tolerante, pero
  el smoke manual debe confirmar el viaje completo antes de que la doc lo
  prometa sin nota.

## Migration Plan

Aditivo: un verbo nuevo, un remedio mejor, documentación. Nada del contrato se
mueve, ningún cliente existente cambia de comportamiento.

## Open Questions

- ¿Debería `meltemi tunnel` componer también el comando del lado cliente
  (`ssh -J bastión …`)? Se deja fuera: el helper compone lo que corre en la
  máquina del daemon; el lado cliente varía por topología y la doc lo cubre
  mejor que un formateador rígido. Si el uso real lo pide, es un delta ADDED
  futuro.
