# sucesion-y-custodia

> Vía rápida (fast-forward): documentación de gobernanza sobre lo que
> `procedencia-de-release` ya decidió. Sin código. Nace del comparativo con
> Orca del 2026-09-24 (`docs/research/comparativo-orca-2026-09-24.md`,
> brecha P10). Después de que `procedencia-de-release` cierre.

## Why

`docs/release.md` ya dice cómo se firma, cómo se rota y cómo se repudia una
clave (`docs/release.md:221-237`), y por qué la firma es manual y fuera de
CI. Lo que no dice es **qué claves existen, dónde viven, quién las tiene y
qué pasa si el mantenedor no está**. Un proyecto con un solo firmante y
una promesa «Apache 2.0, para siempre» (constitución §12) necesita escribir
su sucesión antes de que alguien la pregunte en un issue.

## What Changes

- **Inventario de claves** en `docs/release.md`: la clave minisign de
  releases (huella, fecha, dónde está su ancla de confianza en el árbol),
  las claves de firma de plataforma cuando existan, y la regla «ninguna
  clave en secretos de Actions», ya vigente, repetida donde se lee.
- **Sucesión** en `GOVERNANCE.md`: un segundo custodio designado por el
  mantenedor (persona o mecanismo de umbral cuando minisign lo permita por
  vía de claves múltiples), el procedimiento para que publique una release
  firmada si el mantenedor no puede, y cómo se anuncia públicamente un
  cambio de custodio (nueva clave, repudio de la anterior).
- **Plan de pérdida**: qué se publica y dónde si una clave se pierde o se
  compromete, con el texto listo para copiar.
- Las decisiones —quién es el segundo custodio— son del mantenedor y se
  listan en «Pendientes exclusivos del arquitecto».

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `release-distribution`: + requisito «Inventario de claves y sucesión
  documentados».
- `community-governance`: + la sección de custodia y sucesión.

## Impact

- Archivos: `docs/release.md`, `GOVERNANCE.md`, `SECURITY.md` (enlace),
  `docs/plan-de-cambios.md` (pendientes del arquitecto).
- Cero código; cero dependencias.

## Fuera de alcance

- Mover la firma a CI o a un HSM: decidido en contra por
  `procedencia-de-release` (SLSA v1.2), no se relitiga.
