## Context

La v0.1.0 se firmó a mano: el pipeline deja un draft, el mantenedor baja el
`SHA256SUMS`, lo firma con minisign en su máquina, verifica, sube el `.minisig` y
publica. Eso funciona y su propiedad es real —CI jamás sostiene la clave—, pero
deja sin responder una tercera pregunta que ni el checksum ni la firma cubren:
*¿de qué código y qué proceso salieron estos bytes?*

El estado actual, verificado en el árbol: el job `release` declara `contents:
write` y nada más; `id-token: write` aparece solo en `publish-site`, para Pages.
No hay atestación, ni cosign, ni sigstore en ninguna parte del repositorio.

## Goals / Non-Goals

**Goals:** que cada release lleve procedencia verificable del workflow que la
construyó, sin mover la clave de firma ni un centímetro; que la doc diga con
precisión qué atestigua la atestación y qué no; que el requisito de custodia deje
de prometer capacidades que la herramienta elegida no tiene.
**Non-Goals:** cualquier forma de firma en CI; migrar de minisign; atestiguar
artefacto por artefacto en su job constructor; firma de plataforma.

## Decisions

### D1 — Los dos mecanismos conviven; ninguno reemplaza al otro

La atestación responde «este artefacto salió del workflow X, del commit Y». La
firma minisign responde «el mantenedor lo revisó y lo avaló». Son afirmaciones
independientes y ninguna implica la otra, así que quedan las dos.

El argumento que cierra la puerta a reemplazar la firma por keyless: **una cuenta
de GitHub comprometida puede producir una atestación que verifica perfectamente.**
El atacante empuja un tag, el pipeline construye desde su commit, y la atestación
registra fielmente ese commit —hace exactamente su trabajo. Lo único que el
atacante no controla es una clave que vive fuera de GitHub, en una máquina con
passphrase. Perder eso a cambio de comodidad sería cambiar la garantía más fuerte
del esquema por la más automatizable.

La dirección inversa también vale: la firma sola no dice nada del build. Un
mantenedor puede firmar de buena fe un artefacto construido desde un árbol
contaminado. Ahí la atestación es la que habla.

### D2 — Un solo subject: el `SHA256SUMS`, no cada artefacto

El paso va en el job `release`, sobre el `SHA256SUMS` fusionado, con
`subject-checksums` —que acepta el formato shasum que `sha256sum *` ya produce.

Esto tiene una honestidad incómoda que se declara en vez de disimularse: el job
`release` **solo descarga** artefactos de los seis jobs de empaquetado; no
compila nada. Una atestación acuñada ahí atestigua la fusión, no la construcción.
Atestiguar a los constructores reales serían seis pasos, cada uno con su bloque
de permisos, complicados por el renombrado normalizador que cambia los nombres de
subject antes de publicarlos.

Se empieza por el `SHA256SUMS` porque cubre exactamente el mismo alcance que la
firma minisign —un archivo que cubre a todos los demás—, y porque una afirmación
modesta y cierta vale más que una fuerte y matizada. La doc dice cuál se hizo.

### D3 — El ancla de confianza vive en el repositorio

La clave pública no puede venir de la página de release: quien puede publicar una
release puede editar el texto que la acompaña, así que una clave impresa solo ahí
no prueba nada —el atacante reemplaza artefactos, firma con su clave, pega su
clave al lado, y toda instrucción publicada pasa. En el árbol, reemplazarla es un
diff con autor y fecha, en un archivo que miles de clones ya tienen.

Esto no es teórico en este repositorio: la doc decía literalmente «the public key
printed on every release page», y la clave no existía en ninguna parte del árbol.

### D4 — La custodia declara los límites de su herramienta

minisign no tiene soporte de HSM ni PKCS#11, así que «hardware-backed» no es
alcanzable sin cambiar de herramienta; y no tiene mecanismo de revocación, así que
no existe mensaje que haga que una firma vieja deje de verificar. El requisito se
enmienda para prometer almacenamiento offline y definir «repudiar» como publicar
clave nueva en el repositorio y declarar la vieja retirada desde una fecha. Eso
funciona precisamente porque D3 puso el ancla donde el reemplazo es auditable.

### D5 — La nota de transparencia se escribe antes de que la descubran

En un repositorio público la atestación va a un log de transparencia público, con
el digest del artefacto y la identidad del workflow. Es metadato de build y no
dato de usuario, así que no roza §9 —pero es público y permanente, y un proyecto
que promete «sin telemetría oculta» lo dice él mismo.

## Risks / Trade-offs

- **Dependencia de servicio externo en la ruta de release.** Un paso de
  atestación que falle aborta el release. Hoy el pipeline no necesita ningún
  servicio fuera de GitHub Actions. Es simétrico al costo de la clave local —que
  necesita el portátil del mantenedor— y por eso se acepta, pero es real.
- **Verificación offline asimétrica.** minisign verifica sin red. `gh attestation
  verify` consulta la API por defecto, y su modo offline exige material
  descargado antes desde una máquina en línea. Para un producto local-first la
  doc debe decirlo, no callarlo.
- **Un prerrequisito más para el usuario.** `gh` no viene con ningún sistema
  operativo. Se suma a `minisign`. La verificación completa ya cuesta dos
  herramientas que hay que instalar; la doc debe ordenar los pasos por lo que
  cada uno compra, para que quien solo quiera el checksum sepa dónde parar.
- **§6 favorece la atestación, no a minisign.** El predicado es
  `slsa.dev/provenance/v1`, in-toto bajo especificación de la Linux Foundation, y
  Sigstore es proyecto graduated de OpenSSF. minisign está documentado pero no es
  un estándar: conservarlo es una desviación consciente, justificada por
  ergonomía y por la existencia de `rust-minisign`, que abriría la puerta a un
  `meltemi verify` sin binarios externos. Queda escrito, no implícito.
- **Qué invalidaría esta decisión:** que GitHub o Sigstore ganaran una forma de
  atestación que un tag empujado por una cuenta comprometida no pudiera producir
  —por ejemplo exigiendo una aprobación humana fuera de banda—. Entonces la firma
  manual pasaría a ser redundante y esta change habría sido el paso correcto de
  todos modos.

## Sin verificar al escribir este design

No se convierten en afirmación y se comprueban al implementar: si el repositorio
o la organización permiten efectivamente `id-token: write` y `attestations:
write` en un job disparado por tag (que `publish-site` ya pida `id-token` para
Pages es evidencia, no prueba); la política de retención de las atestaciones, que
**no** debe confundirse con los 90 días de los artifacts de Actions; el SHA exacto
de `actions/attest` a pinear, porque la versión vigente se publicó días antes de
escribir esto; y si la atestación añade assets al conjunto publicado y por tanto
a los presupuestos de tamaño (se espera que no).
