<!-- SPDX-License-Identifier: Apache-2.0 -->

# Acceso remoto

Meltemi se controla en remoto sin abrir un solo puerto de red. El daemon
`meltemid` escucha únicamente en un socket local; un túnel SSH del propio usuario
lleva ese socket al otro extremo, y el daemon no distingue una conexión así de
una local. Esta página declara la **frontera vigente** del acceso remoto: qué se
puede hacer, cómo, y qué deliberadamente no existe.

## La frontera: túnel vivo

Todas las capacidades del daemon operan en remoto **únicamente a través de un
túnel vivo** del usuario hacia el socket local. Con el túnel establecido,
funciona todo; sin túnel, no funciona nada — no hay ni notificación ni control
alguno sin una conexión establecida.

Esto es una **postura de privacidad, no una carencia**. Habilitar control o
notificaciones sin túnel exigiría un relé en la nube o un puerto de red en el
daemon, y ambos están prohibidos por la constitución (§3, seguridad por defecto)
y por el rumbo del producto (Meltemi no es un servicio en la nube). Preferimos
que el acceso remoto sea indistinguible de estar sentado frente a la máquina —
con la misma superficie de ataque: ninguna.

## Los tres verbos remotos

Sobre un túnel vivo, cualquier cliente (TUI, GUI, y en fase 3 el compañero
móvil) ejerce los tres verbos del control remoto, todos por paridad de núcleo
(§4) y ninguno exclusivo de una sola superficie:

- **Monitorear** — `session/list` y `session/log` consultan las sesiones activas
  e históricas y su transcripción JSONL.
- **Aprobar** — la cola de permisos sobrevive a las reconexiones: una petición
  de permiso pendiente sigue ahí cuando el cliente vuelve, y se resuelve en
  remoto igual que en local.
- **Dirigir** — `session/direct` envía una instrucción a una sesión existente:
  se encola como el siguiente turno de una sesión activa, o reanuda una sesión
  terminada y reanudable. Cada instrucción queda registrada en el log (encolada,
  luego despachada), de modo que la dirección remota es auditable.

## El helper de túnel

`meltemi tunnel [user@host]` compone —usando el `ssh` que el usuario ya tiene— el
comando exacto de reenvío inverso (`ssh -R`) que lleva el endpoint local de este
daemon al host remoto, más el snippet equivalente de `~/.ssh/config` y el valor
de `MELTEMI_ENDPOINT` que el extremo remoto debe fijar. Con `--exec` lanza ese
`ssh` como proceso visible; jamás deja túneles en segundo plano sin pedirlo.

El helper es un **formateador**: no abre sockets propios, no introduce transporte
de red en el daemon y nunca genera, lee ni almacena material de claves. Se
ejecuta en la máquina donde vive el daemon.

Un daemon en Windows escucha en un named pipe, que OpenSSH estándar no sabe
reenviar; el helper lo **rehúsa con honestidad** y su remedio, en lugar de
imprimir un comando que no puede funcionar. Una máquina Windows sí puede ser el
**cliente** que ejecuta el comando contra un daemon remoto Unix. Al fijar
`MELTEMI_ENDPOINT` desde git-bash, recuerda `MSYS_NO_PATHCONV=1` para que la ruta
del socket no se transforme (ver [plataformas.md](plataformas.md)).

## Sin push sin túnel — y por qué

No existe notificación push sin conexión, ni la habrá sin una enmienda
fundacional previa. Toda propuesta de cambio que introduzca notificaciones o
control sin un túnel establecido —un relé en la nube, un puerto de red, un push
sin túnel— **se rechaza** salvo que una enmienda a la constitución la apruebe
antes. La frontera se documenta como postura, no se negocia caso por caso.
