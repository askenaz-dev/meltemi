<!-- SPDX-License-Identifier: Apache-2.0 -->

# Accesibilidad

[English](../accesibilidad.md)

La interfaz de terminal de Meltemi está diseñada para poder usarse sin color, sin caracteres
Unicode para dibujar cuadros e incluso sin TTY. La CLI ofrece salida en texto y estructurada;
elige la que mejor funcione con tu terminal o tus herramientas de asistencia.

## La opción `--json`

Todos los subcomandos que usan RPC aceptan `--json` y producen una salida legible por máquinas
que no depende del estilo de la terminal. Úsala con lectores de pantalla, scripts y CI:

```
meltemi --json status
meltemi --json sessions
```

Un subcomando con `--json` siempre funciona mediante scripts y termina tras ejecutarse; nunca
recurre a la interfaz interactiva.

## Color

El cliente respeta [`NO_COLOR`](https://no-color.org): cuando está definida, no emite color ANSI.
El color sólo es decorativo: el significado se expresa mediante texto, nunca únicamente con
color.

## Alternativa ASCII

Define `MELTEMI_ASCII=1` si la terminal o la fuente no puede dibujar símbolos Unicode. La interfaz
interactiva usa sus equivalentes ASCII. Define por separado `MELTEMI_BELL=1` si quieres que la
terminal emita una señal ante eventos que requieren atención; que suene depende de tu terminal.

## Movimiento

El movimiento indica que hay trabajo en curso o una carga, y nada más. La terminal tiene una
sola animación: el indicador giratorio de la cabecera, que sólo se dibuja mientras una sesión
ejecuta un turno y tiene su equivalente ASCII (`-\|/`) cuando se usa esa alternativa. En reposo
no hay animaciones y el pulso no añade redibujados. El tiempo que lleva trabajando una sesión se
muestra en su detalle; una sesión que sigue viva sin trabajar no muestra tiempo, porque nada lo
mantendría actualizado.

El escritorio respeta `prefers-reduced-motion`: cada animación se retira en lugar de congelarse.
Las barras del esqueleto se quedan quietas y aparece la palabra de carga, la marca se muestra ya
dibujada y el anillo ambiental se sustituye por un marco de acento. Los relojes siguen contando:
una cifra que cambia no es movimiento. El tiempo junto a un turno en curso se oculta de la región
viva de la conversación para que el lector de pantalla anuncie una vez que está trabajando, en
vez de hacerlo cada segundo. Estos ajustes no cambian las decisiones de permisos ni los tiempos
de espera de las sesiones.

## Sin bloqueos cuando no hay TTY

Ejecutar el cliente sin argumentos ni TTY produce un error de uso que remite a `meltemi help`:
el cliente nunca espera una entrada interactiva cuando no hay terminal. Con un subcomando siempre
se ejecuta mediante scripts y termina tras la operación.

Lo mismo se aplica a los permisos que solicita un agente. Sólo se preguntan cuando stdin y
stderr son una terminal; en otro caso, cada solicitud se deniega inmediatamente con el rechazo
del propio agente y un aviso en stderr, sin esperar una tecla. Cuando se preguntan, la solicitud
es texto simple en stderr, sin color, cuadros ni pantalla completa, y cada respuesta es una
sola tecla: un dígito para elegir una opción del agente, `d` o Esc para denegar y Ctrl+C para
detener. Enter nunca decide. Las teclas pulsadas antes de que se pueda leer una pregunta se
descartan en vez de tomarlas como respuesta; una vez que puede leerse, no hay límite de tiempo
para la persona: el único plazo es `[permissions] wait` del daemon. stdout no se modifica, por
lo que `--json` sigue siendo un único objeto.

## Terminales remotas o limitadas

Para el uso remoto, crea un túnel SSH del socket local; consulta las
[notas de plataformas](plataformas.md). La opción `--json` sigue funcionando con cualquier
transporte y en las terminales más limitadas.
