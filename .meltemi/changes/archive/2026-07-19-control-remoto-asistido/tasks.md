## 1. Dirección de sesiones

- [x] 1.1 `proto/`: método `session/direct` + evento `instruction_queued` (aditivos, schema + conformance) _(design D1)_
- [x] 1.2 Cola de instrucciones por sesión en el registro; bucle de sesión multi-turno (despacha la cola como prompts sucesivos de la misma sesión ACP al concluir cada turno) _(Req: Dirección de una sesión existente; design D1/D2 — revalidar mecánica contra `acp.rs`)_
- [x] 1.3 Rama de reanudación: sesión terminada+reanudable → `load_session` + instrucción como prompt, enlazada como `resumed_from`; no dirigible → 2004 con remedio _(Req: Dirección — reanudable / rehúso)_
- [x] 1.4 CLI `direct <session-id> "<instrucción>"` _(paridad §4)_

## 2. Helper de túnel

- [x] 2.1 Composición por plataforma: comando `ssh` de reenvío del endpoint local + snippet de config + `MELTEMI_ENDPOINT` remoto (con advertencia git-bash/MSYS) _(Req: Helper de túnel auditable; design D3)_
- [x] 2.2 `--exec` lanza el `ssh` del usuario como proceso visible; Windows-servidor rehúsa honesto con diagnóstico y remedio _(Req: Helper — ejecución visible / rehúso de plataforma; design D4)_
- [x] 2.3 CLI `tunnel [user@host] [--exec]`

## 3. Frontera documentada

- [x] 3.1 `docs/acceso-remoto.md`: túnel vivo como única vía, los tres verbos remotos (monitorear/aprobar/dirigir) sobre las capacidades existentes, la ausencia de push explicada como postura (§3, rumbo) _(Req: Frontera honesta del acceso remoto)_
- [x] 3.2 Lint de docs: presencia + secciones de la frontera (patrón de `documentacion-inicial`)

## 4. Tests y calidad

- [x] 4.1 Unit: composición del comando de túnel por plataforma (UDS vs named pipe, rehúso Windows-servidor); cola FIFO por sesión
- [x] 4.2 E2e sobre fixture con mock-agent: dirigir sesión activa (encola, no interrumpe, despacha al concluir el turno, JSONL con encolado+despacho); dirigir sesión reanudable (reanuda enlazada); no dirigible rehúsa; cancelar con cola no vacía deja estado consistente _(constitución: fixtures temporales, sin red)_
- [x] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes en las tres plataformas
