## ADDED Requirements

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

## MODIFIED Requirements

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
