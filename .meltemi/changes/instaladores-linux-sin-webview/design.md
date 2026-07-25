## Context

El gate de tamaño del pipeline es un gate bloqueante que implementa un requisito
de la verdad viva. Falló en Linux con `Meltemi_0.1.0_amd64.AppImage` = 78 678 520
B contra un presupuesto de 15 728 640 B. Ninguna corrida anterior había llegado a
ese paso en Linux, así que esta es la **primera medición de un instalador de
Linux en la historia del proyecto**: los dos QA de presupuestos publicados
(`docs/qa/2026-07-20`, `docs/qa/2026-07-25`) miden Windows y solo Windows.

El requisito que el gate implementa vive en `release-distribution` y ata número y
mecanismo en una sola frase: «El instalador SHALL mantenerse por debajo de 15 MB
por plataforma — el runtime de webview del sistema se aprovecha o se bootstrapea,
MUST NOT embeberse». El mismo techo se repite, sin el porqué, en el requisito
«Presupuestos de huella de la GUI» de `gui-shell`. El origen es el design D7 de
`gui-tauri-paridad`, que da la razón por escrito: el número es *consecuencia* de
no empaquetar motor, no una meta de frugalidad por sí misma.

Un AppImage embebe WebKitGTK por construcción. En el CLI pineado
(`@tauri-apps/cli` 2.11.4, `desktop/ui/package.json`) `AppImageConfig` expone
solo `bundleMediaFramework` —ya en su default `false`, y documentado como
«increases the bundle size by ~15-35MB»— y `files`; el bundler copia
`WebKitNetworkProcess`, `WebKitWebProcess` y el *injected bundle* en el AppDir y
después ejecuta `linuxdeploy` con el plugin de GTK, que arrastra módulos de GTK3,
cargadores de gdk-pixbuf, librsvg, módulos GIO y el árbol de esquemas de glib.
Es código, no configuración. **78 MB es el piso del formato**, no un descuido.

`DebConfig` y `RpmConfig`, en cambio, exponen `depends`: esos formatos pueden
*declarar* el motor como dependencia externa. Esa asimetría de las herramientas
es exactamente la asimetría de la promesa.

## Goals / Non-Goals

**Goals:** que toda frase pública sobre el tamaño y el motor del instalador sea
cierta para todo artefacto que publicamos; que el `.deb` no instale limpio para
después no abrir; que las dos specs que repiten el presupuesto digan el mecanismo
que lo sostiene; y que el hueco de cobertura que esto abre esté nombrado, con su
salida, en vez de disimulado.
**Non-Goals:** cambiar el presupuesto de 15 MB (no hace falta); añadir `rpm` o
Flatpak (siguiente paso, con su propia verificación); firmar instaladores;
tocar el archivo del núcleo (`tar.gz`/`zip`), que no depende de motor alguno y
funciona en cualquier distribución.

## Decisions

### D1 — Se retira el AppImage; no se ensancha el presupuesto

Las tres salidas reales eran: (a) presupuesto por formato, con el AppImage en su
propia categoría; (b) retirar el AppImage; (c) no publicar instalador de GUI en
Linux. La (c) es la peor de todas y nadie la defiende.

Se elige **(b)**. El argumento decisivo no es el número sino su significado: D7
dejó por escrito que 15 MB *codifica* «no empaquetamos motor de navegador», y
meltemi.md §7 rechaza esa vía por su nombre, cuantificando su costo como «un
orden de magnitud mayor» en tamaño de instalador. Ensanchar el presupuesto para
el AppImage no preserva la razón del número: la abandona, y obliga a volver
condicional una promesa que hoy es absoluta en cinco textos públicos («no
empaqueta un navegador»). Un artefacto de 79 MB dentro de un producto cuyo
documento fundacional presume de instaladores de pocos megabytes no es una
excepción bien etiquetada: es la refutación de la tesis.

La (a) además no compra lo que parece. Un usuario que lee «no empaqueta un
navegador» un párrafo más abajo y elige entre dos enlaces sin tamaño ni nota de
motor —que es literalmente lo que hoy ofrecen las dos páginas de descargas— se
lleva 4 MB o 79 MB según cuál toque. Etiquetarlo honestamente exige tanto texto
que el propio texto delata que el artefacto no encaja.

### D2 — El `.deb` declara `libwebkit2gtk-4.1-0` y `libgtk-3-0`

Hoy el proyecto no configura `bundle.linux.deb.depends`. Un `.deb` sin `Depends`
del motor instala sin error y después no abre: el peor modo de fallo posible,
porque el gestor de paquetes —el único componente capaz de resolverlo— dice que
todo salió bien. Declararlas convierte la promesa en mecanismo verificable por el
sistema del usuario: *uso* el motor del sistema, y el paquete lo exige.

Los nombres son los de Debian 12 / Ubuntu 22.04 en adelante, que es la línea base
que Tauri v2 documenta para WebKitGTK 4.1 y la que usa el runner. Si el bundler
ya emitiera un `Depends` equivalente, declararlo es redundante e inofensivo; si
no lo emite, arregla un fallo real. En ninguno de los dos casos conviene que la
promesa dependa del comportamiento implícito de una herramienta.

### D3 — Presupuesto único, redactado por mecanismo

El presupuesto sigue siendo 15 MB para todo instalador publicado, y las specs
dejan de poder contradecirse: el requisito de `release-distribution` enumera los
formatos que publicamos y exige que ninguno embeba el motor, atándolo a que los
formatos elegidos permitan declararlo como dependencia; el de `gui-shell` deja de
repetir el techo a secas. Un formato autocontenido no queda prohibido para
siempre —queda fuera de *este* conjunto de artefactos— pero admitir uno exigiría
volver a abrir esta decisión, que es precisamente lo que debe costar.

### D4 — El hueco se nombra, con su salida y su plazo

Fuera de la familia Debian no queda instalador de GUI. Decirlo es parte de la
decisión, no una nota al pie: quien use Fedora, RHEL, openSUSE, Arch o NixOS
compila la app desde el código (vía ya documentada y ahora correcta, con
`tauri build`) o usa el núcleo desde el `tar.gz`, que no depende de motor alguno.

La salida es `rpm`, target de primera clase del bundler que ya expone `depends` y
que devolvería Fedora/RHEL/openSUSE con la misma honestidad que el `.deb`. No
entra aquí porque exige verificar los nombres del paquete del motor en esas
distribuciones y si el runner necesita `rpmbuild`: la documentación de Tauri no
los da, y este proyecto no promete lo que no ha medido.

## Risks / Trade-offs

- **Cobertura gráfica de Linux estrechada a la familia Debian.** Es el costo
  aceptado y el motivo de que `rpm` sea change inmediata y no «algún día».
- **Un AppImage es lo que un usuario de distribución inmutable espera.** En
  Silverblue o SteamOS, «un archivo, ejecútalo» no tiene sustituto directo; ahí
  la respuesta honesta hoy es la TUI, y mañana Flatpak.
- **Declarar `Depends` con nombres de Debian 12+** deja fuera derivadas más
  antiguas. Es la misma línea base que Tauri v2 documenta para WebKitGTK 4.1, así
  que el paquete no habría funcionado en ellas de todos modos: ahora falla al
  instalar, con un mensaje que nombra lo que falta, en vez de fallar al abrir.
- **Qué invalidaría esta decisión:** que el bundler ganara una forma soportada de
  emitir un AppImage que enlace el WebKitGTK del sistema. Entonces el formato
  volvería sin tocar la promesa, y esta change habría sido el paso correcto de
  todos modos.
