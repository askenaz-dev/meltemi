<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `identidad-propia`

**Fecha**: 2026-10-01 · **Plataforma**: Windows 11 Pro, WebView2
**Binarios**: `target/release/{meltemi-desktop,meltemid,meltemi,mock-mesh}.exe`,
el de escritorio construido con `tauri build --no-bundle`.
Daemon, datos y configuración **aislados** (endpoint, `MELTEMI_DATA_DIR`,
`MELTEMI_CONFIG_DIR` propios), sobre un repositorio fixture temporal. Nunca
contra este repositorio, nunca contra la red, y **sin ningún cliente de malla
real**: el configurado es `mock-mesh`, el simulado del workspace.

## Método

Sin puerto de depuración: lo que había que ver es cromo y texto, así que el
conductor abre la app, navega con clics sintéticos y fotografía la ventana con
`PrintWindow` (DPI por monitor fijada antes de cualquier llamada de ventana, o
`GetWindowRect` informa coordenadas lógicas mientras `PrintWindow` dibuja
físicas y la captura sale recortada). El CLI se interrogó primero contra el
mismo daemon, porque si el daemon no contesta bien no hay pantalla que valga.

**Gotcha nuevo**: la paleta de la GUI, con «settings» escrito, ofrece primero
`subscription/link` —coincidencia difusa sobre nombres de método— y no la vista
Ajustes. No es de esta change y no se toca aquí; queda anotado porque costó dos
intentos y porque un usuario escribiendo «settings» espera Ajustes.

## Qué se midió, por el CLI

```
$ meltemi identity
guillmar@mock.example on https://headscale.example
this machine: mi-pc

  mi-pc        online   windows  (this machine)
  el-portatil  online   macOS
  la-torre     offline  linux  last seen 2026-09-30T18:21:05Z
```

`--json` devuelve la respuesta del contrato tal cual, con `machines[]` en el
orden de la respuesta y sin un solo campo fuera del modelo: ni direcciones, ni
claves, aunque la salida del cliente trae ambas.

`identity-link` imprimió el gesto **con la ruta citada** para PowerShell, que es
la plataforma de la corrida, y con el nombre real de este equipo:

```
'C:\Users\guill\AppData\Local\Temp\meltemi-identity-smoke\mock-mesh.exe' up --login-server https://headscale.example --hostname ASKENAZ-PC
```

Las comillas no son decorativas: la ruta tiene espacios, y es exactamente el
caso que las dos formas citadas existen para sobrevivir.

## Veredicto por escenario

| Escenario | Medido |
| --- | --- |
| Ajustes muestra quién soy y mis equipos | sección «Identity» con `guillmar@mock.example`, Mesh `https://headscale.example`, This machine `mi-pc`, y las tres máquinas |
| …con la presencia en símbolo **y** palabra | `● mi-pc online`, `● el-portatil online`, `○ la-torre offline` — relleno/hueco **y** la palabra, nunca una sola |
| …con el gesto copiable | el gesto en bloque monoespaciado + botones «Copy the gesture» y «Re-read the mesh»; ningún control abre un navegador |
| La promesa de privacidad sigue siendo cierta | «No accounts: Meltemi has no login and no server.» intacta, **inmediatamente debajo** de la sección de identidad, y la frase «The identity is your mesh's: Meltemi reads it, it does not keep it» encabezándola |
| Sin enlazar, la barra ofrece enlazar y no exige nada | la fila del pie lleva a Ajustes al pulsarla (medido: un clic en la fila abrió Ajustes) |
| Sin configurar, el cromo no pide nada | con `[identity]` ausente, el pie muestra **solo** Ajustes: ninguna fila, ninguna invitación. El CLI dice `no identity (not_configured)` con su remedio |
| Plegada, la identidad sigue alcanzable | plegada a riel, la marca propia (tres nodos enlazados) queda sobre el engranaje de Ajustes, ambas como botones de solo icono |
| Ajustes sigue siendo la última entrada | en las tres capturas —desplegada, plegada y con identidad apagada— Ajustes es la fila de más abajo |
| La identidad nunca es requisito | el diálogo de bienvenida de primera corrida no menciona identidad ni la pide |

Capturas en `%TEMP%\meltemi-identity-smoke\` (no se publican: llevan el nombre
real de este equipo).

## Lo que este smoke **no** midió

- **Un cliente de malla real**: no hay ninguno instalado en este equipo. Toda la
  corrida es contra `mock-mesh`, cuya salida reproduce la forma verificada contra
  las definiciones de tipo publicadas del cliente en `v1.102.5`. La verificación
  de campo —`meltemi identity` una vez contra un cliente instalado y enlazado—
  sigue pendiente y está anotada en `docs/acceso-remoto.md`.
- **`identity-link --exec` contra un cliente de verdad**: lo que se ejecutó en la
  suite es el simulado, que rehúsa cualquier argv que no sea `status --json`; eso
  prueba que la entrada y la salida se heredan (su queja llega al llamador sin
  filtro) pero no un `up` que termine en un navegador.
- **macOS y Linux**: corrida de Windows. Los gates de las tres plataformas son
  los de CI.
