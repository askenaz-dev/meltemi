# Tareas: Fase 0 — Fundación de Meltemi

## 1. Fundación del repositorio

- [x] 1.1 Inicializar git, `.gitignore` de Rust, `LICENSE` (Apache-2.0), `NOTICE` y política de cabeceras SPDX, más la estructura del monorepo (`core/`, `proto/`, `docs/`; `brand/` ya existe)
- [x] 1.2 Ratificar `meltemi.md` (v0.2 → v1.0), `constitution.md` y los archivos de `.meltemi/rumbo/` (los borradores ya están redactados), registrando fecha y ratificador en cada documento
- [x] 1.3 Crear workspace Cargo raíz con crates `core/meltemid`, `core/mock-agent` y `proto/meltemi-proto`
- [x] 1.4 CI en GitHub Actions con matriz {ubuntu, macos, windows}: build + clippy (-D warnings) + fmt + test + cargo-deny (licencias y advisories), con `rust-toolchain.toml` pineado
- [x] 1.5 Revisar la proyección de contexto de la raíz (`AGENTS.md`, `CLAUDE.md`) tras la ratificación, manteniéndola sincronizada con constitución y rumbo

## 2. Contrato `proto/`

- [x] 2.1 JSON Schemas del protocolo daemon↔cliente: `status`, `shutdown`, `propose`, eventos de sesión y permisos, incluidos el catálogo de códigos de error (D11: `{kind, detail, remedy}`) y el esquema versionado del evento de sesión JSONL (D12)
- [x] 2.2 Tipos serde equivalentes en el crate `meltemi-proto`
- [x] 2.3 Test de conformidad que valida que los tipos serializan conforme a los schemas

## 3. Daemon `meltemid`

- [x] 3.1 Abstracción de transporte local: UDS con permisos 0700 (macOS/Linux) y named pipe con ACL de usuario (Windows), con tests por plataforma
- [x] 3.2 Bucle servidor JSON-RPC 2.0 con delimitación por líneas y errores estándar ante mensajes malformados
- [x] 3.3 Instancia única y arranque bajo demanda desde el cliente (spawn desacoplado + reconexión)
- [x] 3.4 Métodos `status` (versión, uptime, sesiones) y `shutdown` (terminación ordenada de subprocesos y logs)
- [x] 3.5 Registro JSONL apend-only por sesión en el directorio de datos del usuario
- [x] 3.6 Configuración mínima (`agent.command`) en TOML de usuario, con override por proyecto

## 4. Capa ACP

- [x] 4.1 Módulo `acp/` sobre el crate oficial (versión pineada): lanzamiento del binario configurado como subproceso stdio
- [x] 4.2 Handshake `initialize` + creación de sesión, con negociación de versión y errores informativos
- [x] 4.3 Envío de prompt y reenvío en orden del streaming de `session/update` al cliente
- [x] 4.4 Passthrough de `session/request_permission` al cliente, con denegación por defecto sin cliente conectado
- [x] 4.5 Cancelación y terminación de sesión sin procesos huérfanos (incluido el apagado del daemon)
- [x] 4.6 `mock-agent`: agente ACP de guion fijo (incluye una petición de permiso) para el e2e automatizado

## 5. Flujo `/propose`

- [ ] 5.1 Inicialización de `.meltemi/changes/` si falta + andamiaje determinista de `proposal.md` con manejo de colisiones
- [ ] 5.2 Delegación del contenido al agente ACP con CWD en la raíz del repositorio
- [ ] 5.3 Resultado estructurado (nombre, ruta, estado) y streaming de progreso hacia el cliente
- [ ] 5.4 CLI de prueba mínima (cliente JSON-RPC) que ejercita `status`, `propose` con aprobación interactiva de permisos y `shutdown`

## 6. Verificación y cierre

- [ ] 6.1 Test e2e automatizado en CI: CLI de prueba → `meltemid` → `mock-agent` → permiso → proposal.md generado
- [ ] 6.2 E2E manual documentado en `docs/` con un agente ACP real instalado — el hito de la Fase 0
- [ ] 6.3 Verificación de la implementación contra las specs de esta change (escenario por escenario) y ajustes finales
