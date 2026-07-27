# sdd-authoring — delta

## MODIFIED Requirements

### Requirement: Verbo explore sin escritura
El verbo `explore` SHALL conducir deliberación con el agente (leer el repo,
sopesar opciones, proponer rumbo) en streaming, y MUST NOT escribir ni modificar
artefactos ni archivos del proyecto. El verbo SHALL admitir un parámetro opcional
de agente, resuelto por el orden vigente de la flota —perfil de lanzamiento, id
del catálogo y, en su defecto, el agente configurado del proyecto—, con la
resolución efectiva registrada en el log de la sesión; un agente nombrado que
resuelve a un binario no detectado MUST rehusar con diagnóstico y remedio y MUST
NOT degradar en silencio a otro proveedor. La elección de agente MUST NOT relajar
la garantía de no escritura: la deliberación sigue siendo inocua sea cual sea el
agente elegido.

#### Scenario: Exploración inocua
- **WHEN** un turno de explore concluye
- **THEN** el árbol del proyecto SHALL quedar sin modificaciones
- **AND** la deliberación SHALL quedar solo en el log de sesión

#### Scenario: Explore con agente nombrado sigue sin escribir
- **WHEN** un turno de explore corre con un agente nombrado distinto del configurado
- **THEN** el árbol del proyecto SHALL quedar sin modificaciones
- **AND** la resolución con su fuente SHALL constar en el log de la sesión

#### Scenario: Explore con agente no detectado rehúsa sin degradar
- **IF** el agente nombrado resuelve a un binario no detectado
- **THEN** `explore` SHALL rehusarse con diagnóstico y remedio
- **AND** ningún otro proveedor SHALL deliberar en su lugar
