# mobile-companion — delta

## ADDED Requirements

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

Sin canal, la superficie móvil SHALL mostrar qué falta y cómo se arregla, y NO
SHALL presentarlo como un error de la aplicación ni ofrecer ninguna vía
alternativa de conexión.

#### Scenario: Sin canal

- **WHEN** no hay conexión con ningún daemon
- **THEN** la pantalla SHALL decir qué falta y cómo se arregla
- **AND** NO SHALL ofrecer relé, puerto ni cuenta
