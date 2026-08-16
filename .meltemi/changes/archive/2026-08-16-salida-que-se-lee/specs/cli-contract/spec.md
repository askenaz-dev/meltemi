# cli-contract — delta

## ADDED Requirements

### Requirement: El formato de salida es una elección declarada

El binario SHALL tratar el formato de salida como una elección única entre
humano, JSON y YAML, y NO SHALL admitir dos formatos legibles por máquina a la
vez. El flag global `--yaml` SHALL emitir exactamente un documento YAML en
stdout —tanto en éxito como en error— con el mismo contenido que `--json`
emitiría, y MUST NOT mezclar texto humano en stdout. Bajo cualquier formato
legible por máquina la salida NO SHALL llevar color.

#### Scenario: YAML emite un documento y nada más

- **WHEN** se invoca un subcomando scriptable con `--yaml`
- **THEN** stdout SHALL contener exactamente un documento YAML
- **AND** NO SHALL contener texto humano ni secuencias de color

#### Scenario: El error en YAML también es un documento

- **WHEN** un subcomando invocado con `--yaml` falla
- **THEN** stdout SHALL contener exactamente un documento YAML de error con el
  código de la taxonomía de salida
- **AND** stderr SHALL permanecer libre de ese documento

#### Scenario: Dos formatos de máquina a la vez se rehúsan

- **WHEN** se invocan `--json` y `--yaml` en la misma orden
- **THEN** el binario SHALL rehusar con un error de uso
- **AND** NO SHALL elegir uno por su cuenta

### Requirement: Los listados se leen de un vistazo

Todo listado del binario SHALL encabezarse con un resumen de los totales que de
otro modo habría que sumar a mano, y SHALL alinear sus columnas con anchos
derivados del contenido, nunca fijos. El resumen de los cambios SHALL declarar
cuántos esperan una decisión.

#### Scenario: El listado abre con su resumen

- **WHEN** se listan las capacidades de la verdad viva o los cambios
- **THEN** la salida SHALL abrir con los totales del listado
- **AND** el resumen de los cambios SHALL decir cuántos esperan una decisión

#### Scenario: Las columnas se alinean con el contenido

- **WHEN** un listado incluye un elemento de nombre más largo que los demás
- **THEN** las columnas SHALL seguir alineadas
- **AND** el ancho NO SHALL provenir de un número fijo en el código

### Requirement: Color redundante y apagable en la salida del cliente

La salida humana del binario MAY usar color para señalar estado y tipo, y el
color MUST ser solo decorativo: toda distinción que el color marque SHALL
marcarse además con símbolo, palabra o cifra, de modo que retirar todo el color
NO SHALL retirar información alguna. El binario SHALL renderizar sin color
alguno cuando se invoque `--no-color`, cuando `NO_COLOR` esté definida con valor
no vacío, cuando `TERM` sea `dumb`, o cuando stdout no esté conectado a un TTY.

#### Scenario: Sin color no se pierde información

- **WHEN** se compara la salida humana coloreada con la misma salida sin color
- **THEN** ambas SHALL contener el mismo texto
- **AND** cada estado y cada tipo SHALL seguir distinguiéndose por símbolo,
  palabra o cifra

#### Scenario: La salida redirigida no lleva color

- **WHEN** stdout no está conectado a un TTY
- **THEN** la salida NO SHALL contener secuencia de color alguna

#### Scenario: El usuario apaga el color

- **WHERE** se invoca `--no-color`, o `NO_COLOR` tiene valor no vacío, o `TERM`
  es `dumb`
- **THEN** la salida NO SHALL contener secuencia de color alguna
