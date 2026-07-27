# fleet-catalog — delta

## ADDED Requirements

### Requirement: Vigencia de las rutas de instalación de la instantánea
Cada comando de instalación que la instantánea del registro declare SHALL
nombrar la distribución canónica vigente del proyecto upstream, verificada
contra su fuente de distribución en la fecha de la revisión — nunca citada de
memoria. WHEN una distribución declarada queda archivada, deprecada o
renombrada por su upstream, la siguiente revisión de la instantánea MUST
reemplazarla por su sucesora, actualizando el campo `version`, y MUST NOT
seguir remitiendo a la ruta muerta; la guía de agentes SHALL actualizarse en
el mismo cambio, forzada por la verificación de coherencia registro↔guía
vigente.

#### Scenario: Comando de instalación verificado contra la distribución vigente
- **WHEN** se revisa la instantánea del registro
- **THEN** cada comando de instalación declarado SHALL corresponder a una distribución publicada y vigente
- **AND** la verificación (fuente y fecha) SHALL quedar documentada en la change que revisa la instantánea

#### Scenario: Distribución archivada reemplazada por su sucesora
- **IF** una distribución declarada fue archivada, deprecada o renombrada por su upstream
- **THEN** la instantánea SHALL apuntar a la distribución sucesora con el campo `version` actualizado
- **AND** la guía de agentes SHALL reflejar el mismo comando en el mismo cambio
