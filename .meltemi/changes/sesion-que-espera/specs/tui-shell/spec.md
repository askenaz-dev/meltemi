# tui-shell — delta

## ADDED Requirements

### Requirement: La sesión que espera se dirige desde el terminal

La TUI SHALL declarar la sesión en espera con símbolo y palabra, y su flujo de
dirección SHALL ofrecerse sobre ella como sobre una sesión activa, sin rótulo de
reanudación y sin refuerzo del daemon.

#### Scenario: El shell dice que la sesión espera

- **WHEN** una sesión queda esperando instrucciones
- **THEN** el listado SHALL declararla con símbolo y palabra
- **AND** dirigirla SHALL ofrecerse como sobre una sesión activa
