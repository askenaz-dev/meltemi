# daemon-lifecycle — delta

## MODIFIED Requirements

### Requirement: Arranque bajo demanda e instancia única
El daemon `meltemid` SHALL arrancar bajo demanda cuando un cliente lo invoque
sobre el endpoint propio del usuario en esta máquina y no exista una instancia
en ejecución, y SHALL garantizar que nunca corren dos instancias para el mismo
usuario. Un cliente MUST NOT arrancar un daemon en un endpoint explícito que no
es el propio —uno reenviado por un túnel, uno de otra herramienta— salvo que
se le pida expresamente arrancar ahí: si ese endpoint no responde, SHALL
rehusar de inmediato con diagnóstico y remedio, sin esperar un arranque que no
va a intentar.

#### Scenario: Primer cliente arranca el daemon
- **WHEN** un cliente intenta conectarse a su endpoint propio y no hay ninguna instancia de `meltemid` en ejecución
- **THEN** el cliente arranca `meltemid` de forma desacoplada y completa su conexión contra la nueva instancia

#### Scenario: Segunda invocación reutiliza la instancia
- **WHEN** un cliente intenta conectarse y ya existe una instancia de `meltemid` en ejecución
- **THEN** la conexión se establece contra la instancia existente y no se crea un segundo proceso

#### Scenario: Un endpoint ajeno que no responde no arranca nada
- **WHEN** un cliente intenta conectarse a un endpoint explícito que no es el propio y nada responde
- **THEN** el cliente SHALL rehusar de inmediato con diagnóstico y remedio
- **AND** NO SHALL arrancar un daemon, aunque tenga el binario a mano

#### Scenario: Arrancar en un endpoint explícito se pide
- **WHEN** se pide expresamente arrancar en un endpoint explícito y nada responde en él
- **THEN** el cliente SHALL arrancar `meltemid` en ese endpoint y completar su conexión
