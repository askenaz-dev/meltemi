<!-- SPDX-License-Identifier: Apache-2.0 -->
# Verificación de campo — `identidad-propia` contra un cliente real

**Fecha**: 2026-10-02 · **Plataforma**: Windows 11 Pro
**Cliente de malla**: Tailscale **1.102.4**, instalado en este equipo el
2026-10-02 en `C:\Program Files\Tailscale\`. **Sin sesión iniciada.**
**Binarios**: `target/release/{meltemi,meltemid}.exe` de `main` (1ae42f4).

Esto cierra el primer punto de «lo que no midió» del smoke del 2026-10-01: toda
aquella corrida fue contra `mock-mesh`, porque no había cliente instalado.

## Método

Daemon, configuración y datos **aislados** — `MELTEMI_ENDPOINT`,
`MELTEMI_CONFIG_DIR`, `MELTEMI_DATA_DIR` propios, bajo `%TEMP%`— para no tocar
el daemon ni la configuración del usuario. El rail de arranque hizo su trabajo
sin que se lo pidieran: con `MELTEMI_ENDPOINT` puesto y sin `MELTEMI_START_HERE`,
el cliente **se negó** a levantar un daemon en un endpoint ajeno y dijo cómo
hacerlo a propósito. Está anotado porque es la primera vez que ese rail se
ejerce fuera de sus tests.

## Los tres estados que se pueden medir sin iniciar sesión

| Estado | Medido, contra el cliente real |
| --- | --- |
| `not_configured` | sin tabla `[identity]`: `no identity (not_configured)` + «identity is off: add an `[identity]` table with `mesh` to your user config.toml» |
| `client_missing` | con `mesh = "tailscale"` y el binario **fuera** del PATH del daemon: `could not run 'tailscale status --json': program not found — install your mesh client or set [identity] command to its path`. No fue un montaje: el instalador no actualiza el PATH de un shell ya abierto, así que el daemon heredó un PATH sin Tailscale. El remedio impreso es exactamente la perilla que lo arregla |
| `not_linked` | con `command` apuntando al binario: `your mesh client is installed and this machine has no session on the mesh (it says 'NeedsLogin'): run the linking gesture to sign in` |

El tercero es el que importa: **es la salida real del cliente**, no la del
simulado, y el estado del backend viaja literal a la explicación.

`identity-link` compuso el gesto con la ruta real, citada porque tiene espacios,
y con el nombre verdadero de este equipo:

```
'C:\Program Files\Tailscale\tailscale.exe' up --hostname ASKENAZ-PC
```

## Lo que la realidad corrigió

**1. El gesto para PowerShell no se ejecuta cuando la ruta lleva espacios.**
`identity-link --json` emite el mismo texto en `posix` y en `powershell`, y en
PowerShell una línea que **empieza** por una cadena entrecomillada se analiza en
modo expresión, no en modo comando. Medido de dos maneras:

```
Parser::ParseInput("'C:\Program Files\...\tailscale.exe' version")
  -> PARSE ERROR: Unexpected token 'version' in expression or statement.
Invoke-Expression "'C:\Windows\System32\whoami.exe'"
  -> devuelve la CADENA, no ejecuta nada
```

Falta el operador de llamada: `& 'ruta con espacios' up …`. El caso por defecto
—`tailscale` en el PATH, sin comillas— sí corre, así que el defecto solo asoma
cuando el usuario fija `command`… que es justo lo que la instalación por defecto
de Windows obliga a hacer. Windows es plataforma de primera clase (§7), así que
esto es un defecto, no una aspereza. **No se corrige aquí**: no es de ninguna
change activa y se anota como propuesta, no se cuela.

**2. El fixture `unlinked` es menos fiel que la realidad.** El cliente real, sin
sesión, devuelve un `Self` **poblado** (`HostName`, `OS`, `UserID: 0`,
`Online: false`) junto a `BackendState: "NeedsLogin"`; `mock-mesh` devuelve
`"Self": null` para ese mismo escenario. El parser acierta en ambos casos porque
la puerta de `BackendState` se evalúa antes que `Self`, de modo que no hay bug
que arreglar — pero un fixture que no se parece a lo que manda el cliente es un
fixture que deja de proteger el día que la puerta se mueva. También queda
anotado.

## Lo que esta verificación **no** midió

- **El estado enlazado**, que es el único que muestra nombre, malla y máquinas.
  Requiere iniciar sesión en la malla del usuario con su propio proveedor, y eso
  lo hace **él**: la constitución §2 prohíbe que Meltemi —o quien trabaje en su
  nombre— se autentique por él. El gesto quedó impreso y listo para que lo corra.
- **`identity-link --exec`**, por lo mismo: ejecutarlo abre un navegador y
  empieza un inicio de sesión.
- **macOS y Linux**: corrida de Windows.
