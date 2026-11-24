# mobile-companion Specification

## Purpose
El puesto remoto del Agent Boss: la superficie compañera con la que el usuario monitorea, aprueba, revisa y dirige su flota desde fuera de la oficina, sin autoría de código ni de specs, siempre a través de su propio túnel SSH. Gobierna la change de fase 3 que la implemente.
## Requirements

### Requirement: Alcance del compañero móvil
La superficie móvil de Meltemi SHALL ser el puesto remoto del **Agent Boss**:
una superficie compañera limitada a cuatro verbos — **monitorear** (estado de
la flota, sesiones, tareas y lo que espera decisión), **aprobar** (peticiones
de permiso y planes pendientes), **revisar** (leer los diffs de una carrera y
decidir sobre el trabajo: gates, checklist de revisión, adopción o reversión
de archivos con confirmación explícita) y **dirigir** (enviar instrucciones a
sesiones existentes). Revisar es decidir, no redactar: la superficie móvil
NO SHALL ofrecer autoría de código ni de specs (edición libre de contenido);
adoptar o revertir bajo confirmación es una decisión gobernada y trazable, no
edición. Toda ampliación de este alcance exige enmienda fundacional previa.

#### Scenario: Aprobación de una petición de permiso desde el móvil
- **WHEN** un agente solicita un permiso y el usuario responde desde la superficie móvil
- **THEN** la aprobación o denegación es explícita por petición, con el mismo efecto que si se emitiera desde la TUI o la GUI

#### Scenario: Adopción de un archivo de un competidor desde el móvil
- **WHEN** el usuario revisa una carrera desde la superficie móvil y adopta el archivo de un competidor
- **THEN** la adopción exige confirmación explícita y queda trazada como cualquier decisión de merge
- **AND** la superficie móvil NO SHALL ofrecer editar el contenido adoptado

#### Scenario: Propuesta de autoría en el móvil
- **WHEN** una propuesta de cambio introduce autoría de código o specs (edición libre de contenido) en la superficie móvil
- **THEN** la propuesta se rechaza salvo enmienda fundacional aprobada que amplíe el alcance del compañero

### Requirement: Regla de subconjunto respecto de TUI y GUI
Toda capacidad del daemon consumida por la superficie móvil SHALL estar disponible también desde la TUI y la GUI. La superficie móvil NO SHALL ser la única consumidora de ninguna capacidad del daemon (constitución §4: ninguna funcionalidad del daemon accesible desde una sola superficie).

#### Scenario: Nueva capacidad del daemon motivada por el móvil
- **WHEN** una propuesta de cambio añade al daemon una capacidad destinada a la superficie móvil
- **THEN** la propuesta incluye su consumo desde la TUI y la GUI, o se rechaza

### Requirement: Acceso remoto exclusivamente por túnel SSH
La superficie móvil SHALL conectarse al daemon únicamente a través de un túnel SSH del usuario que desemboque en el socket local (constitución §3). El daemon NO SHALL abrir transporte de red ni distinguir una conexión tunelizada de una local.

#### Scenario: Conexión del compañero móvil
- **WHEN** la superficie móvil establece conexión con `meltemid`
- **THEN** la conexión llega al socket local a través de un túnel SSH gestionado por el usuario, y el daemon la atiende como a cualquier cliente local

#### Scenario: Propuesta de transporte de red para el móvil
- **WHEN** una propuesta de cambio requiere que el daemon escuche en un puerto de red o dependa de un servicio de retransmisión en la nube para servir al móvil
- **THEN** la propuesta se rechaza (constitución §3 y rumbo tech: sin puertos de red, jamás; sin servicio en la nube)

### Requirement: El compañero habla SSH por sí mismo, con una clave suya

La superficie móvil SHALL hablar el canal SSH ella misma —no hay un `ssh` del
sistema que lanzar en el teléfono— y SHALL desembocar en el socket local del
daemon por el mismo comando puente que usa cualquier cliente remoto. El cliente
SSH SHALL estar confinado al crate móvil y NO SHALL entrar al crate de
escritorio, donde el `ssh` sigue siendo el del usuario.

La superficie móvil SHALL generar su propio par de claves en el teléfono y
mostrar la pública para que el usuario la autorice; NO SHALL leer, importar ni
pedir claves de otras aplicaciones ni del usuario, y SHALL guardar la privada
cifrada en reposo con material del almacén de claves del sistema. La clave del
host SHALL fijarse en el primer uso y comprobarse después; un cambio SHALL
cortar la conexión y decirlo, y NO SHALL aceptarse en silencio.

#### Scenario: El canal del teléfono termina en el socket local

- **WHEN** la app móvil conecta con el daemon
- **THEN** la conexión SHALL llegar al socket local a través de SSH y del
  comando puente
- **AND** el daemon NO SHALL distinguirla de un cliente local

#### Scenario: La clave es de la app y de nadie más

- **WHEN** la app necesita una clave para conectarse
- **THEN** SHALL generar la suya en el teléfono y mostrar la pública para
  autorizarla
- **AND** NO SHALL leer ni pedir la clave privada del usuario ni la de otra
  aplicación

#### Scenario: La clave del host cambia

- **WHERE** la clave del host no es la fijada en el primer uso
- **THEN** la conexión SHALL cortarse con un diagnóstico que lo diga
- **AND** NO SHALL continuar ni aceptarla en silencio

### Requirement: Suspendida no es conectada

La superficie móvil NO SHALL mantener la conexión viva mientras está en segundo
plano, y NO SHALL presentarse como cliente conectado cuando no lo está: el
deny-by-default del daemon depende de que haya alguien mirando. Al volver a
primer plano SHALL reconectar y reconciliar el estado con la lista de sesiones
y sus afirmaciones. Lo que trae al usuario de vuelta SHALL ser el aviso de
espera opt-in, que sale de la infraestructura del propio usuario.

#### Scenario: La app vuelve al frente

- **WHEN** el usuario vuelve a la app tras haberla dejado en segundo plano
- **THEN** SHALL reconectar y reconciliar sesiones y estados antes de mostrar
  nada como actual
- **AND** lo que vea SHALL ser el estado del daemon, no el que tenía guardado

#### Scenario: La app está en segundo plano

- **WHERE** la app no está en primer plano
- **THEN** NO SHALL sostener el socket
- **AND** NO SHALL contarse como cliente conectado para las decisiones del
  daemon

### Requirement: Sin túnel, la pantalla dice qué falta

Sin canal, la superficie móvil SHALL mostrar qué falta y cómo se arregla —y
SHALL ofrecer ahí mismo el camino: la clave pública de la app, el destino y
conectar—, y NO SHALL presentarlo como un error de la aplicación ni ofrecer
ninguna vía alternativa de conexión.

#### Scenario: Sin canal

- **WHEN** no hay conexión con ningún daemon
- **THEN** la pantalla SHALL decir qué falta y cómo se arregla, con la clave,
  el destino y conectar en la misma pantalla
- **AND** NO SHALL ofrecer relé, puerto ni cuenta

### Requirement: El emparejamiento vive en la app

La superficie móvil SHALL ofrecer, sin conexión, todo lo que hace falta para
abrirla: la clave pública de la app lista para copiar, un campo para el
destino `usuario@host[:puerto]` y la acción de conectar. El destino SHALL
recordarse entre arranques y poder cambiarse u olvidarse desde Ajustes; la
clave pública SHALL verse también desde Ajustes. NO SHALL pedir contraseña,
token ni clave ajena.

#### Scenario: La clave se ve y se copia sin conexión

- **WHEN** la app no tiene conexión y nunca se emparejó
- **THEN** muestra la línea de `authorized_keys` de su propia clave y un
  control para copiarla
- **AND** no hay ningún campo para pegar una clave privada

#### Scenario: El destino se escribe una vez

- **WHEN** el usuario escribe `usuario@host[:puerto]` y conecta
- **THEN** la app lo recuerda y al volver al frente conecta sola a ese destino
- **AND** el destino puede cambiarse u olvidarse desde Ajustes

#### Scenario: Conectar abre el túnel y la pantalla deja de prometer

- **WHEN** el usuario conecta y el servidor acepta la clave
- **THEN** la app queda conectada por SSH al puente y muestra las sesiones del
  daemon
- **AND** el texto de «sin túnel» no apunta a un lugar que no existe

#### Scenario: Una clave rechazada no se reintenta en bucle

- **WHEN** el servidor rechaza la clave de la app, o la clave del host cambió
- **THEN** la app lo muestra como su propio estado, con la causa y qué hacer
- **AND** no entra en reintentos con backoff hasta que el usuario actúe

### Requirement: Un puente, compartido

El actor que posee la única conexión al daemon —marcar, `initialize`,
reconexión con backoff, paso de peticiones y reenvío de lo que el daemon
empuja— SHALL ser uno solo para el escritorio y el compañero, en el cliente
compartido, y SHALL diferir únicamente en quien marca: el endpoint local o de
comando en el escritorio, el canal SSH de la app en el teléfono. El contrato
hacia la superficie (`daemon_request`, `daemon_notify`, `daemon:state`,
`daemon:incoming`) SHALL ser el mismo en ambas.

#### Scenario: El escritorio y el compañero corren el mismo actor

- **WHEN** se compila cualquiera de las dos superficies
- **THEN** el actor del puente viene del cliente compartido y no de una copia
  por superficie
- **AND** lo que cambia entre ellas es el marcador, no el bucle

#### Scenario: Un fallo de transporte reintenta con backoff

- **WHEN** marcar falla porque el otro extremo no responde
- **THEN** el actor reintenta con backoff y publica `unreachable` con el
  detalle
- **AND** una petición que llega durante la espera falla rápido como
  inalcanzable

#### Scenario: Un rechazo detiene el actor y lo publica

- **WHEN** marcar falla porque el servidor rechaza la clave o la clave del
  host cambió
- **THEN** el actor publica el estado `refused` con su causa y se detiene
- **AND** no vuelve a marcar hasta que el anfitrión lo arranque de nuevo

### Requirement: El canal se prueba contra un daemon real

El canal del compañero SHALL probarse de extremo a extremo —SSH, puente,
daemon— en bucle local, contra un servidor SSH de prueba del workspace que
acepta una sola clave y ejecuta únicamente el puente, sin `sshd` del sistema
ni red. La misma pieza SHALL servir al smoke en un dispositivo o emulador.

#### Scenario: La app alcanza un daemon real por SSH en bucle local

- **WHEN** el e2e abre el canal contra el servidor de prueba con la clave de
  la app
- **THEN** `initialize` y `status` contestan desde un `meltemid` real por el
  puente
- **AND** el servidor de prueba rehúsa cualquier comando que no sea el puente

#### Scenario: Un host reinstalado se nota en la prueba

- **WHEN** el servidor de prueba cambia de clave de host entre dos conexiones
- **THEN** el canal se rehúsa con la clave del host cambiada
- **AND** tras olvidar el host, vuelve a abrirse
