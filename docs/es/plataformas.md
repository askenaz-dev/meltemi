<!-- SPDX-License-Identifier: Apache-2.0 -->

# Notas de plataformas

[English](../plataformas.md)

Meltemi admite Windows 10 1809+ / Windows 11, macOS 13+ y las distribuciones habituales de Linux
con glibc. Esta página trata la instalación, los datos locales y los ajustes de entorno que
puedas necesitar.

## La aplicación de escritorio en Linux

El núcleo, formado por el daemon y el cliente de terminal, funciona desde el archivo de la
release en cualquier distribución habitual con glibc. La aplicación de escritorio se publica
**sólo como `.deb`** y declara `libwebkit2gtk-4.1-0` y `libgtk-3-0` para que el gestor de paquetes
instale el motor que utiliza.

No se publica AppImage ni RPM. En distribuciones ajenas a la familia Debian, usa el archivo del
núcleo para el daemon y el cliente de terminal, o compila la aplicación de escritorio siguiendo
las instrucciones de compilación del repositorio. El escritorio usa la vista web del sistema
operativo en lugar de incluir un motor de navegador.

## Rutas de datos

El daemon guarda dos tipos de archivos en las carpetas por usuario habituales del sistema
operativo. La carpeta de **datos** contiene los registros de sesión JSONL a los que sólo se
añaden entradas, los registros del propio daemon, el índice de proyectos y las métricas locales.
La carpeta de **configuración** guarda los ajustes y las reglas de permisos.

- **Windows**, en el perfil itinerante: datos en `%APPDATA%\meltemi\data\` y configuración en
  `%APPDATA%\meltemi\config\`.
- **macOS**: ambos en `~/Library/Application Support/meltemi/`.
- **Linux**: datos en `$XDG_DATA_HOME/meltemi/` o `~/.local/share/meltemi/`; configuración en
  `$XDG_CONFIG_HOME/meltemi/` o `~/.config/meltemi/`.

Los artefactos del método propios de un repositorio están en `.meltemi/`, dentro de ese
repositorio.

## El socket local

El daemon escucha **sólo** en un endpoint local, nunca en un puerto de red:

- **Unix, macOS/Linux**: un socket de dominio Unix con permisos `0700`.
- **Windows**: un named pipe con una ACL limitada al usuario.

### Uso remoto mediante SSH

El daemon nunca abre un puerto de red: el acceso remoto utiliza el SSH de la persona. Qué se
ejecuta en cada extremo:

- **En el equipo del daemon, macOS/Linux**: `meltemi tunnel <user@host>` imprime el comando
  `ssh -R` que lleva el socket de este daemon al host remoto, con una comprobación periódica para
  que un túnel caído termine en vez de permanecer abierto. El otro extremo apunta
  `MELTEMI_ENDPOINT` al socket reenviado.
- **En el equipo de un cliente macOS/Linux**: se puede traer el mismo socket mediante
  `ssh -L <local.sock>:<daemon.sock> <host>`, la operación equivalente ejecutada desde el otro
  extremo.
- **Si el daemon está en Windows**: su named pipe no se puede reenviar; `ssh <pc> meltemi bridge`
  transporta la conexión por la entrada y salida estándar de SSH, y los comandos ejecutables
  mediante scripts, como `ssh <pc> meltemi permissions`, se ejecutan en el propio PC.
- **Desde cualquier cliente hacia cualquiera de las tres plataformas**: un endpoint de la forma
  `exec:<command>` hace que el cliente lance ese comando y hable el protocolo por su entrada y
  salida estándar. Así, `MELTEMI_ENDPOINT="exec:ssh <pc> meltemi bridge"` proporciona al escritorio
  y a la interfaz de terminal el alcance que el puente ya ofrecía a un script. El comando procede
  del entorno de la persona o de su configuración de usuario, nunca de un repositorio; un
  endpoint de este tipo nunca inicia un daemon local.

Un cliente cuyo `MELTEMI_ENDPOINT` apunta a un socket reenviado nunca inicia un daemon allí: si
el túnel no funciona, lo indica e identifica el túnel. El PC Windows al que te conectas, el
teléfono, el archivo `~/.ssh/config` que necesita un endpoint de comando y los ajustes de espera
se explican en [acceso remoto](../acceso-remoto.md).

### Aislar un entorno

`MELTEMI_ENDPOINT`, `MELTEMI_DATA_DIR`, `MELTEMI_CONFIG_DIR` y `MELTEMI_DAEMON_BIN` aíslan un daemon
del habitual de la persona, para tests, scripts o una segunda instalación. Un cliente sólo
inicia un daemon bajo demanda en el endpoint habitual de la persona; para permitirle iniciarlo
en un endpoint aislado, define también `MELTEMI_START_HERE=1`. Sin esta variable, un cliente que no
encuentra nada allí rechaza la conexión inmediatamente en lugar de iniciar uno.

## Windows (git-bash)

En git-bash / MSYS sobre Windows, la traducción de rutas POSIX del intérprete puede alterar las
variables de entorno cuyos valores parecen rutas Unix. Una variable como `MELTEMI_ENDPOINT`, con
la ruta de un named pipe o de un socket, puede reescribirse; por ejemplo, una `/` inicial puede
convertirse en una ruta de unidad, de modo que el cliente no encuentra el daemon.

**La solución:** desactiva la traducción de esa variable al definirla:

```
MSYS_NO_PATHCONV=1 MELTEMI_ENDPOINT='<endpoint>' meltemi status
```

También puedes definir la variable desde un intérprete nativo, como PowerShell o `cmd`, en lugar
de git-bash. Ante la duda, prefiere PowerShell en Windows para definir las variables de entorno
de Meltemi.

## Accesibilidad

El cliente respeta `NO_COLOR`, ofrece una alternativa ASCII para dibujar cuadros y, con `--json`
en los comandos que llaman al daemon, proporciona una salida legible por máquinas que nunca depende del estilo de
la terminal. Consulta [accesibilidad](accesibilidad.md).
