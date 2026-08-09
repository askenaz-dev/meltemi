# Smoke conducido — `flota-por-suscripcion`

**Fecha**: 2026-08-09 · **Plataforma**: Windows 11, WebView2
**Binario**: `target/release/meltemi-desktop.exe`, sobre un repositorio fixture
temporal con `mock-agent` y directorios de datos y configuración aislados; el
veredicto se calcula **dentro de la página** y no en la terminal. Puerto de
depuración remoto temporal, revertido al terminar.

## Fixture

Dos agentes del catálogo (`provider-a`, `provider-b`) con una suscripción
enlazada cada uno (`work`, `thorough`), más cinco suscripciones cuyo agente
declarado **no** está en ese catálogo (`claude-code` y `codex-cli`): el caso
huérfano, que aquí aparece sin fabricarlo.

## Resultado

| Comprobación | Medido |
| --- | --- |
| Cada suscripción declara su agente como texto | 7 de 7 filas hijas |
| Varios agentes con sus suscripciones agrupadas | `{"Provider A": 1, "Provider B": 1}` |
| El agente declara cuántas tiene | «Provider A · 1 suscripción», «Provider B · 1 suscripción» |
| La suscripción sin agente conocido no desaparece | 5 filas, cada una con «— ese agente no está en el catálogo» y el id que declara |
| El nivel se dice con palabras | 9 de 9 filas con «declarado» o «verificado» |
| El singular no dice «1 suscripciones» | 0 casos |

## Lo que el smoke encontró y esta change corrigió

1. **«1 suscripciones».** El recuento usaba una sola cadena para cualquier
   número. El singular es ahora su propia cadena en los dos idiomas.
2. **La sangría no se aplicaba.** `.childAgent` estaba declarada **antes** de
   `.agent`, y `padding: 0` de esa última ganaba por especificidad igual: una
   regla que se veía correcta y no hacía nada. Ahora va calificada
   (`.agent.childAgent`) y después, y el test comprueba el orden en el
   archivo, no solo la existencia de la regla.

Ninguno de los dos era visible desde el código fuente.

## Nota de alcance

El fixture usa un registro simulado, de modo que las suscripciones enlazadas
contra `claude-code` y `codex-cli` aparecen como huérfanas. Eso **es** el caso
que la spec pide comprobar y se aprovecha como tal; el caso contrario —una
suscripción cuyo agente sí está en el catálogo— queda cubierto por `work` y
`thorough`. Ambos aparecen en la misma captura.
