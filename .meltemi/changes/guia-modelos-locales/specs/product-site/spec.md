# product-site — delta

## ADDED Requirements

### Requirement: Enlace honesto a los modelos propios
La página de agentes del sitio, en sus dos idiomas, SHALL enlazar la guía de
modelos locales con una frase que diga cómo se trae hoy un modelo propio —por un
agente de la flota que habla con proveedores locales por su propia
configuración— y que el motor propio de Meltemi no existe todavía. La frase MUST
NOT nombrar productos de terceros y MUST NOT anunciar como disponible una
capacidad no publicada.

#### Scenario: La página de agentes enlaza la guía de modelos propios
- **WHEN** un visitante busca cómo usar su propio modelo en la página de agentes
- **THEN** SHALL encontrar, en inglés y en español, la frase honesta y el enlace a la guía
- **AND** la frase SHALL NOT nombrar productos de terceros
