## Why

Firmar la v0.1.0 a mano dejó una pregunta legítima del mantenedor: las
organizaciones grandes no firman desde un portátil. Es cierto. Lo que hacen es
sacar la clave de las manos de las personas —HSM o KMS, clave no exportable, el
pipeline *pide* firmas por API— o eliminar la clave de largo plazo con firma
keyless atada a la identidad del workflow. El humano aparece en la custodia de la
clave, no en cada release: la KSK de la raíz del DNS se activa con 3 de 7 tarjetas
en un HSM desconectado de toda red, unas cuatro veces al año, y lo que firma son
otras claves. Fedora, el análogo más cercano a este proyecto, corre `sigul` en un
servidor que no acepta conexiones entrantes y firma cada artefacto de forma
automática. **La ceremonia firma claves; la máquina firma artefactos.**

La conclusión, sin embargo, no es mover la clave a CI. Hay tres modelos que se
confunden constantemente: material de clave dentro de un secreto (el pipeline *es*
la identidad), HSM/KMS (el pipeline pide firmas y la clave no es exportable), y
keyless (no hay clave de largo plazo). Solo el primero es indefendible, y lo es en
lenguaje normativo: SLSA v1.2 §«Provenance is Unforgeable» exige que el material
secreto «MUST NOT be accessible to the environment running the user-defined build
steps». Una `meltemi.key` en un Actions secret es exactamente ese caso, y GitHub
lo confirma desde su lado: cualquiera con write lee todos los secretos, y una
sola action comprometida los alcanza todos —este pipeline invoca nueve, dos de
terceros, todas pineadas a tag mayor flotante.

Y hay un argumento que decide contra reemplazar la firma manual por keyless: **es
el único paso que una cuenta de GitHub comprometida no puede completar.** Un
atacante con write empuja un tag, CI construye, atestigua, y la atestación
verifica perfectamente —porque registra fielmente el commit del atacante. El gate
del draft más una clave en una máquina que no es GitHub es lo que corta eso.

Lo que falta, entonces, no es cambiar la firma: es que nada en la release diga
*qué proceso construyó estos bytes*. El checksum dice que llegó intacto; la firma
dice quién lo avaló; ninguna dice de qué commit y qué workflow salió.

## What Changes

- **Atestación de build sobre el `SHA256SUMS` publicado.** El job `release` gana
  `id-token: write`, `attestations: write` y `artifact-metadata: write` —el
  tercero es más nuevo que casi cualquier receta publicada— y un paso de
  `actions/attest` con `subject-checksums` apuntando al `SHA256SUMS` fusionado,
  que ya está en formato shasum. Una atestación cubre exactamente lo mismo que la
  firma: un archivo que cubre a todos los demás.
- **Honestidad sobre qué atestigua.** El job `release` solo descarga artefactos;
  no los construye. Una atestación acuñada ahí atestigua el merge, no los seis
  jobs de empaquetado. Se documenta esa frontera en vez de insinuar la afirmación
  fuerte; atestiguar a los constructores reales es promoción futura con evidencia.
- **La verificación publicada gana su segundo comando**: `gh attestation verify
  <archivo> --repo askenaz-dev/meltemi`, con `--signer-workflow` para fijar el
  workflow. Eso es lo que minisign no puede: fijar *qué proceso* construyó el
  artefacto. El costo es `gh` como prerrequisito, y se dice.
- **El requisito de custodia se enmienda para que sus promesas sean cumplibles**:
  la clave pública vive en el repositorio y no en la página de release, el
  almacenamiento es offline (no hardware-backed: minisign no tiene HSM ni
  PKCS#11), y «revocar» queda definido como publicar clave nueva y repudiar la
  vieja, porque minisign no tiene mecanismo de revocación alguno.
- **Nota de §9 escrita, no descubierta**: en un repositorio público la atestación
  va a un log de transparencia público con el digest del artefacto y la identidad
  del workflow. Es metadato de build, no dato de usuario, pero es público y
  permanente, y este proyecto lo dice antes de que lo note un lector.

## Capabilities

### Modified Capabilities
- `release-distribution`: + requisito de procedencia verificable de la release;
  el requisito de artefactos firmados enmendado para que el ancla de confianza
  viva en el repositorio y para declarar los límites reales de la herramienta de
  firma.

## Impact

- `.github/workflows/release.yml` (permisos del job `release`; paso nuevo entre
  la fusión de checksums y la creación del draft), `docs/release.md` (sección de
  verificación de procedencia y la nota de transparencia), `README.md`,
  `LEEME.md`, `site/downloads.html`, `site/es/downloads.html` (el lint del sitio
  falla si divergen), `core/meltemid/tests/release.rs`.
- Modo de fallo nuevo en la ruta crítica: un paso de atestación que falle —OIDC,
  permisos, indisponibilidad del servicio— es una forma nueva de abortar un
  release en un pipeline que por lo demás no necesita servicios externos. Es
  simétrico al costo de la clave local, que necesita el portátil del mantenedor,
  y se asume con los ojos abiertos.
- Asimetría de verificación offline que merece una frase en la doc: minisign
  verifica sin red con la clave y el `.minisig`; `gh attestation verify` consulta
  la API de GitHub por defecto, y su ruta offline exige descargas previas desde
  una máquina en línea. Para un producto local-first eso no es un detalle menor.

## Fuera de alcance

- **Mover la clave de firma a CI**, en cualquiera de sus formas. El secreto está
  prohibido por SLSA; HSM/KMS exigiría migrar de herramienta, una cuenta cloud y
  una credencial dentro de CI, sin comprar nada que la atestación no dé gratis.
- **Reemplazar minisign por firma keyless**: perdería la propiedad de que un tag
  empujado por una cuenta comprometida no basta para publicar.
- **Atestiguar cada artefacto en su job constructor**: seis pasos más, cada uno
  con su bloque de permisos, y el renombrado normalizador complica los nombres de
  subject. Se promueve con evidencia, no de entrada.
- **Firma de plataforma** (Authenticode, notarización de Apple): otro problema y
  otra respuesta —dinero, no criptografía—, con su deuda ya declarada.
- **Activar immutable releases**: es un ajuste de repositorio, no código, y su
  disponibilidad en el plan actual está sin verificar. Se anota en el checklist
  de lanzamiento, no aquí.
- **Publicar la clave pública**: es reparación de algo ya prometido y no espera
  detrás de esta propuesta.
