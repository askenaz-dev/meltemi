## ADDED Requirements

### Requirement: Contabilidad local agregada sobre los registros de sesión
El daemon SHALL computar la contabilidad de consumo plegando exclusivamente
registros locales — el índice de sesiones y los logs JSONL por proyecto, más el
log de ediciones del proyecto cuando su raíz es alcanzable — y MUST NOT
introducir un segundo almacén de verdad: los logs siguen siendo la fuente. La
agregación SHALL organizarse en celdas de proyecto × agente × perfil × período,
donde el agente es el binario efectivo que corrió y el perfil el de la
resolución cuando existió, y toda métrica MUST derivarse de hechos registrados.
WHERE un hecho no puede atribuirse a un agente o a un perfil, MUST contabilizarse
en una cubeta explícita de no atribuido y MUST NOT repartirse entre las celdas
atribuidas.

#### Scenario: Celdas por proyecto, agente, perfil y período
- **WHEN** un proyecto acumula sesiones de dos perfiles distintos en el mismo período
- **THEN** la agregación SHALL presentar una celda por combinación de agente y perfil
- **AND** cada métrica SHALL contar únicamente los hechos de su celda

#### Scenario: Hecho sin atribución en cubeta explícita
- **WHEN** un hecho registrado no permite atribuir agente ni perfil, como una edición humana aplicada sin sesión activa
- **THEN** SHALL contabilizarse en la cubeta de no atribuido
- **AND** MUST NOT repartirse entre las celdas atribuidas

#### Scenario: Agregación sin índice de sesiones
- **WHEN** el índice de sesiones falta y existen logs JSONL del proyecto
- **THEN** la agregación SHALL computarse desde los logs
- **AND** las cifras SHALL corresponder a los hechos registrados en ellos

#### Scenario: Duración medida solo en sesiones cerradas
- **WHERE** una sesión no tiene fin registrado
- **THEN** SHALL contarse como sesión sin fin
- **AND** su duración MUST NOT extrapolarse ni sumarse a los segundos activos

### Requirement: Método de agregación de consumo por contrato
El daemon SHALL exponer el método aditivo `analytics/usage`, que computa la
agregación en el daemon y devuelve celdas con sus totales, admitiendo filtros de
proyecto, agente y perfil, rango temporal y granularidad de período (día,
semana, mes o total). WHERE la petición no nombra proyecto, SHALL agregar todos
los proyectos con registros en el directorio de datos del usuario. Ningún
cliente MUST leer el disco del daemon para obtener estas métricas, de modo que
la contabilidad funcione igual por túnel que en local, y un parámetro inválido
SHALL rehusarse con diagnóstico en vez de degradar a un período por defecto.

#### Scenario: Agregación por rango y granularidad
- **WHEN** un cliente pide la contabilidad de un proyecto con rango y granularidad mensual
- **THEN** la respuesta SHALL contener una celda por período y combinación de dimensiones dentro del rango
- **AND** los totales SHALL corresponder a la suma de las celdas devueltas

#### Scenario: Agregación multiproyecto sin filtro de proyecto
- **WHEN** un cliente pide la contabilidad sin nombrar proyecto
- **THEN** la respuesta SHALL incluir celdas de cada proyecto con registros en el directorio de datos
- **AND** cada celda SHALL identificar su proyecto

#### Scenario: Proyecto sin registros responde vacío honesto
- **WHEN** un cliente pide la contabilidad de un proyecto sin registros
- **THEN** la respuesta SHALL devolver cero celdas declarando la ausencia de registros
- **AND** MUST NOT fabricar filas en cero

#### Scenario: Parámetro inválido rehúsa
- **IF** la petición trae un rango invertido o una granularidad desconocida
- **THEN** el daemon SHALL rehusarla con diagnóstico y remedio
- **AND** MUST NOT sustituirla en silencio por un período por defecto

### Requirement: Tokens únicamente medidos, jamás estimados
La contabilidad de tokens SHALL alimentarse exclusivamente de contadores
emitidos por la interfaz oficial del agente y persistidos como evento local del
log de sesión. El daemon MUST NOT estimar tokens contando texto propio, MUST NOT
consultar cuota, saldo ni facturación de la cuenta del proveedor y MUST NOT leer
configuración ni credenciales del agente para obtenerlos. Un contador que la
salida oficial no declara MUST permanecer ausente en la agregación: la ausencia
MUST NOT rellenarse con cero.

#### Scenario: Contadores del stream oficial agregados
- **WHEN** una ejecución headless registró contadores de uso en el log de su sesión
- **THEN** la celda correspondiente SHALL sumar esos contadores medidos
- **AND** SHALL declarar cuántas sesiones los aportaron

#### Scenario: Contador no declarado permanece ausente
- **WHERE** la salida oficial no declara un desglose, como los tokens servidos desde caché
- **THEN** ese contador SHALL presentarse como ausente
- **AND** MUST NOT presentarse como cero

#### Scenario: Ninguna estimación ni consulta a la cuenta
- **WHEN** una sesión no aporta contadores medidos
- **THEN** la agregación MUST NOT estimar su consumo por conteo de texto
- **AND** MUST NOT consultar la cuenta del proveedor para completarlo

### Requirement: Frontera explícita entre medido y no reportado
La respuesta SHALL distinguir lo medido de lo no reportado con un motivo
explícito y estable por sesión sin dato — el protocolo no transporta uso, el
nivel no ejecuta proceso, o la salida estructurada no declaró contadores — y
MUST NOT sumar ambos conjuntos en una misma cifra. Todo total de tokens SHALL
declarar su cobertura, con las sesiones medidas y las sesiones sin dato, y las
superficies SHALL rotular el dato ausente con símbolo y palabra; un dato ausente
MUST NOT renderizarse como cero.

#### Scenario: Sesión ACP marcada como no reportada por el protocolo
- **WHEN** la contabilidad incluye una sesión pilotada por ACP
- **THEN** sus tokens SHALL presentarse como no reportados por el protocolo
- **AND** la celda SHALL conservar íntegras sus métricas de actividad

#### Scenario: Cobertura declarada junto al total
- **WHEN** un total de tokens se calcula sobre parte de la actividad del período
- **THEN** SHALL declarar cuántas sesiones lo respaldan y cuántas quedaron sin dato

#### Scenario: Medido y no reportado nunca comparten cifra
- **WHEN** un período combina sesiones medidas y sesiones sin dato
- **THEN** la cifra medida SHALL sumar solo las sesiones medidas
- **AND** las sesiones sin dato SHALL aparecer con su motivo, jamás como cero

### Requirement: Declaración de honestidad junto a los números
Toda superficie que presente la contabilidad SHALL mostrar, en la misma vista y
sin interacción previa, qué se mide, de qué registro sale, qué no es visible —
la cuota, el saldo y la facturación de la cuenta del proveedor — y que ningún
dato sale de la máquina. El daemon SHALL devolver esa declaración como dato
estructurado de claves estables y cada superficie MUST renderizarla desde su
catálogo de mensajes ES/EN; el agregador MUST NOT abrir conexiones de red ni
escribir métricas fuera del directorio de datos del usuario.

#### Scenario: Declaración visible con las cifras
- **WHEN** una superficie presenta la contabilidad de consumo
- **THEN** la declaración SHALL mostrarse en la misma vista que los números
- **AND** MUST NOT quedar detrás de una interacción adicional

#### Scenario: Declaración estructurada, texto de la superficie
- **WHEN** el daemon devuelve la declaración
- **THEN** SHALL emitir claves estables y no prosa localizada
- **AND** cada superficie SHALL renderizar su texto desde el catálogo ES/EN

#### Scenario: Ningún dato sale de la máquina
- **WHEN** el daemon computa la contabilidad
- **THEN** SHALL leer únicamente registros locales
- **AND** MUST NOT abrir conexiones de red ni escribir métricas fuera del directorio de datos del usuario

### Requirement: Paridad de superficies de la contabilidad
El método de contabilidad SHALL ser consumible por igual desde las tres
superficies: subcomando scriptable en la CLI con salida legible y variante
`--json` de exactamente un objeto, entrada en la paleta de la TUI y entrada en
el registro tipado de la GUI, con su fila en la matriz `docs/paridad-nucleo.md`.
La CI MUST fallar si alguna superficie no lo registra, y la referencia CLI
generada SHALL quedar al día con la gramática.

#### Scenario: CLI de contabilidad con salida de un objeto
- **WHEN** se invoca el subcomando de contabilidad con `--json`
- **THEN** el binario SHALL emitir exactamente un objeto JSON en stdout
- **AND** stderr SHALL permanecer libre de JSON

#### Scenario: Método con casa en las tres superficies
- **WHEN** el contrato incorpora el método de contabilidad
- **THEN** el gate de paridad SHALL encontrarlo en la paleta de la TUI, en el registro de la GUI y en la matriz
- **AND** SHALL fallar señalando la superficie ausente si falta en alguna

### Requirement: Panel de contabilidad con la densidad del design system
El panel de contabilidad de la GUI SHALL presentar las celdas como tablas densas
del design system —filas de 32 px, celdas de 8 px, paneles de 16 px, radios de 4
y 8, hairlines y un único nivel de sombra— con numerales tabulares para que las
cifras no salten, operación completa por teclado con foco visible y textos por
el catálogo ES/EN. El panel MUST NOT animar su layout ni desplazar la bandeja de
permisos o los banners de señal, y MUST NOT introducir dependencias de gráficos:
la representación es tabular.

#### Scenario: Panel denso con numerales tabulares
- **WHEN** el usuario abre el panel de contabilidad
- **THEN** las celdas SHALL renderizarse con la densidad y las formas del design system
- **AND** las cifras SHALL usar numerales tabulares

#### Scenario: El refresco no mueve nada bajo el cursor
- **WHEN** el panel actualiza sus cifras
- **THEN** SHALL actualizarlas sin animar el layout
- **AND** la bandeja de permisos y los banners de señal SHALL permanecer en su sitio

#### Scenario: Panel operable por teclado
- **WHERE** el usuario navega solo con teclado
- **THEN** filtros, selector de período y celdas SHALL ser alcanzables con foco visible
