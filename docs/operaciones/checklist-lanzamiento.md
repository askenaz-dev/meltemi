<!-- SPDX-License-Identifier: Apache-2.0 -->
# Checklist de lanzamiento (mantenedor)

Los registros y activos que no vive ningún agente: dominio, firma, namespaces.
Cada línea dice qué hacer, con qué comando se comprueba y qué queda pendiente.
Los procedimientos largos viven en `docs/release.md`; aquí está el orden.

## 1. Dominio `meltemi.dev` → GitHub Pages

El repositorio ya declara el dominio de dos formas —`site/CNAME` y el ajuste de
Pages— así que solo falta el DNS del registrador (Dynadot). El sitio se publica
desde el workflow, no desde una rama.

**Registros en el panel DNS del dominio.** El apex no admite `CNAME`, así que va
por dirección; los cuatro valores de cada familia son los de GitHub Pages y hay
que poner los cuatro (son el balanceo, no alternativas):

| Tipo | Nombre / Host | Valor | TTL |
|---|---|---|---|
| A | `@` (raíz) | `185.199.108.153` | por defecto |
| A | `@` | `185.199.109.153` | por defecto |
| A | `@` | `185.199.110.153` | por defecto |
| A | `@` | `185.199.111.153` | por defecto |
| AAAA | `@` | `2606:50c0:8000::153` | por defecto |
| AAAA | `@` | `2606:50c0:8001::153` | por defecto |
| AAAA | `@` | `2606:50c0:8002::153` | por defecto |
| AAAA | `@` | `2606:50c0:8003::153` | por defecto |
| CNAME | `www` | `askenaz-dev.github.io.` | por defecto |

Las AAAA son opcionales en el sentido de que el sitio funciona sin ellas, y
obligatorias en el sentido de que sin ellas una red IPv6 pura no lo alcanza.

En Dynadot esto es **Mis dominios → meltemi.dev → DNS** con el modo de registros
DNS (no el reenvío ni el estacionamiento: el reenvío rompe la verificación de
Pages y el certificado). Cualquier registro `A`/`AAAA`/`CNAME` previo en la raíz
—el estacionamiento del registrador suele dejar uno— hay que borrarlo: un apex
con dos destinos falla la mitad de las veces, que es peor que fallar siempre.

**Comprobación**, en cuanto propague (minutos a unas horas):

```bash
nslookup meltemi.dev
curl -sI https://meltemi.dev | head -3
```

Después, en **Settings → Pages** del repositorio, activar **Enforce HTTPS**. No
aparece hasta que GitHub emite el certificado de Let's Encrypt, lo que ocurre
solo cuando el DNS ya resuelve: si el botón está gris, el DNS todavía no llegó,
no es un fallo del repositorio.

Mientras el DNS no apunte, el sitio sigue sirviéndose en
`https://askenaz-dev.github.io/meltemi/`.

## 2. Firma de la primera release

Procedimiento completo en [`docs/release.md`](../release.md) («Signing a
release»). Resumen del orden: generar la clave minisign una vez, firmar el
`SHA256SUMS` del draft, verificar la firma uno mismo, subir el `.minisig`,
pegar la clave pública en las notas y publicar el draft.

La clave privada no entra en este repositorio ni en CI, nunca.

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

Consecuencia operativa que hay que tener presente: **firmar antes de publicar es
obligatorio**. Si publicas primero, esa versión ya no admite el `.minisig` y el
único remedio es cortar otra. `scripts/sign-release.ps1` lo comprueba antes de
pedirte la passphrase.

## 5. Deuda de firma de instaladores

El MSI y el DMG salen sin firma de plataforma: Authenticode y la notarización de
Apple exigen certificados comprados. Hasta entonces el instalador avisa en el
primer arranque. Está declarado en `docs/plan-de-cambios.md` y en el sitio; no se
disimula.
