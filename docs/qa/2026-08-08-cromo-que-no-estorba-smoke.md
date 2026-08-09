# Smoke conducido — `cromo-que-no-estorba`

**Fecha**: 2026-08-08 · **Plataforma**: Windows 11, WebView2
**Binario**: `target/release/meltemi-desktop.exe`, sobre un repositorio fixture
temporal con `mock-agent` y directorios de datos y configuración aislados.
Puerto de depuración remoto **temporal**, revertido y el binario reconstruido
limpio al terminar.

## Resultado

| Escenario | Medido |
| --- | --- |
| El cajón parte la ruta larga en vez de desplazarla | `overflow-x: hidden`, `overflow-y: auto`, `scrollWidth == clientWidth` en un panel de 267 px |
| Hacer clic fuera cierra la paleta | clic en (40, 736), sobre el velo y lejos del panel: el velo desaparece |
| La confirmación se retira sola | «vínculo … creado» presente tras la acción, ausente 7,5 s después, sin gesto |
| El error se queda hasta que alguien lo retira | dos rechazos de `subscription/link` siguen en pantalla tras 7,5 s |

Las dos mitades del par de avisos se observaron en ejecuciones separadas: una
con una confirmación en pantalla, otra con dos rechazos. El comportamiento
combinado —una confirmación que se va mientras un error se queda— está cubierto
por `desktop/ui/tests/notices.test.ts`, que lo ejecuta con reloj simulado y
avanza cien veces el plazo para probar que **ningún** temporizador alcanza a un
aviso que no sea informativo.

## Lo que se ve

Los avisos son tarjetas con filo de color y línea de contorno, dentro de una
región con margen, en vez de bandas teñidas de borde a borde. Siguen en flujo y
a su altura natural, como la regla de disposición del shell exige: son barras,
no una capa superpuesta.

## Observación fuera de alcance, anotada

La tabla de Flota **sí** lista los perfiles enlazados como filas propias
(`prueba-de-aviso`, `work`, `thorough` junto a los agentes del registro), así
que varias suscripciones por agente ya existen en el núcleo y en la superficie.
Lo que la fila no dice es **de qué agente** es cada suscripción: el contrato
lleva `underlyingAgent` y la tabla no lo muestra, de modo que «varios Claude y
varios Codex» es imposible de leer aunque esté configurado. Es el objeto de su
propia change.
