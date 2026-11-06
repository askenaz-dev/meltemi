# Documentación para usuarios

## ADDED Requirements

### Requirement: Guías públicas completas en ambos idiomas
WHEN se construye la documentación del IDE, THE documentación SHALL ofrecer un índice de uso y
las trece guías públicas en inglés y español, conservando los nombres de ruta existentes.

#### Scenario: Una persona cambia de idioma
- **WHEN** abre una guía pública y elige el otro idioma
- **THEN** lee una traducción de la guía, sin aviso de contenido ausente

### Requirement: Instrucciones sustentadas en el producto
WHEN una guía describe una operación, THE documentación SHALL usar los comandos, campos y
controles existentes y declarar sus límites comprobados sin presentar propuestas como entregadas.

#### Scenario: La fuente limita una capacidad
- **WHEN** el agente no ofrece un control, el checkpoint no puede revertir el árbol del usuario o
  un instalador carece de firma de plataforma
- **THEN** la guía explica ese límite y el siguiente paso disponible sin prometer otra capacidad

### Requirement: Referencia de CLI sin deriva
WHEN se verifica la referencia canónica, THE documentación SHALL coincidir con el generador del
CLI; su explicación española SHALL conservar la salida real y traducir la explicación.

#### Scenario: Un comando cambia en la gramática
- **WHEN** la referencia confirmada deja de coincidir con la gramática
- **THEN** la comprobación de documentación falla y exige regenerarla
