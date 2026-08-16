# salida-que-se-lee

> Vía rápida (fast-forward): los cuatro artefactos de una vez, gate único.
> Deltas **solo ADDED** sobre una capability existente (`cli-contract`),
> ninguna capability nueva, ningún MODIFIED ni REMOVED. **Sin colisión**:
> `lanzador-conversacional` tiene un MODIFIED sin archivar sobre
> `cli-contract`, pero solo sobre «Mapeo comando↔método RPC»; los requisitos
> nuevos de esta change son encabezados propios y distintos.

## Why

El mantenedor puso lado a lado la salida de OpenSpec y la nuestra, y la
comparación no admite discusión. Lo suyo:

```
Summary:
  ● Specifications: 29 specs, 174 requirements
  ● Active Changes: 0 in progress
  ● Completed Changes: 0

Specifications
  ▪ tui-shell         23 requirements
  ▪ spec-engine       14 requirements
```

Lo nuestro:

```
36 capabilit(y/ies) in the living truth
  acp-session  10 req  19 scenario(s)
  artifact-format  7 req  14 scenario(s)
```

Tres carencias concretas, no cuestión de gusto. **No hay resumen**: la primera
línea es un recuento y nada más; para saber cuántos requisitos hay en total,
o cuántas changes esperan una decisión, hay que sumar a ojo. **No hay
alineación**: los nombres y las cifras bailan, así que la vista no se puede
recorrer en vertical — que es como se lee una lista de treinta y seis
elementos. **No hay color**: `active` y `archived`, `verify 7/7` y `verify
0/9`, una compuerta que espera y una que no, se ven todos igual, y el ojo no
tiene dónde agarrarse.

Y falta una pieza que el mantenedor pidió aparte: **formatos de salida al
estilo de Kubernetes**. Hoy existe `--json`, que emite el objeto crudo del
comando; falta `--yaml`, y falta que la elección de formato sea una sola
decisión declarada y no un flag que cada comando interpreta a su manera.

## What Changes

- **Un resumen encabeza cada listado**: totales que hoy hay que sumar a ojo —
  capacidades, requisitos y escenarios en `specs`; changes activas, archivadas
  y cuántas esperan una decisión en `changes`.
- **Las columnas se alinean**, calculadas del contenido y no fijadas a mano, de
  modo que treinta y seis filas se recorren en vertical.
- **El color codifica estado y tipo** —activa contra archivada, un contador
  completo contra uno a medias, una compuerta que espera— **y jamás es el único
  portador**: cada distinción que el color marca la marca también un símbolo o
  una palabra. Es la regla que la superficie de terminal ya tiene escrita, y
  esta change la extiende al CLI scriptable.
- **`NO_COLOR`, `--no-color` y `TERM=dumb` se honran**, igual que en el shell
  interactivo, y **stdout sin TTY sale sin color por defecto**: una salida
  redirigida a un archivo o a un `grep` no lleva secuencias de escape.
- **`--yaml` junto a `--json`**, con la misma promesa: exactamente un
  documento en stdout, sin texto humano mezclado, tanto en éxito como en error.
  La elección de formato pasa a ser una sola decisión —humano, JSON o YAML— y
  no tres caminos independientes.

## Capabilities

### Modified Capabilities

- `cli-contract`: + tres requisitos ADDED — el formato de salida como una
  elección declarada (humano/JSON/YAML), la presentación legible de los
  listados (resumen, alineación, color redundante), y la disciplina de color
  del CLI (`NO_COLOR`, `--no-color`, sin TTY sin color). Ningún requisito
  existente se toca: `--json` sigue prometiendo exactamente lo que prometía.

### New Capabilities

- Ninguna.

## Impact

- `tui/src/output.rs` (el formato como elección y el pintado),
  `tui/src/cli.rs` (los flags `--yaml` y `--no-color`),
  `tui/src/run.rs` (los listados que ganan resumen y alineación),
  `docs/referencia-cli.md` (regenerada), `tui/tests/`.
- **Cero dependencias nuevas.** El emisor de YAML es propio y cabe en unas
  decenas de líneas porque YAML 1.2 es superconjunto de JSON: emitiendo siempre
  las cadenas entre comillas dobles con el escapado de JSON, el resultado es
  válido por construcción y los casos de borde no llegan a existir. La
  alternativa —`serde_yaml`— está archivada por su autor desde 2024 y exigiría
  un `ignore` en `deny.toml` elegido a propósito (design D2). El color tampoco
  añade dependencia: son secuencias ANSI que el proyecto ya emite en el shell.
- **Cero cambios en el daemon y en el contrato `proto/`**: esto es la
  superficie del cliente. No nace deber de paridad §4 — la GUI y la TUI tienen
  su propia presentación, gobernada por sus specs.

## Fuera de alcance

- **Rediseñar la TUI interactiva**: tiene sus requisitos de accesibilidad y su
  render propio; esta change es la salida scriptable.
- **`-o wide`, `--template`, o selectores al estilo `jsonpath`**: si el uso lo
  pide, es su propia change con su propio análisis. Aquí entran los dos
  formatos que el mantenedor nombró.
- **Cambiar la forma del objeto JSON de ningún comando**: `--json` es contrato
  y lo que scripts existentes leen. El YAML es ese mismo objeto en otro
  formato, no otro modelo.
- **Colorear la GUI o el sitio**: superficies distintas, specs distintas.
