# context-projection — delta

## ADDED Requirements

### Requirement: Las reglas del proyecto viajan en el bloque gestionado

Las reglas de ámbito proyecto SHALL proyectarse dentro del bloque gestionado
existente de los destinos declarados, con su ámbito de aplicación expresado en
prosa junto a cada una. Todo lo de fuera del bloque SHALL seguir preservándose
byte a byte.

#### Scenario: Una regla del proyecto llega a los destinos del repositorio

- **WHEN** el proyecto declara una regla y se proyecta el contexto
- **THEN** SHALL aparecer dentro del bloque gestionado de cada destino
- **AND** SHALL decir a qué archivos aplica

### Requirement: Lo global del usuario jamás entra al repositorio

El contenido de los ámbitos globales del usuario SHALL proyectarse únicamente a
destinos de ámbito usuario, y NO SHALL entrar en archivo alguno del
repositorio.

La separación SHALL ser estructural —la compilación de lo que va al repositorio
no recibe las fuentes globales— y NO SHALL depender de una comprobación que una
refactorización pueda omitir.

#### Scenario: Proyectar con harness global deja el repositorio intacto

- **WHERE** el usuario tiene harness global declarado
- **WHEN** se proyecta el contexto de un proyecto
- **THEN** los archivos del repositorio NO SHALL contener ese contenido
- **AND** el estado del repositorio SHALL quedar sin cambios por ese contenido

### Requirement: Destinos de ámbito usuario solo donde estén verificados

El mapa de destinos SHALL admitir rutas de ámbito usuario por agente, cada una
con la fecha en que se verificó contra la documentación de su proveedor. Un
agente sin ruta verificada SHALL declararse como tal en la vista y NO SHALL
recibir escritura alguna.

La primera escritura sobre un archivo de ámbito usuario de un agente SHALL
requerir consentimiento explícito y quedar registrada. Meltemi SHALL escribir
únicamente dentro de su propio bloque gestionado y NO SHALL sobrescribir
contenido ajeno.

#### Scenario: Un agente sin destino verificado no recibe escritura

- **WHERE** un agente no tiene ruta de ámbito usuario verificada
- **THEN** SHALL declararse la ausencia
- **AND** NO SHALL escribirse archivo alguno para ese agente

#### Scenario: La primera escritura en config ajena se consiente

- **WHEN** Meltemi va a escribir por primera vez en el archivo de usuario de un
  agente
- **THEN** SHALL pedirse consentimiento explícito
- **AND** la decisión SHALL quedar registrada
