## 1. Config e higiene

- [x] 1.1 `[[mcp.servers]]` en config (proyecto+global, precedencia por nombre; stdio/http; env por referencia) _(Req: Declaración única)_
- [x] 1.2 Lint de higiene: detector de secretos en claro con diagnóstico y remedio _(Req: Higiene de secretos)_

## 2. Inyección

- [x] 2.1 Capacidad MCP del agente desde el handshake ACP; inyección en la creación de sesión cuando anuncia soporte _(Req: Inyección negociada)_
- [x] 2.2 Degradación honesta visible cuando no hay soporte (evento + superficie) _(Req: Inyección — degradación)_

## 3. Visibilidad

- [x] 3.1 Evento `mcp_injected` (nombres) en el JSONL; detalle de Sesión lo muestra; atributo de soporte MCP en `fleet/list` _(Req: Visibilidad y registro)_

## 4. Tests y calidad

- [x] 4.1 Unit: precedencia por nombre, detector de secretos (positivos/negativos), nunca-valores-en-log
- [x] 4.2 E2e con mock-agent extendido para anunciar/aceptar MCP: inyección presente; sin anuncio → sesión sin servidores + declaración visible
- [x] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
