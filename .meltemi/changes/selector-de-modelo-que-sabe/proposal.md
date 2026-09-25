# selector-de-modelo-que-sabe

> Vía completa: un dato nuevo que el daemon guarda entre sesiones y publica en
> `fleet/list`, y las tres superficies que lo muestran. Nace de la revisión
> adversarial de `modelo-por-opcion-anunciada` (2026-09-25), que lo sacó de su
> alcance para darle un diseño propio.

## Why

`modelo-por-opcion-anunciada` hace que el modelo pedido llegue por la opción que
el agente anuncia, y que se rehúse al abrir cuando el agente no lo anunció. Pero
quien elige el modelo lo hace a ciegas: el selector del compositor de la GUI solo
conoce lo declarado en perfiles y lo visto en sesiones anteriores, y ninguna
superficie dice qué modelos anunció un agente la última vez. El resultado es un
rehúso que cuesta un lanzamiento para descubrir lo que el daemon ya supo en la
sesión anterior.

La revisión que sacó esto de aquella change fijó lo que este diseño tiene que
resolver, y por qué no era un detalle:

- **La clave**: un anuncio pertenece al binario que corrió y a la cuenta con la
  que corrió. Un `agent.command` literal junto a un `agent.id` no es ese agente,
  y dos perfiles del mismo agente, en cuentas distintas, pueden ver listas
  distintas.
- **Las cotas**: lo anunciado es texto del agente —OpenCode anuncia 317
  modelos—, y un agente defectuoso u hostil no debe poder hacer crecer sin
  límite un archivo que `fleet/list` lee en cada consulta.
- **La escritura y la lectura**: dos sesiones que abren a la vez no deben perder
  ni romper el archivo, y uno ilegible nunca debe romper la flota.
- **§9**: es un registro, entre proyectos y cuentas, de a qué proveedores llega
  cada cuenta; su contenido exacto se publica antes de existir, y se dice cómo
  borrarlo.
- **La honestidad del selector**: un recuerdo no es una promesa; «su último
  anuncio no traía modelo» no es «no toma modelo», y la entrada libre nunca se
  bloquea por un recuerdo.

## What Changes

- El daemon recuerda, por agente de la flota **y perfil**, solo cuando corrió el
  binario de la entrada, lo que se anunció al abrir —incluido «nada»—, con su
  fecha: únicamente las opciones de categoría `model` y `thought_level`, sus ids
  y nombres, con cotas de cantidad y de largo; escritura atómica bajo un candado
  del proceso y solo cuando cambió; un archivo ilegible es «nada recordado».
- `fleet/list` devuelve por entrada lo recordado y las palancas que la entrada
  declara (aditivos), y deja de marcar configurada una entrada que un comando
  literal sustituye.
- El selector de modelo del compositor de la GUI —su lógica en un módulo puro con
  pruebas— ofrece lo anunciado la última vez, la bandera, o «se confirma al
  arrancar», sin bloquear nunca la entrada libre; consulta la flota del proyecto
  al que apunta.
- `meltemi fleet`, la vista Flota de la TUI y la de la GUI dicen por agente su
  palanca por bandera y lo último que anunció, con cuántos valores y cuándo, o
  que no se sabe nada todavía.
- La documentación publica el contenido exacto del archivo y cómo borrarlo.

## Capabilities

### Modified Capabilities

- `fleet-catalog`: + la flota recuerda lo anunciado (clave, cotas, escritura,
  lectura tolerante) y + las palancas se ven en toda superficie.
- `gui-shell`: el selector de modelo ofrece lo que se sabe.

## Impact

- `core/meltemid/src/fleet.rs` y un módulo nuevo para el recuerdo, `proto/`
  (campos aditivos de `fleet/list`), `tui/`, `desktop/ui/`, `docs/`.
- Depende de `modelo-por-opcion-anunciada` (la vía anunciada y el registro de
  palancas).

## Fuera de alcance

- Precios, créditos o un catálogo propio de modelos.
- Refrescar lo recordado sin abrir una sesión.
