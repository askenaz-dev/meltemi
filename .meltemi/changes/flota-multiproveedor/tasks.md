## 1. Resolución por sesión

- [ ] 1.1 Función de resolución en la flota: nombre → perfil → id del catálogo → agente configurado; id no detectado rehúsa con 2001; resultado con fuente `profile|catalog|configured` _(Req: Resolución de agente por sesión; design D1)_
- [ ] 1.2 Cablear la resolución en los lanzamientos que nombran agente (`sdd/implement`, despacho 3.x) + evento de resolución (binario efectivo y fuente) en el JSONL _(Req: Resolución — registro)_

## 2. Perfiles de lanzamiento

- [ ] 2.1 Config `[[fleet.profile]]` (name/agent/env) con referencias `${VAR}` resueltas al lanzar (reuso `resolve_ref`) y lint de higiene que rehúsa secretos en claro (reuso `looks_like_plaintext_secret`) _(Req: Perfiles ciegos a credenciales; design D2)_
- [ ] 2.2 Sobrecapa de entorno aplicada al subproceso del binario oficial en el lanzamiento ACP _(Req: Perfiles — contexto de autenticación)_
- [ ] 2.3 `fleet/list` incluye perfiles (fuente, agente subyacente, detección del binario subyacente) + render en CLI `fleet` _(Req: Perfiles visibles en el catálogo; design D4)_

## 3. Despacho (primitivo de carrera)

- [ ] 3.1 `proto/`: método `worktree/dispatch` + tipos + schema (aditivos) _(design D3)_
- [ ] 3.2 Handler: resolver binario (1.1) → worktree de la asignación + checkpoint → turno bajo reglas → commit con trailers; jamás tick de `tasks.md` _(Req: Despacho de competidores)_
- [ ] 3.3 CLI `dispatch <change> <task> <agent|profile>` _(paridad §4)_

## 4. Tests y calidad

- [ ] 4.1 Unit: orden de resolución (perfil > catálogo > configurado), rehúso 2001 no detectado, higiene de perfiles (secreto en claro rehusado, `${VAR}` aceptada)
- [ ] 4.2 E2e sobre fixture git: dos agentes de flota declarados apuntando al mock con perfiles distinguibles → dos despachos en paralelo sobre la misma tarea → cada worktree evidencia SU binario/contexto; diffs comparables; `tasks.md` intacto; evento de resolución en el log _(constitución: fixtures temporales, mock, sin red)_
- [ ] 4.3 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes en las tres plataformas
