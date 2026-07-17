## 1. Orquestador de tareas

- [ ] 1.1 Bucle de despliegue: elegibilidad por dependencias, ciclo compuesto por tarea (checkpoint→turno→verificación→commit→tick), progreso persistente reanudable _(Req: Despliegue de agentes)_
- [ ] 1.2 Modo planificar/actuar por change con override por tarea; gate de plan con árbol intacto _(Req: Modo planificar o actuar)_
- [ ] 1.3 Degradación autónomo→supervisado sin reglas, con aviso _(Req: Autonomía solo dentro de guardarraíles)_

## 2. Progreso e interrupción

- [ ] 2.1 Eventos de progreso por tarea; vista de Sesión muestra actual/completadas/restantes _(Req: Progreso vivo)_
- [ ] 2.2 Interrupción entre tareas (estado consistente) y a mitad de tarea (cancela solo esa; worktree inspeccionable) _(Req: Interrupción segura)_

## 3. Superficies

- [ ] 3.1 `sdd/implement` en proto; verbo operativo en TUI (Proyecto) y CLI (`--json` de progreso); gramática final sin reservados _(Modified: cli-contract)_

## 4. Tests y calidad

- [ ] 4.1 E2e de composición sobre fixture: change con 3 tareas (una con solapamiento) → despliegue completo con mock-agent; reinicio a medias reanuda; planificar bloquea hasta gate; sin reglas degrada con aviso; interrupciones ambas
- [ ] 4.2 `cargo clippy -- -D warnings`, `fmt --check` y tests verdes
