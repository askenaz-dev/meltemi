<!-- SPDX-License-Identifier: Apache-2.0 -->
# Checklist de lanzamiento (mantenedor)

Los registros y activos que no vive ningún agente: dominio, firma, namespaces.
Cada línea dice qué hacer, con qué comando se comprueba y qué queda pendiente.
Los procedimientos largos viven en `docs/release.md`; aquí está el orden.

## 1. Dominio `meltemi.dev`

Desde el 2026-10-21 meltemi.dev lo sirve
[meltemi-web](https://github.com/askenaz-dev/meltemi-web) desde Cloudflare, con
su DNS declarado en Terraform (`packages/infra`, decisión 0004 de ese
repositorio); el registrador sigue siendo Dynadot. Este repositorio ya no
publica sitio alguno: `site/` y su job de Pages salieron con
`release-con-el-push`, y las capturas que la web usa se escriben en
`docs/media/`.

✅ GitHub Pages apagado en este repositorio y su entorno `github-pages` borrado
el 2026-10-26 (`release-con-el-push` 7.3), después de comprobar que Cloudflare
sirve `meltemi.dev` y `/ide/`. El único entorno que queda es `release`.

## 2. La release por el camino del push

Desde `release-con-el-push` nadie firma a mano: la release la publica el
pipeline, firmada por la atestación de su workflow, cuando un commit trae una
versión nueva a `main`. Procedimiento completo en
[`docs/release.md`](../release.md) («Making a release»).

- ✅ El entorno `release`, su secreto `WEB_DISPATCH_TOKEN` y los rulesets de
  `main` y de los tags `v*`, aplicados desde el Terraform de meltemi-web
  (2026-10-23).
- ✅ **v0.2.0** publicada el 2026-10-23 por el push de `set-version 0.2.0`, con
  la atestación y su bundle entre los assets; el aviso a la web lanzó el deploy
  de meltemi-web y `/ide/` y `/es/ide/` la nombran.
- ✅ El borrador v0.1.1 está borrado. Su tag se queda porque el ruleset de tags
  no admite borrarlo; no estorba, porque el gate pide una versión mayor que
  todo tag.
- ⬜ `WEB_DISPATCH_TOKEN` lleva hoy el token de administración de Terraform de
  la organización, por decisión del mantenedor del 2026-10-23; queda pendiente
  sustituirlo por el token estrecho que pide el design (solo `meltemi-web`,
  permiso Actions de lectura y escritura). Cuando caduque, el aviso falla en
  rojo con la release ya publicada (`docs/release.md`, «The web notice»).

No existe clave de firma que generar, guardar ni rotar, y no debe crearse una.

## 3. Namespaces

- ✅ Organización GitHub `askenaz-dev` · ✅ organización npm `askenaz-dev`.
- ✅ Dominio `meltemi.dev`, servido por Cloudflare desde el 2026-10-21.
- ⬜ Crates `meltemi`, `meltemid`, `meltemi-proto`: verificados libres, sin
  reservar. Requiere el token de crates.io del mantenedor; el procedimiento y el
  criterio de qué nombre reclamar primero están en
  [`docs/release.md`](../release.md) («Crate namespace»).

## 4. Immutable releases

✅ Activado el 2026-07-26. Los assets de una release publicada ya no se pueden
añadir, cambiar ni borrar, y el tag queda fijo a su commit — cierra el hueco que
ni los checksums ni la firma cierran, porque hasta ahora nada impedía reemplazar
un artefacto ya publicado.

Consecuencia operativa que hay que tener presente: una release publicada **no
se corrige**, solo se sustituye por la siguiente. Por eso el pipeline sube todo
a un borrador y lo publica al final, y comprueba que no falte nada; una
release defectuosa se repudia publicando la versión siguiente y declarándola
con fecha (`docs/release.md`, «Repudiating a release»).

## 5. Deuda de firma de instaladores

El MSI y el DMG salen sin firma de plataforma: Authenticode y la notarización de
Apple exigen certificados comprados. Hasta entonces el instalador avisa en el
primer arranque. Está declarado en `docs/plan-de-cambios.md` y en el sitio; no se
disimula.
