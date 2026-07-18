# E2E manual con un agente real — hito de la Fase 0

Este es el hito de la Fase 0: **`meltemid` pilota un agente de codificación real vía ACP y ejecuta un `/propose` de extremo a extremo**. El e2e automatizado (`cargo test`) usa el `mock-agent`; este procedimiento usa un agente real instalado en tu máquina, con su propia autenticación (constitución §2, juego limpio).

## Requisitos

- Toolchain Rust del repo (`rust-toolchain.toml`).
- Un agente con soporte ACP instalado y autenticado con su propia cuenta. Opciones habituales (jul-2026):
  - **Gemini CLI**: `gemini --acp` → `agent.command = ["gemini", "--acp"]`
  - **OpenCode**: `opencode acp` → `agent.command = ["opencode", "acp"]`
  - **Kilo Code**: `kilo acp` → `agent.command = ["kilo", "acp"]`
  - **Cursor CLI**: `agent acp` → `agent.command = ["agent", "acp"]`
  - **Claude Code / Codex** (vía adaptador ACP del ecosistema):
    `agent.command = ["npx", "-y", "@agentclientprotocol/claude-agent-acp@latest"]`
    (Nota ToS: Anthropic no permite suscripciones Pro/Max en herramientas de terceros; usa el binario oficial con su login o una API key — ver `docs/research/integracion-agentes.md`.)

Detalle de disponibilidad y flags por agente: `docs/research/integracion-agentes.md`.

## Pasos

1. **Compilar** el daemon y el cliente de prueba:

   ```
   cargo build -p meltemid
   ```

   Esto produce `target/debug/meltemid` y `target/debug/meltemi-devclient`.

2. **Elegir un repositorio fixture** (nunca este repo — los e2e van contra repos temporales):

   ```
   mkdir /tmp/mi-fixture && cd /tmp/mi-fixture && git init
   ```

3. **Configurar el agente** en `.meltemi/config.toml` del fixture:

   ```toml
   [agent]
   command = ["gemini", "--acp"]   # o el que corresponda
   ```

4. **Lanzar el propose** con el cliente de prueba (arranca el daemon bajo demanda):

   ```
   target/debug/meltemi-devclient propose "add a hello command" /tmp/mi-fixture
   ```

   El cliente de prueba auto-aprueba las peticiones de permiso del agente e imprime los `session/event` a medida que llegan.

5. **Verificar el resultado**. La salida incluye `{changeName, proposalPath, status}`. El archivo `proposalPath` (`/tmp/mi-fixture/.meltemi/changes/<nombre>/proposal.md`) debe contener la propuesta redactada por el agente.

6. **Apagar** el daemon:

   ```
   target/debug/meltemi-devclient shutdown
   ```

## Qué demuestra el hito

- El daemon arranca bajo demanda desde el cliente (tarea 3.3).
- Ejecuta el binario oficial del agente con su propia auth (juego limpio).
- Completa el handshake ACP, crea sesión y envía el prompt (4.1–4.3).
- Las peticiones de permiso del agente fluyen al cliente y su decisión vuelve al agente (4.4).
- El agente rellena `proposal.md` y el turno termina sin dejar procesos huérfanos (4.5, 5.1–5.3).

## Diagnóstico

- **`agent_not_detected` / el daemon no arranca el agente**: revisa que el comando de `agent.command` (o el binario del `agent.id` elegido; `meltemi fleet` muestra qué se detecta) esté en el `PATH` y autenticado.
- **Ver el tráfico ACP**: exporta `MELTEMI_LOG=debug` antes de lanzar el daemon; el log operacional vive en el directorio de datos del usuario (`<data_dir>/logs/meltemid.log`).
- **Sesión inspeccionable**: el registro JSONL de la sesión queda en `<data_dir>/projects/<project_key>/sessions/<session_id>.jsonl` (cada evento, incluidas las decisiones de permiso).
