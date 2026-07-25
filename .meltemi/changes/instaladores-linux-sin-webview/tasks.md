## 1. Empaquetado

- [x] 1.1 Retirar `appimage` de los targets de `desktop/tauri.conf.json` y declarar `bundle.linux.deb.depends` con `libwebkit2gtk-4.1-0` y `libgtk-3-0`
- [x] 1.2 Sacar el AppImage del gate de tamaño y del paso de normalización de nombres en `.github/workflows/release.yml`, con el comentario que explique por qué

## 2. Verdad viva y tests

- [x] 2.1 Actualizar `desktop/tests/surface.rs` (formatos del pipeline) y `core/meltemid/tests/scenarios_sitio.rs` (nombres de artefacto del sitio) a los tres formatos publicados
- [x] 2.2 Cubrir los escenarios nuevos: formato autocontenido no publicado, y el paquete declarando el motor del sistema

## 3. Prosa pública en dos idiomas

- [x] 3.1 Corregir `README.md` y `LEEME.md`: tabla de descargas, paso a paso de Linux y la frase de la promesa
- [x] 3.2 Corregir `site/downloads.html` y `site/es/downloads.html`: la celda de Linux y la frase «no empaqueta un navegador»
- [x] 3.3 Corregir `docs/release.md`: tabla de artefactos, párrafo de instaladores, resumen en español, y la atribución del presupuesto (es meltemi.md §12, no la constitución §12)
- [x] 3.4 Nombrar el hueco de cobertura y su salida en `docs/plataformas.md` y en el plan de cambios

## 4. Medición

- [ ] 4.1 Publicar en `docs/qa/` el tamaño del `.deb` medido en el runner, junto al MSI ya publicado; nada estimado
