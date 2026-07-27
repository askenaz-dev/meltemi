# Tasks — eventos-para-tardios

## 1. Contrato

- [x] 1.1 Método `session/watch` con sus tipos y esquema en `meltemi-proto`,
  y conformance de params y result

## 2. Daemon

- [x] 2.1 Identificador de conexión en `Peer` (`meltemi-client`)
- [x] 2.2 Hub de eventos (`events.rs`) con test de entrega: al origen sin
  suscripción, a un tercero solo si mira, a nadie más
- [x] 2.3 `acp.rs` publica en el hub; `server.rs` lleva el conjunto de
  sesiones miradas por conexión, el brazo de fan-out y el handler del método

## 3. Superficies y paridad

- [x] 3.1 Registro de la paleta TUI, registro GUI y matriz
  `docs/paridad-nucleo.md` (gate de CI)
- [x] 3.2 La TUI filtra el transcript por la sesión mostrada; la GUI se
  suscribe al abrir el detalle y se da de baja al salir

## 4. Escenarios

- [x] 4.1 e2e: un segundo cliente conecta a mitad de turno, declara mirar la
  sesión y recibe actualizaciones que no provocó; sin declarar, no recibe

## 5. Verificación

- [x] 5.1 Gates locales completos y validate del motor sobre change y verdad
  viva
