# Design — companero-movil

## Context

Verificado el 2026-10-02, sobre `main` en `9c8c5b7`:

- **Los cinco prerrequisitos están aterrizados**: `estado-en-vivo`
  (`436496b`), `via-ssh-del-cliente` (`1c446ec`), `aviso-de-espera-opt-in`
  (`4eaf6cf`), `identidad-propia` (`b359bb9`) y `maquinas-remotas`
  (`9c8c5b7`). `verbos-del-agent-boss` ya lo estaba.
- **El canal existe entero y está probado**: `exec:<comando>` lanza el comando
  y habla JSON-RPC por su stdio, sin shell, dentro de un ámbito de proceso
  (`meltemi-client/src/exec.rs`); `meltemi bridge` bombea el endpoint local
  sobre stdio. `ssh <pc> meltemi bridge` es un canal completo, con test
  (`tui/tests/exec_endpoint.rs`). Lo que falta del lado del móvil es **quién
  hace de `ssh`**: en Android no hay un `ssh` del sistema que lanzar.
- **El cliente sabe cambiar de máquina sin reiniciarse** y leer la lista del
  usuario (`meltemi-client/src/machines.rs`), y sabe leer la identidad de la
  malla (`meltemid/src/identity.rs`). La app móvil consume ambas.
- **El daemon no distingue una conexión tunelizada de una local** y no debe:
  `mobile-companion` lo exige y `remote-access` lo repite.
- **La superficie de escritorio es Svelte 5** en `desktop/ui`, con sus stores,
  su catálogo de mensajes ES/EN, su i18n y sus componentes. `App.svelte` es el
  **shell de escritorio**: barra lateral, pestañas, tres zonas. El design
  system ya prevé el compañero (`--touch-min: 44px`).
- **La maqueta** (`design-system/Meltemi Mobile.dc.html`) dibuja tres
  pantallas —conexión, aprobar, flota y ajustes— y deja **una pregunta
  abierta por escrito**: si con la app en segundo plano el sistema puede
  mostrar un aviso local de permiso pendiente, «depende de que la conexión
  sobreviva en background, lo que en iOS no está garantizado. Está sin
  resolver y conviene decidirlo antes de escribir la app». D5 la responde.
- **`deny.toml` prohíbe HTTP y TLS** en el workspace: `reqwest` (único
  envoltorio permitido: `tauri`), `hyper`, `rustls`, `native-tls`, `openssl`,
  `ureq`, `curl`, `isahc`. Ninguna prohibición menciona SSH.
- **El toolchain Android está entero en este equipo**: SDK con plataformas
  33–36, NDK 27.0.12077973, build-tools 35/36, `adb`, emulador con dos AVD
  (`Medium_Phone_API_36.1`, `Pixel_Tablet`), JDK 21, Tauri CLI 2.11.4. Los
  cuatro targets de Rust **se instalaron el 2026-10-02** y ya están. Ni
  `ANDROID_HOME` ni `NDK_HOME` están exportadas, pero las rutas existen.

## Goals / Non-Goals

**Goals**: una app Android que haga los cuatro verbos del Agent Boss —
monitorear, aprobar, revisar, dirigir — contra el daemon del usuario, por un
canal SSH que la app misma habla, sin relé, sin puerto y sin cuentas.

**Non-Goals**: autoría de código o de specs (la spec lo prohíbe salvo enmienda
fundacional); relé en la nube o push sin túnel (§3); iOS en esta change; un
cliente SSH en el crate de escritorio, donde el `ssh` es el del usuario.

## Decisions

### D1 — Android primero, en `mobile/`, reusando lo que hay debajo del shell

Un crate `mobile/` con su propio `tauri.conf.json`. El frontend **no es una
copia** de `desktop/ui` ni un shell de escritorio encogido: es una **segunda
entrada de la misma aplicación Vite**, `mobile.html` + `Mobile.svelte`, que
reutiliza lo que ya está probado y es independiente del shell —los stores, el
catálogo ES/EN, la i18n, `identity.ts`, `machines.ts`, `session-state.ts`,
`attention.ts`— y aporta su propia navegación de cuatro verbos.

Reusar el shell sería lo contrario de un compañero: la barra lateral, las
pestañas y las tres zonas existen porque hay un ratón y 1.280 píxeles. Lo que
se reutiliza es lo que no depende de eso, que es casi todo lo que importa.

### D2 — El SSH lo habla la app, con `russh`, y `deny.toml` **gana prohibiciones**

En Android no hay un `ssh` que lanzar, así que el canal `exec:` no sirve tal
cual: la app tiene que hablar SSH ella misma y ofrecer al cliente un stream
que, del otro lado, es `meltemi bridge`.

**`russh 0.63.3`**, y la verificación está hecha, no supuesta:

- **Licencia Apache-2.0**, la misma del proyecto (§12). Auditoría trivial.
- **Su árbol obligatorio es RustCrypto puro**: `aes`, `chacha20`-equivalentes,
  `sha2`, `hmac`, `ed25519-dalek`, `curve25519-dalek`, `p256`/`p384`/`p521`,
  `ml-kem` (post-cuántico), `pbkdf2`, `scrypt`, más `tokio` y `futures`, que el
  workspace ya tiene. **No trae `rustls`, ni `openssl`, ni `native-tls`, ni
  `hyper`, ni `reqwest`.**
- `ring` y `aws-lc-rs` son **opcionales** y se quedan apagados.

De ahí la decisión que invierte lo que la propuesta anticipaba: **`deny.toml`
no necesita una excepción**, porque SSH no es HTTP ni TLS y nada del árbol
obligatorio está prohibido. Lo que `deny.toml` gana son **dos prohibiciones
nuevas** —`ring` y `aws-lc-rs`— para que la puerta que russh deja entornada con
sus features no la abra nadie sin argumentarlo aquí por escrito, que es
exactamente la disciplina que `reqwest` ya tiene. Una change que endurece la
auditoría en vez de relajarla.

**Lo que sí hay que declarar, porque §10 lo pide y callarlo sería elegir la
versión cómoda de la verdad**: russh pina `ssh-key =0.7.0-rc.11`, un
**candidato de versión**. Está pineado con `=` y no con un rango, y es el crate
de formatos de clave de RustCrypto. Las alternativas son peores: `libssh2-sys`
es C y arrastra OpenSSL, que está prohibido; `thrussh` es el antecesor sin
mantenimiento. Se acepta, se escribe aquí, y `cargo deny` lo audita como a
cualquier otra.

El cliente SSH vive **solo en `mobile/`**. El crate de escritorio no lo gana ni
lo ve: allí el `ssh` es el del usuario, y eso no cambia.

### D3 — La clave es de la app, generada en el teléfono, y nunca la de otra app

La app **genera su propio par ed25519 en el teléfono** la primera vez, muestra
la **pública** para que el usuario la añada a su `authorized_keys`, y guarda la
privada cifrada en reposo con una clave simétrica del **Android Keystore**.

Dicho así de concreto porque la frase cómoda —«la clave vive en el Keystore»—
no es cierta y hay que escribirlo: el Keystore de Android **no exporta claves
privadas asimétricas**, y una biblioteca SSH en espacio de usuario necesita la
clave para firmar. Lo que el Keystore guarda es la clave simétrica que cifra la
privada en disco, que es el patrón real de la plataforma. Prometer firma por
Keystore exigiría JNI y un backend de firma propio en russh: no está en esta
change, y decirlo es mejor que insinuarlo.

**Meltemi no lee claves de otras apps, ni pide la del usuario, ni la importa**
(§2). El par es de esta app y solo sirve para esto; revocarlo es borrar una
línea del `authorized_keys`, que es la propiedad que lo hace aceptable.

El host key del servidor se **fija en el primer uso** y se compara en cada
conexión; un cambio **corta y avisa**, nunca se acepta en silencio.

### D4 — Suspensión honesta: ningún socket en segundo plano

La pregunta que la maqueta dejó abierta se responde aquí, y se responde que
**no**: la app **no mantiene el socket vivo en segundo plano** y no intenta
mostrar avisos locales de permisos pendientes.

Tres razones, y la tercera es la que decide. Técnicamente, en iOS no está
garantizado y en Android cuesta un servicio en primer plano con su
notificación permanente. Constitucionalmente, un cliente conectado es el que
sostiene el deny-by-default: una app suspendida que *pareciera* conectada haría
que el daemon esperara a alguien que no está mirando. Y porque **ya existe la
respuesta correcta**: el aviso de espera opt-in (`aviso-de-espera-opt-in`),
que sale de la infraestructura del propio usuario y es lo que lo trae de
vuelta. Añadir un segundo camino para lo mismo, peor y con una conexión
colgando, sería construir el problema.

Al volver al frente: reconectar y **reconciliar** con `session/list` y las
afirmaciones de `session/state`, que para eso existen.

### D5 — Los cuatro verbos, y lo que cada pantalla consume

Todo RPC que la app usa **ya tiene casa en la TUI y en la GUI** (regla de
subconjunto, §4). No se añade ni un método.

| Pantalla | Verbo | Consume |
| --- | --- | --- |
| Conexión | — | la lista de `maquinas-remotas`, `initialize`, `status` |
| Sesiones | monitorear | `session/list`, `session/state`, `status` |
| Petición | aprobar | `permission/pending`, `permission/decide`, `permission/changed` |
| Revisión | revisar | `worktree/diff`, `sdd/review`, `sdd/review-decide`, `worktree/merge-file` **con confirmación** |
| Dirigir | dirigir | `session/direct`, `session/cancel` |
| Ajustes | — | `identity/status`, la lista de máquinas |

**Revisar es decidir, no redactar.** No hay editor, no hay campo de texto
libre sobre contenido, y adoptar un archivo exige confirmación explícita, como
manda la spec. El compositor de «dirigir» es texto para el agente, que es una
instrucción y no autoría de archivos.

### D6 — Qué se ve, con el pulgar en mente

Navegación inferior de cuatro destinos (sesiones, permisos, revisión,
ajustes), objetivos de 44 px (`--touch-min`, que ya existe), y la regla que la
maqueta ya fijó: **la acción destructiva nunca queda bajo el pulgar por
defecto** y el layout no se mueve mientras corre un temporizador. Sin túnel no
hay nada que mostrar, y esa pantalla es la más importante: dice qué falta y
cómo se arregla, no un error.

### D7 — El APK, su presupuesto y su firma

Presupuesto declarado **antes de compilar**, como se hizo con la TUI: **≤ 25
MB** por ABI en release. Se mide y se escribe; si no cabe, se dice, no se
ajusta el número a lo que salió.

`arm64-v8a` es el ABI de la release; `x86_64` se construye para el emulador y
no se publica. La firma de release es tarea del mantenedor con su propio
keystore, como el resto de la distribución: esta change deja el flujo y el
`.gitignore`, nunca una clave en el árbol.

### D8 — Lo que esta change no hace

iOS (el mismo código, otro trámite y otra tienda), autoría de ningún tipo,
relé, push sin túnel, y notificaciones locales de permisos. Cada uno está
escrito arriba con su razón.

## Risks / Trade-offs

- **El build de Android en CI** es el riesgo real de calendario: NDK, targets
  y tiempo. Mitigado haciéndolo primero en local, con los targets ya
  instalados, y dejando CI para una tarea propia que puede fallar sin
  bloquear al resto.
- **`ssh-key` en RC**: declarado arriba. Pineado y auditado.
- **La reconciliación tras volver al frente** puede ser cara si una sesión
  tiene un log largo. Se reconcilia con `session/list` y `session/state`, que
  son pequeños, y el transcript se pide solo de la sesión que se abre.
- **Una clave por teléfono** significa que perder el teléfono es borrar una
  línea del `authorized_keys`. Es la propiedad que la hace aceptable, y va
  escrita en la documentación, no solo aquí.
