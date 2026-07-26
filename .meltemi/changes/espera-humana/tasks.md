# Tasks — espera-humana

## 1. Contrato

- [x] 1.1 `PendingPermission.expires_in_seconds` → `Option<i64>` en
  `meltemi-proto`, fuera de `required` en `permission.schema.json`, y
  conformance cubriendo presencia y ausencia

## 2. Daemon

- [x] 2.1 `[permissions]` en `config.rs` (`wait`, `implement-wait`,
  `no-client-grace`) con diagnósticos de valor inválido y tests de parseo
- [x] 2.2 Registro de clientes inicializados (contador + watch) en el ciclo
  de conexión de `server.rs`
- [x] 2.3 `pending.rs` con plazo opcional y `acp.rs` escalando por política:
  el push no resuelve en fallo de transporte, la gracia sin clientes deniega
  `default_deny`, la cota vencida deniega `timeout`; los cuatro flujos de
  `server.rs`/`propose.rs`/`sdd_flow.rs` pasan la política desde la config

## 3. Superficies

- [ ] 3.1 TUI y GUI renderizan la espera sin plazo («esperando tu decisión»,
  ES/EN) y sus tests de cableado/unidad se actualizan

## 4. Escenarios

- [ ] 4.1 e2e en `e2e_permisos.rs`: los cuatro escenarios del delta «Espera
  humana» contra el mock-agent y config de fixture con cotas cortas

## 5. Verificación

- [ ] 5.1 Gates locales completos + revisión adversarial multi-agente del
  diff; validate del motor sobre change y verdad viva
