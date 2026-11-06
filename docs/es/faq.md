<!-- SPDX-License-Identifier: Apache-2.0 -->

# Preguntas y resolución de problemas

[English](../faq.md)

Las primeras dudas y los problemas que más cuesta identificar. Cada respuesta enlaza la guía
con los detalles. Si tu pregunta no aparece aquí, abre una incidencia con la plantilla de
preguntas.

## ¿Por dónde empezar si algo falla?

Por el diagnóstico:

```
meltemi doctor
```

Indica qué está instalado, qué responde y qué falta: el cliente y el daemon y si sus versiones
coinciden, el endpoint, tus carpetas y configuración, los agentes que están preparados o a medio
instalar con lo que debes hacer, `git` y `ssh`, y los últimos errores del registro del daemon. En
cada caso explica qué hacer. Nunca inicia un daemon ni ejecuta un agente y termina con código
`14` cuando algo falla. La aplicación de escritorio ofrece el mismo informe en **Ajustes →
Información → Diagnóstico**, y la interfaz de terminal lo tiene en `doctor` dentro de la paleta.
No se envía a ningún sitio: pégalo en una incidencia sólo si lo decides, después de leerlo.

Si el cliente y el daemon muestran versiones diferentes, sigue ejecutándose un daemon anterior:
usa `meltemi stop` y el siguiente comando iniciará el que está instalado.

## La flota dice que mi agente no se puede lanzar

En Windows, un agente instalado mediante npm deja varios scripts de arranque juntos: un `.cmd`
que el sistema puede ejecutar y un `.ps1` que no puede ejecutar directamente. Cuando sólo está el
`.ps1`, la flota muestra `not_launchable` en lugar de pasar al lanzador una ruta que fallará.
Reinstala el agente de forma global para que exista su lanzador `.cmd`. Consulta la
[guía de agentes](agentes.md#windows).

## La flota dice que falta una capa

Algunos agentes tienen dos capas: la CLI oficial y un adaptador que habla el protocolo ACP
(Agent Client Protocol). La fila indica qué capa falta y el comando exacto que la instala. Los
adaptadores propios de Meltemi se distribuyen en sus instaladores, junto al daemon; si falta uno,
repara la instalación de Meltemi. Consulta la [guía de agentes](agentes.md).

## La línea de comandos deniega todos los permisos

Un comando que se ejecuta sin terminal, desde un script, una tubería o CI, no tiene a quién
preguntar: cada solicitud se deniega con el rechazo del propio agente y se indica en stderr. En
una terminal se te pregunta. Para conceder permisos sin intervención humana, escribe una regla
en `.meltemi/permissions.toml` del proyecto o en tu archivo global `permissions.toml`; pulsar `r`
ante una solicitud escribe la regla más específica por ti. Consulta
[sesiones libres](sesion-libre.md).

## Se me pregunta repetidamente por el mismo comando

Pulsa `r` cuando se te pregunte: Meltemi escribe una regla para ese comando o ruta exactos y deja
de preguntar a partir de la siguiente sesión. Una regla que permite un comando no permite otro
comando encadenado, conectado por una tubería, redirigido o sustituido después de él (`;`, `&&`,
`|`, `>`, `$(…)`); por eso esas variantes siguen preguntando. Una regla de denegación las abarca a
todas.

## El agente editó archivos sin preguntarme

Meltemi sólo decide sobre lo que un agente solicita mediante su canal de permisos. Las
herramientas que el agente ejecuta sin enviar una solicitud no pasan por esa decisión. La
[guía de agentes](agentes.md) explica los límites de cada integración. Revisa el registro de la
sesión y la configuración del propio agente antes de continuar.

## La sesión terminó sin decir nada

Un turno en el que el agente no escribe ni usa herramientas se informa como turno vacío, y la
línea de comandos termina con código `15` en vez de considerarlo un éxito. Suele significar que
la CLI del agente no pudo funcionar: sesión cerrada, cuota agotada o un modelo que no ofrece.
Ejecuta manualmente una vez la CLI del propio agente en el mismo proyecto. Los códigos de salida
aparecen en la [referencia CLI](referencia-cli.md#códigos-de-salida).

## ¿Cómo uso Meltemi desde otro equipo?

El daemon nunca abre un puerto de red. El acceso remoto utiliza tu propio SSH: un túnel hacia
el socket local del daemon o `meltemi bridge` ejecutado mediante `ssh`. Esta segunda opción
también llega a un daemon de Windows, cuyo named pipe no se puede reenviar con SSH. La
[guía de acceso remoto](../acceso-remoto.md) incluye ambas opciones y los comandos para aprobar y
seguir una sesión desde un teléfono.

## El instalador avisa de que no tiene firma

El MSI y el DMG todavía no tienen firma de plataforma. Comprueba primero la descarga con
`sha256sum --check --ignore-missing SHA256SUMS` y
`gh attestation verify <file> --repo askenaz-dev/meltemi`; consulta
[releases y verificación](release.md). Después:

- **Windows**: SmartScreen indica que protegió tu equipo. Elige _Más información_ (_More info_)
  y después _Ejecutar de todas formas_ (_Run anyway_).
- **macOS**: Gatekeeper no permite abrirlo. Haz Control-clic en la aplicación y elige _Abrir_
  (_Open_), o ábrela desde _Configuración del Sistema → Privacidad y seguridad → Abrir igualmente_
  (_System Settings → Privacy & Security → Open Anyway_).

El paquete Debian y los archivos comprimidos del núcleo no están afectados.

## ¿Dónde están mis datos y registros?

En las carpetas por usuario habituales de tu sistema operativo. La carpeta de datos guarda los
registros de sesión JSONL a los que sólo se añaden entradas, los registros del propio daemon y el
índice de proyectos; la carpeta de configuración guarda los ajustes y las reglas de permisos.
Las rutas de cada plataforma están en las [notas de plataformas](plataformas.md#rutas-de-datos).
Meltemi conserva estos registros localmente y no envía telemetría; los agentes que ejecutas
usan sus propias conexiones con sus proveedores.

## ¿Qué envía Meltemi por la red?

Nada propio. El daemon no abre puertos ni hace solicitudes; los agentes usan la red que utilizan
sus propias CLI. Sus herramientas integradas y servidores MCP pueden tener canales de permisos
fuera del proxy de Meltemi; consulta [agentes](agentes.md).
