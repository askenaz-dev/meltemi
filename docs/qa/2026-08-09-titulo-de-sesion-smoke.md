<!-- SPDX-License-Identifier: Apache-2.0 -->
# Smoke conducido — titulo-de-sesion (2026-08-09)

Medición sobre el **binario de release** con la GUI conducida por CDP. Confirma
lo que los tests de fuente prueban por orden y por ausencia, y una cosa que
solo el binario podía enseñar: cómo se lee una tira **mixta**, con sesiones
tituladas junto a otras que nunca lo fueron.

## Montaje

Receta de `docs/qa/2026-08-09-piel-de-pestanas-smoke.md`, con sus dos
condiciones: patch de puerto en `desktop/tauri.conf.json` (revertido al
terminar, verificado) **y** `WEBVIEW2_USER_DATA_FOLDER` propio y nuevo.

- Daemon, CLI, mock y GUI construidos **en un `CARGO_TARGET_DIR` aparte**: los
  binarios del árbol son anteriores al título y `target/release` está tomado
  por los procesos del mantenedor, que no se detienen.
- Dos repos fixture (`harbour`, `quay`) con endpoint, datos y config propios.
- **La mezcla es deliberada**: `harbour` ya contenía sesiones creadas con el
  binario anterior, que no tienen título. Junto a las nuevas, forman
  exactamente el caso que la degradación honesta tiene que sostener.

## Lo que la change pedía medir

### La pestaña dice de qué trata la sesión

Confirmado. Las cinco sesiones creadas para el smoke se leen por su trabajo, no
por su hash:

```
■ Corregir el login del portal y su mensaje de error
■ Medir el arranque en frio de la interfaz
■ Documentar el harness global y sus cuatro ambitos
■ Preparar la release firmada de la version siguiente
```

El nombre accesible de cada una añade su estado —`… — ended`— y el emergente
conserva la historia completa: `mock · ended · <uuid> · <ruta del proyecto>`.
El identificador no desapareció; se movió a donde no gasta ancho.

### Una sesión sin título se nombra como antes

Confirmado, y es la razón de haber conservado el fixture anterior. Entre las
tituladas, una sesión creada con el binario previo aparece como:

```
■ mock a296f430
```

Exactamente lo que mostraba antes de esta change. La degradación no es una
promesa del design: está fotografiada junto a las que sí tienen nombre.

### El proyecto se antepone solo ante ambigüedad

Confirmado **en los dos sentidos**, que es lo que hacía falta para no dar por
buena media regla.

Con pestañas de dos proyectos abiertas:

```
■ quay · Publicar la guia de instalacion
■ harbour · Corregir el login del portal y su mensaje de error
■ harbour · mock a296f430
```

Y al cerrar las dos pestañas de `quay`, dejando todas en un solo proyecto, los
rótulos **pierden el prefijo en el mismo gesto**:

```
■ Preparar la release firmada de la version siguiente
■ Corregir el login del portal y su mensaje de error
■ mock a296f430
```

Medido por el driver: ningún rótulo conserva el nombre del proyecto
(`anyPrefixed: false`). El ancho vuelve al nombre, que era el objetivo.

## Reversión

El patch de `additionalBrowserArgs` fue retirado al terminar y su ausencia
verificada en el árbol antes de commitear. Los procesos del mantenedor —su GUI
y su daemon— no se detuvieron en ningún momento: el smoke corrió con sus
propios binarios, su propio endpoint y su propio user data folder.
