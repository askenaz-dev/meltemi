<!-- SPDX-License-Identifier: Apache-2.0 -->
# Checklist de lanzamiento (mantenedor)

Los registros y activos que no vive ningún agente: dominio, firma, namespaces.
Cada línea dice qué hacer, con qué comando se comprueba y qué queda pendiente.
Los procedimientos largos viven en `docs/release.md`; aquí está el orden.

## 1. Dominio `meltemi.dev` → GitHub Pages

El único declarante que cuenta es **el ajuste de Pages del repositorio**. Existe
un `site/CNAME`, pero al publicar desde un workflow propio de Actions GitHub lo
ignora — está en su documentación: si publicas desde un workflow personalizado,
cualquier archivo CNAME se ignora y no es necesario. Se conserva por si la
publicación volviera a una rama, y su lint solo comprueba que no esté vacío. Quien
depure esto empezando por ese archivo perderá el tiempo: el dominio se cambia en
Settings → Pages.

**Registros en el panel DNS del dominio.** La raíz de una zona no admite `CNAME`
(RFC 1034 §3.6.2: un CNAME no puede convivir con otros datos, y la raíz lleva SOA
y NS por definición). Pero sí admite un **ANAME**, que Dynadot ofrece en su
sección *Domain Record* y GitHub documenta al mismo nivel que los A: el
autoritativo resuelve el destino por ti y responde direcciones. Esa es la fila
preferida, porque delega en GitHub el mantenimiento de sus propias IPs:

| Tipo | Nombre / Host | Valor |
|---|---|---|
| **ANAME** | `@` (raíz) | `askenaz-dev.github.io` |
| CNAME | `www` | `askenaz-dev.github.io.` |

El ANAME **sustituye** al CNAME o a los A de la raíz, no se suma: Dynadot rechaza
combinarlo con `A`, `AAAA`, `Forward` o `Stealth Forward`, y GitHub avisa aparte
de que registros extra en `@` pueden impedir que se genere el certificado.

Verifica en el autoritativo que el ANAME devuelve **direcciones** y no un CNAME
— Dynadot no documenta el comportamiento en el cable, y ANAME no es un tipo del
DNS estándar sino una síntesis del servidor:

```bash
nslookup -type=a meltemi.dev ns1.dyna-ns.net
nslookup -type=aaaa meltemi.dev ns1.dyna-ns.net
```

Si sigue apareciendo una línea `Aliases:` o un CNAME, el ANAME de Dynadot es un
CNAME rebautizado y hay que caer a los registros por dirección de abajo.

**Fallback por dirección.** Los cuatro valores de cada familia son el balanceo, no
alternativas, y hay que poner los cuatro:

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

> **El CNAME va SOLO en `www`, jamás en la raíz.** Es el error que este dominio ya
> cometió una vez, y enganña porque *parece* funcionar: un CNAME en el apex
> resuelve, el sitio responde por HTTP, y todo se ve bien. Pero GitHub Pages exige
> A/AAAA en un dominio raíz — con un CNAME ahí **nunca pide el certificado**, y como
> `.dev` está en la lista HSTS preload el navegador se niega a usar HTTP, así que
> el sitio queda inalcanzable con un error de TLS que no menciona el DNS por
> ninguna parte. Además un CNAME en el apex viola el DNS (no puede coexistir con
> los SOA y NS que la raíz lleva obligatoriamente) y bloquearía el correo del
> dominio. Si el panel te deja poner un CNAME en `@`, el panel se equivoca.

**Diagnóstico rápido cuando el certificado no aparece.** No adivines ni esperes a
ciegas: pregúntale a GitHub qué ve.

```bash
gh api repos/askenaz-dev/meltemi/pages/health
```

Los campos que deciden son `is_a_record`, `has_cname_record`,
`should_be_a_record`, `is_pointed_to_github_pages_ip` y `caa_error`. Un apex sano
lee `is_a_record: true` y `is_pointed_to_github_pages_ip: true`. Para ver lo que
hay realmente en la zona, sin cachés de resolvers de por medio, consulta al
nameserver autoritativo:

```bash
nslookup -type=any meltemi.dev ns1.dyna-ns.net
```

Y `https_error: "peer_failed_verification"` no significa que falte el certificado:
significa que GitHub está presentando el de `*.github.io`, que no cubre tu
dominio — el síntoma de que el certificado del dominio nunca se emitió.

En Dynadot esto es **Mis dominios → meltemi.dev → DNS** con el modo de registros
DNS (no el reenvío ni el estacionamiento: el reenvío entrega tu TLS al
registrador —que emite un certificado propio— e impide que GitHub emita el suyo). Cualquier registro `A`/`AAAA`/`CNAME` previo en la raíz
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
