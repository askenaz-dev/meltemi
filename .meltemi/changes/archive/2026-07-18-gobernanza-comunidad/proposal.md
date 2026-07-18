## Why

Antes de que el repositorio sea público, la comunidad necesita reglas escritas:
cómo se gobierna el proyecto, cómo se contribuye (¡vía specs!), qué conducta se
espera, cómo se reportan vulnerabilidades y qué firma un contribuidor. Sin esto,
el primer PR externo llega sin cauce y el "de la comunidad" del lema queda en
eslogan. (§9.3: CLA acotado; la opción copyleft reservada solo a componentes de
servidor futuros.)

## What Changes

- **GOVERNANCE.md**: modelo de gobernanza (mantenedor fundador hoy; camino
  declarado hacia más mantenedores), cómo se ratifican enmiendas fundacionales
  (formaliza la práctica de `method-bootstrap`).
- **CONTRIBUTING.md**: la contribución es spec-driven — toda feature entra como
  propuesta de cambio; guía del ciclo, estilo de artefactos (ES/EN), calidad
  (clippy/fmt/tests 3 plataformas), SPDX y convención de commits (sin trailers
  de co-autoría).
- **CODE_OF_CONDUCT.md** (estándar adoptado, adaptado) y **SECURITY.md**
  (divulgación responsable, alcance, tiempos de respuesta).
- **Texto del CLA acotado** (§9.3) + plantillas `.github/` (issue de propuesta
  de change, PR con checklist de spec).
- El **mecanismo de firma del CLA** lo decide el mantenedor (pendiente
  registrado en el plan); esta change deja el texto y el hueco de tooling.

## Capabilities

### New Capabilities
- `community-governance`: spec de gobernanza que fija qué documentos existen y
  qué garantizan (como `method-bootstrap`, gobierna proceso, no código).

### Modified Capabilities
- _Ninguna._

## Impact

- Solo documentos en la raíz y `.github/`; cero código. Español e inglés según
  audiencia (documentos comunitarios en inglés con resumen en español, decisión
  fina en el design).

## Fuera de alcance

- Programa de seguridad pagado/bug bounty; fundación u organización legal
  (decisión del mantenedor, fase futura).
- Tooling de firma del CLA (se integra cuando el mantenedor lo elija).
