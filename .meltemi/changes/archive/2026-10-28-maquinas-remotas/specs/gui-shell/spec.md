# gui-shell — delta

## ADDED Requirements

### Requirement: Las máquinas se eligen donde se eligen los ajustes

La superficie de escritorio SHALL mostrar en Ajustes una sección de máquinas
con las que el usuario configuró —nombre, endpoint y, a pedido, la presencia
que la malla conoce—, SHALL ofrecer probar la conexión de cada una informando
versión y latencia, y SHALL permitir hacer activa a otra, reconectando sin
reiniciar la aplicación y advirtiendo del trabajo sin guardar con la misma
guarda del cambio de proyecto. La barra de estado SHALL decir en qué máquina
está. Sin máquinas configuradas, el cromo NO SHALL cambiar.

#### Scenario: Ajustes lista las máquinas con su endpoint

- **WHEN** hay máquinas configuradas y se abre Ajustes
- **THEN** la sección SHALL mostrar cada una con su nombre y su endpoint
- **AND** SHALL marcar cuál es la activa

#### Scenario: Probar una máquina dice si respondió y cuánto tardó

- **WHEN** el usuario prueba una máquina
- **THEN** SHALL informarse la versión del daemon y la latencia, o el
  diagnóstico de por qué no respondió
- **AND** la máquina activa NO SHALL cambiar por probar

#### Scenario: Cambiar de máquina avisa del trabajo sin guardar

- **WHERE** hay trabajo sin guardar y el usuario elige otra máquina
- **THEN** SHALL advertirse con la misma guarda que el cambio de proyecto
- **AND** al confirmar, la aplicación SHALL reconectar contra la nueva sin
  reiniciarse

#### Scenario: Sin máquinas configuradas el cromo no cambia

- **WHERE** no hay máquinas en la configuración de usuario
- **THEN** la barra de estado NO SHALL ganar nada
- **AND** Ajustes SHALL decir cómo se configuran, sin pedir nada
