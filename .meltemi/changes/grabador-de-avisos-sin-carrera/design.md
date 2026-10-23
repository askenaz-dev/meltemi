# Design — grabador-de-avisos-sin-carrera

## Context

Los tests de `notices.rs` observan el aviso de la única forma honesta: el
comando configurado es un script que escribe lo que recibió, y el test lee lo
que llegó. `write_appender` añade una línea por aviso para que los avisos se
puedan contar. En Windows el script es un `.cmd`, y `cmd` abre el destino de
`>>` sin compartirlo con otros procesos.

Mientras cada aviso llegaba solo, eso no importaba. El test del aviso de
prueba pide dos seguidos, y `send` no espera al comando: los dos `cmd.exe`
corren a la vez y, cuando se pisan, el segundo no puede abrir el archivo.

## Goals / Non-Goals

**Goals**: que el grabador de Windows no pierda una línea cuando dos avisos
llegan juntos, y que un test lo vigile.

**Non-Goals**: cambiar `send`, el emisor o cualquier comportamiento del aviso;
arreglar los otros tests frágiles del proyecto.

## Decisions

### D1 — Un candado con `mkdir` dentro del script

El script hace, en este orden: intenta `mkdir` de `append.lock` junto al
archivo de lo visto y, si falla, vuelve a intentarlo; añade su línea con `>>`;
borra el directorio con `rmdir`. Crear un directorio es atómico: de dos
procesos que lo intentan a la vez, uno lo crea y el otro falla. Así cada
proceso añade su línea solo cuando el archivo es suyo.

El formato de lo visto no cambia, así que `arrived` y `wait_for` siguen igual,
y la versión de Unix no se toca.

*Alternativas.*
- Un archivo por aviso. El nombre único tendría que salir de `cmd`, y
  `%RANDOM%` se siembra con la hora: dos `cmd` nacidos en el mismo segundo
  sacan el mismo número.
- Reintentar el `>>` cuando falla. Pide una pausa entre intentos, y `cmd` no
  tiene una que funcione sin consola: `timeout` exige una, y `ping` a
  localhost es un truco.
- Un script de PowerShell. Tarda en arrancar, y `Add-Content` choca igual con
  un archivo abierto sin compartir.
- Esperar la primera línea antes de pedir el segundo aviso, dentro del test.
  Arregla este test, pero deja el fixture listo para la próxima carrera.

### D2 — El fixture tiene su propio test

`two_notices_launched_together_are_both_recorded` lanza el grabador dos veces
a la vez, directamente y sin pasar por `send`, espera a los dos y cuenta dos
líneas, veinte rondas seguidas. Corre en las dos plataformas. En Windows es el
test que habría visto la carrera antes que el CI: con el script de antes, una
ronda de cada cuatro pierde una línea, y veinte rondas limpias son menos de
una vez en cien.

## Risks / Trade-offs

- **El candado espera girando**: un `goto` sin pausa. La sección crítica es
  una línea, así que el giro dura milisegundos. Si un proceso muriera dentro,
  los demás girarían hasta el `BUDGET` de diez segundos de `send` y el test
  fallaría en rojo, no en silencio.
- **Un candado huérfano** no sobrevive a una corrida: `temp_dir` borra el
  directorio de cada test al empezar.
