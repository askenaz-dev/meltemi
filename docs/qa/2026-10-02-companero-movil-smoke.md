<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — `companero-movil`

**Fecha**: 2026-10-02 · **Plataforma de build**: Windows 11 Pro ·
**Dispositivo**: emulador Android, AVD `Medium_Phone_API_36.1` (API 36, x86_64)
**Toolchain**: SDK con plataformas 33–36, NDK 27.0.12077973, JDK 21,
**Tauri CLI nativo 2.12.1** (`cargo install tauri-cli`), los cuatro targets de
Rust de Android instalados el 2026-10-02.

## Qué se midió

| Escenario | Medido |
| --- | --- |
| El APK instala y arranca | `adb install -r` → `Success`; `am start` abre `dev.meltemi.mobile/.MainActivity` |
| Sin canal, la pantalla dice qué falta | arranca en **«No tunnel»**: «Remote control exists only while there is an SSH tunnel of yours to your daemon. There are no relays and no open ports, and that is deliberate.» y debajo cómo se arregla |
| …y no ofrece otra vía | ni relé, ni puerto, ni cuenta en ninguna parte de esa pantalla |
| Cuatro destinos y no un quinto | Sessions · Permissions · Review · Settings, con Sessions al frente |
| Los objetivos son pulsables | la navegación respeta `--touch-min` (44 px) y las filas también |

**Tamaños, medidos y no estimados**:

| Artefacto | Tamaño |
| --- | --- |
| APK **release** arm64 (sin firmar) | **10,69 MB** — cabe en el techo de 25 MB |
| `libmeltemi_mobile.so` release arm64 | 9,15 MB |
| AAB debug universal | 44,71 MB |
| APK **debug** universal | 322,93 MB (sin despojar, todas las ABI; **no** comparable) |

## Lo que el dispositivo corrigió

`env(safe-area-inset-bottom)` **sola no bastó**: la barra de gestos del sistema
cruzaba las etiquetas «Permissions» y «Review». Se ve en la primera captura y no
se deduce desde un escritorio. Corregido a `max(env(safe-area-inset-bottom),
16px)` —el inset real cuando el dispositivo informa uno, y un piso cuando no— y
vuelto a medir: la barra queda limpia debajo de las etiquetas.

## El fallo del build, y por qué el mensaje no lo dice

El primer `tauri android build` **falló** con:

```
Execution failed for task ':app:rustBuildX86_64Debug'.
> A problem occurred starting process 'command 'C:\nvm4w\nodejs\node.bat''
```

Eso parece un problema de extensión y **no lo es**. El proyecto Gradle generado
registra **cómo se invocó el CLI de Tauri** para poder volver a llamarlo; desde el
shim de npm registra `node` con el nombre del script como argumento, que Gradle no
puede arrancar. El código generado prueba `.exe`, `.cmd` y `.bat` por turno y
lanza la **última** excepción, así que el mensaje nombra el último recurso y no la
causa.

Remedio: el **CLI nativo** (`cargo install tauri-cli --version "^2.11"`), que hace
que el proyecto registre `cargo`. Anotado en `mobile/README.md` con su razón,
porque un build que depende de algo que nadie escribió es un build que funciona
una vez.

## Lo que este smoke **no** midió

- **Una conexión real por SSH desde el teléfono**: no se midió un extremo a
  extremo teléfono → `sshd` → `meltemi bridge`. El emulador no tiene acceso al
  named pipe del host, y montar un `sshd` para la ocasión habría medido OpenSSH.
  El canal tiene sus tests en el crate, y el puente el suyo en
  `tui/tests/exec_endpoint.rs`; lo que se midió aquí es la app y su pantalla sin
  canal, que es el estado en el que arranca cualquiera.
- **Aprobar, revisar y dirigir contra un daemon**: dependen de lo anterior. Sus
  reglas —una decisión por petición, adoptar tras dos puertas, un solo campo de
  texto en toda la app— están asertadas en `desktop/tests/scenarios_mobile.rs` y
  en los tests de UI.
- **Un teléfono de verdad**, iOS, y la firma de release: lo primero y lo segundo
  quedan; la firma es del mantenedor con su propio keystore.
