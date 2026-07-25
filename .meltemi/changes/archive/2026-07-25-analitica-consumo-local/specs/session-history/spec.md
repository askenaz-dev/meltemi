## ADDED Requirements

### Requirement: Evento local de uso en ejecuciones headless
WHERE una ejecución de nivel 3 emite contadores de uso en su salida estructurada
oficial, el daemon SHALL persistirlos en el log JSONL de la sesión como un evento
de uso propio, con el origen del dato y el modelo cuando la salida lo declare,
sin alterar los eventos ya existentes ni la versión del envelope. Los contadores
que la salida no declara MUST quedar ausentes del evento — nunca en cero — y el
evento MUST NOT contener credenciales, cabeceras, cookies ni identificadores de
la cuenta del proveedor. WHERE la sesión corre sobre ACP, cuyo protocolo no
transporta uso, ningún evento de uso SHALL fabricarse.

#### Scenario: Contadores de uso persistidos desde la salida oficial
- **WHEN** una ejecución de nivel 3 emite en su salida estructurada la línea con contadores de uso
- **THEN** el daemon SHALL registrar un evento de uso en el log JSONL de la sesión
- **AND** el evento SHALL declarar el origen del dato junto a los contadores

#### Scenario: Contador ausente no se registra en cero
- **WHERE** la salida estructurada declara solo parte de los contadores
- **THEN** el evento SHALL registrar únicamente los declarados
- **AND** los no declarados MUST quedar ausentes del evento

#### Scenario: Sesión ACP no fabrica evento de uso
- **WHERE** la sesión corre sobre ACP
- **THEN** el log SHALL quedar sin evento de uso
- **AND** la contabilidad SHALL leerlo como no reportado por el protocolo

#### Scenario: El evento de uso no transporta identidad de la cuenta
- **WHEN** se registra un evento de uso
- **THEN** SHALL contener únicamente contadores, origen y modelo
- **AND** MUST NOT contener credenciales ni identificadores de la cuenta del proveedor
