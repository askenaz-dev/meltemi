## 1. Atestación en el pipeline

- [x] 1.1 Añadir `id-token: write`, `attestations: write` y `artifact-metadata: write` al job `release`, y el paso `actions/attest` con `subject-checksums` sobre el `SHA256SUMS` fusionado, pineado por SHA (§10) tras verificar el tag vigente
- [ ] 1.2 Comprobar en una corrida real que el repositorio permite esos permisos en un job disparado por tag, y que la atestación no añade assets al conjunto publicado (presupuestos de tamaño intactos)

## 2. Custodia y ancla de confianza

- [ ] 2.1 Enmendar el requisito de custodia en la documentación: almacenamiento offline (no hardware-backed), y repudio definido como clave nueva en el repositorio más declaración fechada
- [ ] 2.2 Publicar la clave pública en `docs/release.md` y enlazarla desde el sitio y los dos readmes, cuando el mantenedor la entregue

## 3. Verificación publicada

- [ ] 3.1 Documentar `gh attestation verify` con `--signer-workflow`, diciendo qué atestigua el job que la emite y qué no
- [ ] 3.2 Declarar la nota de transparencia (§9) y la asimetría de verificación offline entre minisign y la atestación
- [ ] 3.3 Reflejar los tres pasos —checksum, firma, procedencia— en `README.md`, `LEEME.md` y las dos páginas de descargas, ordenados por lo que cada uno compra

## 4. Cobertura

- [ ] 4.1 Tests por escenario: procedencia publicada, alcance declarado sin exagerar, registro público declarado, ancla fuera de la página que autentica, límites de la herramienta declarados
