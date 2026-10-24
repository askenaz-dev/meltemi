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

Queda una acción del mantenedor, **después** de publicar la 0.2.0: apagar GitHub
Pages en este repositorio (Settings → Pages, o `gh api -X DELETE
repos/askenaz-dev/meltemi/pages`) y borrar su entorno `github-pages`.

## 2. La primera release por el camino del push

Desde `release-con-el-push` nadie firma a mano: la release la publica el
pipeline, firmada por la atestación de su workflow, cuando un commit trae una
versión nueva a `main`. Procedimiento completo en
[`docs/release.md`](../release.md) («Making a release»). Antes de la primera,
en este orden:

1. El entorno `release` de este repositorio, su secreto `WEB_DISPATCH_TOKEN` y
   los rulesets aplicados desde el Terraform de meltemi-web.
2. `cargo run -p meltemi-release -- set-version 0.2.0`, commit y push a `main`.
3. Con `main` ya en 0.2.0, borrar el borrador v0.1.1 y su tag: mientras ese tag
   existe, un push con la versión 0.1.1 no puede publicarse solo.

No existe clave de firma que generar, guardar ni rotar, y no debe crearse una.

## 3. Namespaces

- ✅ Organización GitHub `askenaz-dev` · ✅ organización npm `askenaz-dev`.
- ✅ Dominio `meltemi.dev` (pendiente el DNS de arriba).
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
