# fleet-catalog — delta

## MODIFIED Requirements

### Requirement: Resolución de agente por sesión desde la flota
El daemon SHALL resolver el agente de cada sesión que nombra uno a partir de la
flota, en este orden: perfil de lanzamiento, id del catálogo (registro o
declarado por el usuario) y, en su defecto, el agente configurado del proyecto;
el binario efectivo y la fuente de la resolución MUST registrarse en el log de
sesión, de modo que jamás sea ambiguo qué binario corrió. Un id resuelto cuyo
binario no está detectado MUST rehusarse con diagnóstico y remedio, nunca
degradar en silencio a otro proveedor. Toda negativa de resolución —id
desconocido, binario no detectado, o ausencia total de agente configurado— SHALL
devolver un error **estructurado** que, además del diagnóstico y el remedio en
prosa, lleve los **candidatos detectados** de la flota con su id, su detección,
su estado de instalación y su remedio accionable, para que cualquier superficie
pueda ofrecer elegir en vez de transcribir un lamento. Esos candidatos MUST
calcularse por el mismo camino de detección que alimenta el listado de la flota,
de modo que el error y la vista Flota no puedan discrepar, y el payload MUST NOT
incluir valores de entorno, rutas de credenciales ni ningún dato con forma de
secreto (constitución §2).

#### Scenario: Sesión lanza el binario de su id de catálogo
- **WHEN** una sesión se lanza nombrando un id del catálogo detectado en el sistema
- **THEN** el daemon SHALL lanzar el binario de ese id
- **AND** el log de sesión SHALL registrar el binario efectivo y la fuente de resolución

#### Scenario: Etiqueta libre cae al agente configurado con registro
- **WHEN** el nombre no corresponde a ningún perfil ni id del catálogo
- **THEN** la sesión SHALL usar el agente configurado del proyecto
- **AND** la resolución con fuente de fallback SHALL constar en el log de sesión

#### Scenario: Id no detectado rehúsa sin degradar
- **IF** el nombre resuelve a un id del catálogo cuyo binario no está detectado
- **THEN** el lanzamiento SHALL rehusarse con diagnóstico y remedio
- **AND** ningún otro proveedor SHALL lanzarse en su lugar

#### Scenario: La negativa trae los candidatos detectados
- **WHEN** una resolución de agente rehúsa por no haber agente configurado o por no estar detectado el nombrado
- **THEN** el error SHALL incluir los agentes detectados de la flota con su id y su estado de instalación
- **AND** cada candidato SHALL traer su remedio accionable

#### Scenario: La negativa no filtra material de autenticación
- **WHEN** el daemon compone la negativa de resolución
- **THEN** el payload SHALL limitarse a ids, detección, estado y remedios
- **AND** SHALL NOT incluir valores de entorno ni nada con forma de secreto

#### Scenario: El error y la vista Flota no discrepan
- **WHEN** un cliente compara los candidatos del error con el listado de la flota
- **THEN** el estado de instalación de cada agente SHALL coincidir en ambos
- **AND** SHALL provenir del mismo camino de detección
