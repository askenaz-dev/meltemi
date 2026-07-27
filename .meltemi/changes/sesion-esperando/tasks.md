# Tasks — sesion-esperando

## 1. Contrato

- [x] 1.1 `ChangeInfo` gana `gate_pending`/`gate_artifact` en `meltemi-proto`,
  declarados en `change.schema.json` (requerido / opcional), con conformance
  cubriendo con gate y sin gate

## 2. Daemon

- [ ] 2.1 Contador de esperas por sesión en `session.rs`
  (`begin_waiting`/`end_waiting`, transiciones 0↔1) con test de anidamiento
- [ ] 2.2 `acp.rs` marca y desmarca la espera alrededor del escalado, con la
  registry cableada hasta el handler
- [ ] 2.3 `navigate.rs` agrega el gate pendiente leyendo el estado del ciclo,
  con test de presencia y de ausencia honesta

## 3. Superficies

- [ ] 3.1 TUI y GUI muestran el gate pendiente en el listado de changes
  (ES/EN), con sus tests de cableado/unidad

## 4. Escenarios

- [ ] 4.1 e2e: `session/list` declara `waiting_permission` mientras la
  petición espera y `active` tras decidir

## 5. Verificación

- [ ] 5.1 Gates locales completos y validate del motor sobre change y verdad
  viva
