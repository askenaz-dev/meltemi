# permission-rules — delta

## ADDED Requirements

### Requirement: Espera humana
Una petición escalada SHALL esperar la decisión humana según la política
configurada: sin plazo mientras exista al menos un cliente conectado
(`while-connected`, el default de los flujos interactivos) o con cota en
segundos (`wait = N`; los turnos autónomos de implement usan su propia cota,
default 30). El fallo del push vivo hacia la conexión que inició la sesión
NO SHALL resolver la petición: la cola global es la única fuente de
resolución. Cuando el último cliente se desconecta, la petición SHALL
sobrevivir una gracia de reconexión configurable; si la gracia expira sin
clientes conectados, el daemon SHALL denegar de forma explícita y auditada
(`default_deny`, constitución §3). Una cota vencida SHALL seguir el camino
vigente: vencimiento visible, notificación de timeout y denegación auditada
como `timeout`.

#### Scenario: La caída de la conexión dueña no resuelve la petición
- **WHEN** la conexión que inició la sesión cae con una petición pendiente y otro cliente decide después
- **THEN** el agente SHALL recibir la opción que ese cliente eligió
- **AND** el log SHALL registrar la decisión con procedencia de cliente, no una denegación por transporte

#### Scenario: Espera sin plazo declarada sin plazo
- **WHEN** una petición espera bajo `while-connected`
- **THEN** la consulta de pendientes SHALL declararla sin plazo (`expiresInSeconds` ausente)
- **AND** SHALL NOT marcarla vencida mientras espera

#### Scenario: Cota configurada vence auditada
- **WHEN** la política impone una cota y nadie decide dentro de ella
- **THEN** la petición SHALL vencer visible y denegarse auditada como `timeout`

#### Scenario: Sin clientes, denegación constitucional tras la gracia
- **WHEN** el último cliente se desconecta y la gracia expira sin reconexión
- **THEN** la petición SHALL denegarse explícita y auditada como `default_deny`
- **AND** una reconexión dentro de la gracia SHALL continuar la espera sin resolver nada

## MODIFIED Requirements

### Requirement: Cola de pendientes consultable
El daemon SHALL mantener las peticiones de permiso pendientes como cola de primera
clase consultable mediante `permission/pending` (id, sesión, herramienta, resumen,
opciones, y el plazo cuando la política impone cota), y MUST notificar los cambios
de la cola a todos los clientes conectados. Una petición vencida MUST permanecer
visible como vencida en la consulta inmediata, nunca descartada en silencio; una
petición sin plazo SHALL declararse sin plazo, jamás con un vencimiento inventado.

#### Scenario: Bandeja sobrevive la reconexión
- **WHEN** un cliente se reconecta mientras existen peticiones pendientes
- **THEN** `permission/pending` SHALL devolver la cola vigente
- **AND** el indicador del cliente SHALL reflejar el conteo real

#### Scenario: Cambio notificado
- **WHEN** una petición entra o se resuelve
- **THEN** el daemon SHALL emitir la notificación de cambio a los clientes conectados
