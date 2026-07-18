## ADDED Requirements

### Requirement: Resolución de agente por sesión desde la flota
El daemon SHALL resolver el agente de cada sesión que nombra uno a partir de la
flota, en este orden: perfil de lanzamiento, id del catálogo (registro o
declarado por el usuario) y, en su defecto, el agente configurado del proyecto;
el binario efectivo y la fuente de la resolución MUST registrarse en el log de
sesión, de modo que jamás sea ambiguo qué binario corrió. Un id resuelto cuyo
binario no está detectado MUST rehusarse con diagnóstico y remedio, nunca
degradar en silencio a otro proveedor.

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

### Requirement: Perfiles de lanzamiento ciegos a credenciales
La configuración SHALL admitir perfiles de lanzamiento (`[[fleet.profile]]`) con
nombre, agente del catálogo y una sobrecapa de entorno que selecciona el
contexto de autenticación del binario oficial; los valores SHALL admitir
referencias `${VAR}` resueltas al lanzar y el lint de higiene MUST rehusar
valores que parezcan secretos en claro. Meltemi MUST NOT leer, almacenar ni
reenviar material secreto de los agentes: el binario se autentica solo dentro
del contexto seleccionado, y un fallo de autenticación se muestra tal cual.

#### Scenario: Perfil lanza el mismo binario con otro contexto de autenticación
- **WHEN** una sesión se lanza nombrando un perfil
- **THEN** el daemon SHALL lanzar el binario del agente subyacente con la sobrecapa de entorno aplicada
- **AND** el material de autenticación SHALL permanecer gestionado únicamente por el binario

#### Scenario: Secreto en claro rehusado por higiene
- **WHEN** un valor de entorno de un perfil parece un secreto en claro
- **THEN** la configuración del perfil SHALL rehusarse con diagnóstico
- **AND** el remedio SHALL indicar la referencia `${VAR}` resuelta al lanzar

### Requirement: Perfiles visibles en el catálogo
El listado de la flota SHALL incluir los perfiles de lanzamiento declarados, con
fuente propia, su agente subyacente y la detección del binario subyacente, en
todas las superficies por igual.

#### Scenario: fleet/list incluye los perfiles
- **WHEN** un cliente consulta el catálogo de la flota
- **THEN** cada perfil declarado SHALL aparecer con su fuente, su agente subyacente y su detección
