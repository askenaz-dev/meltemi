# remote-access — delta

## ADDED Requirements

### Requirement: Transporte por comando del usuario en el cliente

Un cliente de Meltemi SHALL aceptar un endpoint que nombre un **comando** en
lugar de un socket, lanzarlo como subproceso y hablar el protocolo por su
entrada y salida estándar, con la misma delimitación por líneas que sobre un
socket. El comando SHALL lanzarse directamente, sin intérprete de shell
intermedio.

El comando SHALL leerse únicamente del ámbito de usuario —su entorno o su
configuración personal— y NO SHALL leerse de la configuración de un
repositorio: abrir un proyecto NO SHALL poder hacer que el cliente de quien lo
abre ejecute un comando.

Meltemi NO SHALL añadir opciones al comando del usuario, NO SHALL leer,
almacenar ni reenviar material de claves o contraseñas, y NO SHALL ofrecer al
subproceso un terminal: su entrada y su salida son el canal del protocolo.
WHERE el subproceso termine antes de hablar el protocolo, el cliente SHALL
rehusar con un diagnóstico que incluya lo que ese proceso haya escrito en su
salida de error y el remedio que corresponda.

Al cerrarse el cliente, el subproceso SHALL terminar con él. WHERE la
plataforma permita atar su vida a la del cliente, el cliente SHALL atarla, de
modo que un fin abrupto tampoco deje el proceso vivo.

#### Scenario: El cliente habla por el comando que el usuario nombró

- **WHEN** el endpoint del cliente nombra un comando y ese comando pone un daemon en su stdio
- **THEN** el cliente completa el apretón de manos y opera como si hablara por un socket local
- **AND** el daemon no distingue esa conexión de cualquier otra conexión local

#### Scenario: Un repositorio no decide qué comando corre tu cliente

- **WHEN** la configuración de un repositorio declara un endpoint con forma de comando
- **THEN** el cliente lo ignora y no ejecuta nada
- **AND** el endpoint efectivo sigue siendo el del ámbito de usuario

#### Scenario: El comando se lanza sin intérprete intermedio

- **WHEN** el comando nombrado se lanza
- **THEN** el proceso hijo del cliente es el programa nombrado, no un shell que lo invoque
- **AND** las comillas dobles del comando agrupan un argumento con espacios

#### Scenario: Un comando que no llega a hablar el protocolo se rehúsa con su causa

- **WHEN** el subproceso termina antes de completar el apretón de manos
- **THEN** el cliente rehúsa nombrando el comando y lo que el proceso escribió en su salida de error
- **AND** el mensaje dice el remedio, sin que Meltemi pida ni transporte contraseña alguna

#### Scenario: Al cerrar el cliente no queda el proceso

- **WHEN** el cliente termina, ordenadamente o de golpe
- **THEN** el subproceso que lanzó SHALL haber terminado también
