# Design — enmienda-agent-boss

## Context

La proposal registra la auditoría (2026-07-26) que motivó esta enmienda: el
daemon ya sirve la mayor parte del caso Agent Boss, cinco fricciones
verificadas lo sabotean, y la pregunta de notificaciones que
`enmienda-edicion-movil` dejó abierta necesita postura. Este design fija las
decisiones para que la fase 3 no las relitigue.

## Decisions

### D1 — Cuatro verbos; revisar es decidir, no redactar
El verbo nuevo **revisar** cubre leer los diffs de una carrera y decidir sobre
el trabajo: gates, checklist de revisión, y adopción o reversión de archivos
con confirmación explícita (`worktree/merge-file`, `checkpoint/revert`). La
línea que lo separa de la autoría es operativa, no retórica: una decisión
elige entre resultados existentes bajo confirmación y queda trazada; la
autoría produce contenido nuevo (`worktree/apply-edit`, edición libre) y sigue
excluida del móvil para siempre. Sin esta precisión, `merge-file` quedaba en
tierra de nadie — escribe al árbol, y la exclusión de "edición" invitaba a
vetarlo o a colarlo según conviniera.

### D2 — Aviso de espera: opt-in, autohospedado, contenido mínimo, jamás el daemon
El Agent Boss necesita enterarse de que algo espera su decisión. La postura:
un cliente conectado o un proceso del propio usuario MAY emitir un aviso
mínimo ("una decisión espera") a un endpoint que el usuario opera; desactivado
por defecto; contenido techado por spec — sin proyecto, sin petición, sin
código (constitución §9: especificado antes de existir). El daemon jamás lo
emite: §3 queda intacto y verificable (el daemon sigue sin enlazar red). Se
rechazó el push de plataforma (APNs/FCM directo) porque exige identidad de
cuenta y relé de terceros; el usuario que quiera puentear su aviso
autohospedado hacia su plataforma lo hace fuera de Meltemi, bajo su control.

### D3 — Prerrequisitos como changes de daemon, paridad ×3
Las tres fricciones de daemon se resuelven antes de que exista el móvil, como
changes propias que sirven a TUI y GUI desde el día uno:
- **`espera-humana`**: política de espera configurable — una petición con cola
  viva espera al humano en vez de default-deny a los 120 s, y la caída de la
  conexión dueña no la resuelve (hoy: denegación instantánea, `acp.rs:508`).
- **`sesion-esperando`**: el daemon setea `waiting_permission` de verdad
  (superficie muerta del contrato hoy) y `change/list` expone `gatePending`.
- **`eventos-para-tardios`**: suscripción al stream de eventos de sesiones que
  el cliente no inició, y formas asíncronas (ack + eventos) de los RPC que hoy
  bloquean el turno entero (`sdd/gate`, `sdd/review-decide`,
  `worktree/dispatch`).

### D4 — La frontera Windows se nombra, no se resuelve aquí
El helper de túnel rehúsa en Windows porque OpenSSH no reenvía named pipes —
hoy no hay historia de túnel contra un daemon Windows, la plataforma primaria
del mantenedor. Resolverlo es decisión de design de la change de fase 3, con
candidatos anotados (AF_UNIX en Windows 10+, forwarder local user-run) y una
prohibición fija: jamás un puerto de red del daemon.

### D5 — Web queda fuera, con la puerta descrita
Un cliente navegador no habla el transporte (JSONL sobre socket local) ni con
túnel; exigiría un bridge WS↔socket corrido por el usuario, y el sitio
estático no puede ser PWA (su spec prohíbe JavaScript). Si la demanda existe,
entra por enmienda propia; esta deja constancia de que el bridge jamás será el
daemon.
