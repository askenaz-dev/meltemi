# gui-shell — delta

## MODIFIED Requirements

### Requirement: El modelo y el esfuerzo se eligen y se ven

El lanzador SHALL permitir elegir modelo y esfuerzo antes de arrancar, con
búsqueda y admitiendo entrada libre, y la sesión SHALL mostrar los valores
efectivos. El selector SHALL saber qué ofrece el agente elegido: lo que anunció
la última vez, que toma el modelo por bandera, o que no lo toma; y cuando no se
sabe nada, SHALL decir que el valor se confirma al arrancar. La ficha de un
modelo SHALL mostrar únicamente lo que Meltemi conoce —lo anunciado por el
agente, lo declarado en perfiles y el consumo medido localmente— y NO SHALL
mostrar precios ni créditos.

WHERE se ofrezca cambiar el modelo con la sesión en marcha, SHALL advertirse
que el cambio reinicia la caché del proveedor y puede aumentar el costo.

#### Scenario: Se elige con búsqueda y se admite entrada libre

- **WHEN** se abre el selector de modelo
- **THEN** SHALL poder buscarse
- **AND** SHALL admitirse un valor escrito a mano

#### Scenario: La ficha no inventa lo que no sabe

- **WHEN** se muestra la ficha de un modelo
- **THEN** NO SHALL mostrar precios ni créditos

#### Scenario: Cambiar en marcha se advierte

- **WHERE** la sesión está en marcha
- **WHEN** se ofrece cambiar el modelo
- **THEN** SHALL advertirse el efecto sobre la caché y el costo

#### Scenario: El selector ofrece lo que el agente anunció

- **WHERE** el agente elegido anunció modelos la última vez
- **WHEN** se abre el selector de modelo
- **THEN** SHALL ofrecer esos modelos
- **AND** un valor escrito a mano fuera de ellos SHALL advertirse como a confirmar al arrancar

#### Scenario: Un agente que no toma modelo lo dice

- **WHERE** el último anuncio del agente elegido no traía modelo y su entrada no lo declara
- **WHEN** se abre el selector de modelo
- **THEN** SHALL decir que ese agente no toma modelo
