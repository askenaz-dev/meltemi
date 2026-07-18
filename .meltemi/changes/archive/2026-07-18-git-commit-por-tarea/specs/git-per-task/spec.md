## ADDED Requirements

### Requirement: Commit atómico por tarea completada
El daemon SHALL producir un commit por tarea completada en el worktree de la
tarea: en modo supervisado MUST presentarse al humano (mensaje y resumen del
diff) antes de aplicarse, con posibilidad de editar el mensaje o rechazar; en
modo autónomo SHALL aplicarse directamente dentro de las reglas vigentes. Todo
commit SHALL registrarse como evento en el log de sesión.

#### Scenario: Supervisado propone antes de cometer
- **WHEN** una tarea se completa en modo supervisado
- **THEN** el shell SHALL presentar el commit propuesto para aprobar, editar o rechazar
- **AND** nada SHALL cometerse sin la decisión

#### Scenario: Autónomo comete y registra
- **WHEN** una tarea se completa en modo autónomo
- **THEN** el commit SHALL aplicarse y el evento SHALL quedar en el JSONL

### Requirement: Trailers de trazabilidad
Cada commit por tarea SHALL llevar los trailers `Meltemi-Task` (change y tarea) y
`Meltemi-Req` (capacidad y requisito, uno por requisito cubierto), y MUST NOT
llevar trailers de co-autoría: la autoría es exclusivamente la configuración git
del usuario, que el daemon MUST NOT alterar.

#### Scenario: Trazabilidad hasta el requisito
- **WHEN** se comete la tarea 2.1 de una change
- **THEN** el commit SHALL incluir `Meltemi-Task` con la change y la tarea
- **AND** `Meltemi-Req` con los requisitos que la tarea implementa

#### Scenario: Sin co-autoría jamás
- **WHEN** se genera cualquier commit por tarea
- **THEN** el mensaje SHALL NOT contener trailers de co-autoría

### Requirement: Convención de mensaje garantizada
El daemon SHALL generar el mensaje conforme a la convención: título imperativo en
inglés, cuerpo con el qué y el porqué, y la referencia de change y tarea; el
agente puede proponer contenido, pero la forma MUST garantizarla el daemon.

#### Scenario: Forma garantizada sobre propuesta del agente
- **WHEN** el agente propone un cuerpo de mensaje
- **THEN** el commit final SHALL cumplir la convención de título, cuerpo y referencia

### Requirement: Atomicidad verificada
Tras cada commit, el daemon SHALL verificar que el árbol del worktree queda
limpio y que el commit contiene únicamente rutas tocadas por la tarea (contra su
checkpoint como base); toda desviación MUST reportarse de forma visible, nunca
silenciosa. Un fallo de hooks del usuario SHALL mostrarse tal cual y MUST NOT
sortearse.

#### Scenario: Desviación declarada
- **IF** el commit incluye rutas ajenas a la tarea
- **THEN** el daemon SHALL reportar la desviación con las rutas
- **AND** SHALL ofrecer el paso correctivo sin ocultarla

#### Scenario: Hooks respetados
- **IF** un hook del usuario rechaza el commit
- **THEN** el fallo SHALL presentarse tal cual
- **AND** el daemon SHALL NOT reintentar sorteando hooks
