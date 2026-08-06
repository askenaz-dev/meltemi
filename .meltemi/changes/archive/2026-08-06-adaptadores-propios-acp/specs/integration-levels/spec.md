# integration-levels — delta

## MODIFIED Requirements

### Requirement: Suite de conformidad por nivel
El proyecto SHALL mantener una suite de conformidad ejecutable con
criterios pasa/no-pasa por nivel (streaming, cancelación, permisos, sesión,
salida estructurada, proyección), que en CI MUST correr exclusivamente
contra agentes simulados y sin red; la ejecución contra agentes reales MUST
ser manual y por opt-in explícito. Los criterios del nivel pilotado por
adaptador SHALL ejercerse en CI a través de los adaptadores propios de
Meltemi pilotando procesos proveedor simulados que hablan el cable
documentado de su proveedor; los binarios de proveedores reales MUST
permanecer fuera de CI. El resultado por agente SHALL persistirse con fecha
y versión.

#### Scenario: Conformidad en CI con simulados
- **WHEN** la suite corre en CI
- **THEN** SHALL ejecutar los criterios de cada nivel contra los agentes simulados
- **AND** SHALL NOT contactar red ni agentes reales

#### Scenario: Nivel de adaptador ejercido por los adaptadores propios
- **WHEN** la suite ejerce en CI los criterios del nivel pilotado por adaptador
- **THEN** SHALL pilotar los binarios reales de los adaptadores propios contra procesos proveedor simulados
- **AND** ningún binario de proveedor real SHALL ejecutarse

#### Scenario: Resultado persistido
- **WHEN** una corrida de conformidad concluye para un agente
- **THEN** el resultado por criterio SHALL persistirse con fecha y versión del agente
