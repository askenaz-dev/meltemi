## Why

El gate de tamaño del pipeline falló en Linux por primera vez en la historia del
proyecto: el AppImage pesa **78 678 520 B**, 5 veces el presupuesto de 15 MB. El
gate no está mal calibrado; disparó la primera vez que llegó a apuntar a Linux
(las corridas anteriores morían antes, en `shopt -s globstar`, que bash 3.2 de
macOS no tiene).

El número no es arbitrario y por eso no se puede ensanchar en silencio. La spec
viva lo ata a un mecanismo en la misma frase: «El instalador SHALL mantenerse por
debajo de 15 MB por plataforma — el runtime de webview del sistema se aprovecha o
se bootstrapea, **MUST NOT embeberse**», y enumera el AppImage entre los
artefactos exigidos. Las dos mitades son inconciliables para ese formato: un
AppImage es autocontenido por definición y arrastra WebKitGTK con su cierre de
dependencias. El requisito se contradice consigo mismo.

El origen es un error de hecho, no una regresión: el design D7 de
`gui-tauri-paridad` afirmó que en Linux «el runtime de webview del SO se
aprovecha (WKWebView, WebKitGTK) … nunca se embebe: así el instalador respeta
< 15 MB», confundiendo «enlaza contra el WebKitGTK del sistema» (cierto) con «el
AppImage no lo lleva dentro» (falso). Nunca se midió un instalador de Linux.

Y no hay palanca de configuración: en el CLI pineado (`@tauri-apps/cli` 2.11.4)
`AppImageConfig` expone exactamente dos campos —`bundleMediaFramework`, que ya
está en su default `false`, y `files`—, ninguno excluye bibliotecas; el bundler
copia los procesos auxiliares de WebKitGTK y corre `linuxdeploy` con el plugin de
GTK por código, no por opción. 78 MB ya es el piso de ese formato.

Por debajo de todo esto hay una decisión de producto anterior a la spec:
meltemi.md §7 rechaza por su nombre la vía de empaquetar un motor de navegador,
cuantificando su costo como «un orden de magnitud mayor» en tamaño de instalador.
El AppImage **es** esa vía. Publicarlo obligaría a volver condicional cada frase
de «no empaqueta un navegador» del README, el LEEME, las dos páginas de descargas
y `docs/release.md`.

## What Changes

- **El AppImage deja de publicarse.** Se retira `appimage` de los targets de
  `desktop/tauri.conf.json`, del gate de tamaño y del paso de normalización de
  nombres. La promesa queda literalmente cierta para todo artefacto publicado:
  ninguno lleva motor de navegador, y todos caben en 15 MB.
- **El `.deb` declara su dependencia del motor del sistema.** Hoy no declara
  ninguna, así que instala limpio y falla al arrancar en una máquina sin
  WebKitGTK: se instala bien y no abre. Se declaran `libwebkit2gtk-4.1-0` y
  `libgtk-3-0` en `bundle.linux.deb.depends`. Declarar la dependencia es la
  forma honesta de decir «uso el motor del sistema, no lo llevo dentro»; el
  formato lo permite y el AppImage estructuralmente no.
- **Las specs dicen la verdad sobre el mecanismo.** Delta MODIFIED en dos
  capacidades: `release-distribution` (el requisito de instaladores enumera los
  formatos que publicamos y ata el presupuesto a la declaración de dependencia,
  no al empaquetado del motor) y `gui-shell` (el presupuesto de huella, que hoy
  repite el techo sin el porqué).
- **Se corrige la prosa pública en los dos idiomas**: la tabla de descargas del
  README y el LEEME, el paso a paso de Linux, las dos páginas `downloads.html`,
  la lista de artefactos de `docs/release.md` y su resumen en español. Donde hoy
  se ofrecen AppImage y deb como opciones equivalentes, queda el `.deb` con su
  requisito de sistema dicho en términos del usuario.
- **Se nombra el hueco en vez de disimularlo**: quien no use una distribución de
  la familia Debian se queda sin instalador de GUI hasta que exista un `.rpm`
  (target de primera clase de Tauri, ya disponible, pendiente de verificar sus
  nombres de paquete en Fedora/RHEL antes de prometerlo) o un Flatpak. Mientras
  tanto, el núcleo (daemon + TUI) sigue funcionando en cualquier distribución
  desde el tar.gz, y la app de escritorio se compila desde el código con la vía
  ya documentada.
- **Se publica la primera medición de Linux.** El QA de presupuestos incorpora el
  tamaño real del `.deb` medido en el runner, junto al MSI ya publicado. Ningún
  número se estima: hasta que la corrida lo reporte, no se escribe.

## Capabilities

### Modified Capabilities
- `release-distribution`: el requisito de instaladores de la GUI enumera
  MSI/DMG/deb, ata el presupuesto a la declaración de dependencia del motor del
  sistema y exige que ningún artefacto publicado lo embeba.
- `gui-shell`: el presupuesto de huella declara el mecanismo que lo sostiene, de
  modo que las dos capacidades no puedan volver a divergir.

## Impact

- `desktop/tauri.conf.json` (targets y `bundle.linux.deb.depends`),
  `.github/workflows/release.yml` (gate y normalización),
  `desktop/tests/surface.rs` y `core/meltemid/tests/scenarios_sitio.rs` (los
  tests que fijan formatos y nombres de artefacto), `README.md`, `LEEME.md`,
  `site/downloads.html`, `site/es/downloads.html`, `docs/release.md`,
  `docs/qa/`.
- El sitio y el pipeline se mueven juntos por obligación: la spec `product-site`
  exige que la verificación falle si el sitio nombra un artefacto que el pipeline
  no produce.
- Cobertura de Linux: se estrecha a la familia Debian en la superficie gráfica y
  no cambia en la de terminal. Es el costo declarado de la decisión.

## Fuera de alcance

- **Añadir el target `rpm`**: es el siguiente paso natural y el que devuelve
  Fedora/RHEL/openSUSE a la mesa, pero exige verificar los nombres de paquete del
  motor en esas distribuciones y que el bundler no necesite `rpmbuild` en el
  runner. No se promete lo que no se ha medido; entra como change propia.
- **Flatpak**, por la misma razón y con más superficie (portales, sandbox).
- **Firmar los instaladores** (Authenticode, notarización): deuda declarada
  aparte, dependiente de certificados comprados.
- **Cambiar el presupuesto de 15 MB**: no hace falta. Con el AppImage fuera,
  todos los artefactos publicados caben, y el número sigue significando lo mismo
  que significaba.
