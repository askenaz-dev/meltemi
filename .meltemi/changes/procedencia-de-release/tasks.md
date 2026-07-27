## 1. Atestación en el pipeline

- [x] 1.1 Añadir `id-token: write`, `attestations: write` y `artifact-metadata: write` al job `release`, y el paso `actions/attest` con `subject-checksums` sobre el `SHA256SUMS` fusionado, pineado por SHA (§10) tras verificar el tag vigente
- [ ] 1.2 Comprobar en una corrida real que el repositorio permite esos permisos en un job disparado por tag, y que la atestación no añade assets al conjunto publicado (presupuestos de tamaño intactos)
  > Verificado sin tag (2026-07-27): `default_workflow_permissions: write` y
  > sin política de organización que lo recorte (`gh api
  > repos/askenaz-dev/meltemi/actions/permissions/workflow`); repositorio
  > público, así que las atestaciones están disponibles en el plan actual;
  > GitHub registra el workflow `Release` como activo tras el paso nuevo
  > (`gh workflow list --all`) y el lint estructural del YAML pasa. Queda lo
  > que solo una corrida real puede probar: el paso verde en un job disparado
  > por tag y el conjunto de assets del draft sin adiciones. Se comprueba en
  > el próximo tag `vX.Y.Z`; hasta entonces la casilla queda abierta.

## 2. Custodia y ancla de confianza

- [x] 2.1 Enmendar el requisito de custodia en la documentación: almacenamiento offline (no hardware-backed), y repudio definido como clave nueva en el repositorio más declaración fechada
- [ ] 2.2 Publicar la clave pública en `docs/release.md` y enlazarla desde el sitio y los dos readmes, cuando el mantenedor la entregue
  > En espera de la clave (2026-07-27): la mitad pública no está en ninguna
  > parte del árbol y no se toma de ningún otro canal —la página de release
  > es justo el origen que D3 descarta—. Cuando el mantenedor entregue la
  > línea de `meltemi.pub` (su copia local, `~/keys/meltemi/meltemi.pub`),
  > se pega en la sección «The public key» de `docs/release.md`
  > reemplazando el aviso «Not yet published», y los enlaces del sitio y de
  > los dos readmes ya apuntan a ese archivo como ancla.

## 3. Verificación publicada

- [ ] 3.1 Documentar `gh attestation verify` con `--signer-workflow`, diciendo qué atestigua el job que la emite y qué no
- [ ] 3.2 Declarar la nota de transparencia (§9) y la asimetría de verificación offline entre minisign y la atestación
- [ ] 3.3 Reflejar los tres pasos —checksum, firma, procedencia— en `README.md`, `LEEME.md` y las dos páginas de descargas, ordenados por lo que cada uno compra

## 4. Cobertura

- [ ] 4.1 Tests por escenario: procedencia publicada, alcance declarado sin exagerar, registro público declarado, ancla fuera de la página que autentica, límites de la herramienta declarados
