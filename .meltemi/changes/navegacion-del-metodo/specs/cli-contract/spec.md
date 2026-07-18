## MODIFIED Requirements

### Requirement: Gramática de subcomandos y reserva
El binario `meltemi` SHALL exponer una gramática de subcomandos estable en la que
todo subcomando reconocido es operativo y se despacha en modo scriptable; un
token fuera de la gramática MUST tratarse como error de uso. La **enumeración
autoritativa** de los subcomandos, sus argumentos y la taxonomía de códigos de
salida es la **referencia CLI generada** desde la fuente única (`cli::reference`,
publicada en `docs/referencia-cli.md`); una referencia desactualizada MUST
detectarse en el pipeline. El ciclo SDD completo es operativo: ningún subcomando
del ciclo permanece reservado.

#### Scenario: Subcomando operativo reconocido
- **WHEN** se invoca un subcomando de la gramática (p. ej. `meltemi changes`)
- **THEN** el binario SHALL despacharlo en modo scriptable

#### Scenario: Subcomando desconocido
- **WHEN** se invoca `meltemi` con un token que no pertenece a la gramática
- **THEN** el binario SHALL emitir un error de uso por stderr y terminar con el código de error de uso

#### Scenario: La referencia es la enumeración autoritativa
- **WHEN** la gramática gana o cambia un subcomando sin regenerar la referencia
- **THEN** la verificación de frescura del pipeline SHALL fallar señalándolo

### Requirement: Taxonomía de códigos de salida
El binario SHALL usar una taxonomía de códigos de salida estable: `0` éxito, `1`
error interno inesperado, `2` error de uso, `10` daemon inalcanzable, `11` error
de contrato, `12` operación rechazada por política, `13` operación cancelada,
`14` validación con hallazgos. Todo subcomando MUST terminar con el código que
corresponde a su desenlace.

#### Scenario: Éxito termina en cero
- **WHEN** un subcomando operativo completa su cometido sin error
- **THEN** el binario SHALL terminar con el código `0`

#### Scenario: Daemon inalcanzable
- **WHEN** un subcomando respaldado por RPC no logra conectar ni arrancar el daemon
- **THEN** el binario SHALL terminar con el código `10`

#### Scenario: Respuesta de error del contrato
- **WHERE** el daemon responde con un error JSON-RPC o rechaza la versión de protocolo
- **THEN** el binario SHALL terminar con el código `11`

#### Scenario: Validación con hallazgos distinguible
- **WHEN** una validación concluye con diagnósticos
- **THEN** el binario SHALL terminar con el código `14`
- **AND** una validación limpia SHALL terminar con el código `0`
