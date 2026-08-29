# harness-global-y-por-agente — tasks

> El lector primero, porque todo lo demás depende de leer bien un `RULE.md`
> real. Después el descubrimiento y la resolución, que son puro cálculo y se
> prueban sin tocar disco ajeno. La proyección después, con su guardián
> estructural. Las superficies al final.

## 1. Leer un `RULE.md` real

- [x] 1.1 El escáner de front-matter se generaliza a un mapa `clave → valor`
  con las dos formas que existen (escalar y lista en línea), reutilizado por
  `rumbo/` y por el harness — **sin dependencia nueva** (design D3)
- [x] 1.2 Un escalar con comentario al final se lee sin él (`version: 0.2.0 #
  x-release-please-version`), respetando los `#` que estén entre comillas — el
  defecto que los `RULE.md` de FDH tienen hoy y que `rumbo/` no podía encontrar
- [x] 1.3 Una forma no cubierta —lista de bloque— produce diagnóstico que nombra
  la clave y la forma esperada, en vez de un valor parcial — escenario «Una
  forma no cubierta se diagnostica en vez de leerse a medias»
- [x] 1.4 Las claves que el núcleo no conoce se conservan verbatim y se
  exponen — escenario «Las claves que el núcleo no conoce se conservan»
- [x] 1.5 Fixture con los cuatro `RULE.md` reales de FDH copiados tal cual, para
  que el lector se pruebe contra el formato de verdad y no contra uno inventado
  <!-- 2026-08-19: el fixture encontró un defecto **anterior a esta change** a
  los cinco minutos de existir: `parse_list` partía por cada coma, y el `scope`
  real `["**/*.{ts,tsx}"]` se rompía en `"**/*.{ts` y `tsx}"`. Una regla que
  habría aplicado a un patrón que nadie escribió. Ningún archivo de `rumbo/`
  podía encontrarlo porque ninguno de sus globs ha llevado nunca una coma; hizo
  falta el formato de verdad. Ahora corta solo en las comas fuera de comillas,
  que es lo que YAML significa. -->

## 2. Los cuatro ámbitos y la resolución

- [x] 2.1 Descubrimiento de las cuatro capas con sus rutas (design D1), y el
  identificador de agente validado contra el catálogo de flota — escenario «Un
  agente que el catálogo no conoce no recibe proyección»
- [x] 2.2 Resolución por nombre: la capa más específica gana **entera**, sin
  merge de campos; lo pisado se conserva con quién lo pisó — escenario «Lo
  específico pisa lo general»
- [x] 2.3 El nombre del front-matter debe coincidir con el del directorio;
  si no, inválida con diagnóstico y sin proyectar — escenario «Un nombre que no
  coincide con su directorio se rehúsa»
- [x] 2.4 `agents_supported` se conserva y **no gobierna nada** (design D4): los
  vocabularios de ids no coinciden y adivinar la equivalencia aplicaría reglas
  al agente equivocado en silencio
  <!-- 2026-08-19: la precedencia **es el orden de las variantes del enum**, no
  una tabla aparte: derivar `Ord` deja que la capa se compare sola y quita el
  sitio donde alguien podría ordenarlas al revés sin que nada lo note. Y los ids
  conocidos se leen del catálogo de flota en vez de copiarse aquí: dos listas de
  identificadores de agente es exactamente cómo el eje por agente empezaría a
  aplicar reglas a un agente que este build ya no trae. -->

## 3. El contrato y la vista efectiva

- [x] 3.1 `harness/effective` en `proto/` con su schema, la conformidad de tres
  vías y `gen:forms` commiteado; proyecto y agente opcionales
- [x] 3.2 La respuesta lleva, por pieza, capa de origen y ruta — escenario «Cada
  pieza dice de qué capa viene»
- [x] 3.3 Lo pisado y lo inválido viajan con su motivo — escenario «Lo pisado y
  lo inválido también se ven»
- [x] 3.4 Sin proyecto declarado se responde solo lo global — escenario «Sin
  proyecto solo se responde lo global»
  <!-- 2026-08-19: el valor de front-matter viaja **sin discriminador**: en el
  cable una cadena y un arreglo ya son inconfundibles, y un `type` sería
  ceremonia que ningún lector necesita. La conversión de capa a nombre de cable
  está en un `match` exhaustivo a propósito: una quinta capa no compila hasta
  que también tenga nombre en el contrato. -->

## 4. La proyección, en dos contenidos

- [x] 4.1 Las reglas de ámbito proyecto entran al bloque gestionado con su
  ámbito en prosa — escenario «Una regla del proyecto llega a los destinos del
  repositorio»
- [x] 4.2 **El guardián es estructural** (design D6): la compilación de lo que
  va al repositorio no recibe las fuentes globales, y el test proyecta con
  harness global presente exigiendo el repositorio intacto — escenario
  «Proyectar con harness global deja el repositorio intacto»
  <!-- 2026-08-19: el guardián es **la llamada**, no un filtro después: la
  compilación del repositorio invoca `discover(None, Some(root), …)`, así que el
  directorio de configuración del usuario no tiene por dónde entrar. El
  `debug_assert` que lo acompaña no es el guardián sino su alarma, y el test
  comprueba además que la regla del proyecto **sí** viajó — sin eso, pasaría
  igual proyectando nada. -->
- [x] 4.3 El mapa de destinos gana la columna de ámbito usuario, y **solo
  entradas verificadas con su fecha** (design D7); un agente sin ruta verificada
  se declara — escenario «Un agente sin destino verificado no recibe escritura»
  <!-- 2026-08-19: tres destinos verificados contra la documentación del
  proveedor, cada uno con su cita y su fecha: `~/.claude/CLAUDE.md`,
  `~/.codex/AGENTS.md` y `~/.config/opencode/AGENTS.md`. Y verificar produjo dos
  hechos que ninguna suposición habría dado: Codex lee **`AGENTS.override.md` en
  vez de** `AGENTS.md` cuando el override existe —escribir igual llenaría un
  archivo que nadie lee y parecería haber funcionado—, y su directorio se mueve
  con `CODEX_HOME`. Los dos están en los datos, no en el código. -->
- [x] 4.4 La primera escritura en un archivo de usuario de un agente pide
  consentimiento y queda registrada — escenario «La primera escritura en config
  ajena se consiente»
  <!-- 2026-08-19: el consentimiento es **por agente** y se guarda en la
  configuración del usuario; se registra **antes** de proyectar, para que un
  fallo a mitad deje consentimiento otorgado y no un archivo escrito bajo un
  consentimiento que nadie guardó. Solo se añade: retirarlo es editar ese
  archivo, y el bloque gestionado deja de actualizarse sin borrar nada de lo
  ajeno. La superficie scriptable no consiente nada implícitamente. -->

## 5. Las superficies

- [x] 5.1 CLI: verbo `harness [--agent <id>]` con `--json` y la disciplina de
  salida vigente — escenario «El verbo muestra de qué capa viene cada pieza»
- [ ] 5.2 GUI: drill-in desde la Flota, solo lectura, con capa de origen y con
  lo que no aplica — escenario «La ficha de un agente muestra su harness con el
  origen»
- [ ] 5.3 TUI: la misma lectura con la navegación del shell — escenario «El
  terminal muestra el harness con su origen»
- [x] 5.4 i18n es/en de todo lo nuevo, con el lint como guardián
- [x] 5.5 Paridad: fila en `docs/paridad-nucleo.md` y entrada en los dos
  registros de superficie

## 6. Cierre

- [ ] 6.1 `docs/harness.md`: los cuatro ámbitos, el formato, la precedencia y la
  frontera con FDH — incluido **dónde escribe cada uno** (design D6)
- [ ] 6.2 `rumbo/structure.md` gana los directorios del harness
- [ ] 6.3 `validate` limpio, `verify` con los escenarios enlazados, suite
  completa, clippy, fmt, gates del frontend y paridad revisada
