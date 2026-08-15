# Tareas — acceso-remoto-en-dos-vias

Vía rápida: gate único al final. Un commit atómico por tarea, con referencia
`(acceso-remoto-en-dos-vias N.M)` y sin trailers de co-autoría. Gates del repo
en cada tarea: `cargo clippy -- -D warnings`, `cargo fmt --check` y la suite
del crate tocado. **Ojo concurrencia**: hay otras sesiones con changes en
vuelo — commitear en cuanto cada tarea cierre, nunca dejar el árbol sucio
entre pasos.

## 1. El puente

- [ ] 1.1 Verbo `meltemi bridge` en el cliente: conecta el endpoint local
  (named pipe / socket Unix) y bombea stdio↔endpoint en ambas direcciones;
  sin TTY; rehúso inmediato con diagnóstico y remedio si el daemon no está;
  cierre ordenado cuando un extremo cierra; cero dependencias nuevas (design
  D1) — escenarios «Un canal remoto completo sobre el puente», «Sin daemon,
  el puente rehúsa sin colgarse» y «El cierre de un extremo cierra el puente»
  — gates: suite del crate + e2e contra daemon de fixture (en Windows ejercita
  el pipe real)
- [ ] 1.2 El remedio del rehúso de Windows en `meltemi tunnel` nombra el
  puente con el comando exacto; el test del rehúso se extiende a pinear el
  remedio nuevo sin aflojar el rehúso (design D1) — escenario «El puente en la
  plataforma sin reenvío estándar» — gates: suite del crate

## 2. El patrón

- [ ] 2.1 `docs/acceso-remoto.md`: sección del puente; el punto de encuentro
  en dos vías (todos marcan hacia afuera, matriz de cuadrantes, túnel inverso
  permanente y precauciones del bastión); la variante BYO-network con la tabla
  de licencias verificada; la frontera (infraestructura del usuario, jamás
  dependencia); y las notas de fase 3 (BYO-identity con certificados SSH,
  selector multi-máquina, aviso de espera) marcadas como design de
  `companero-movil` (design D2, D3, D4) — escenarios «Los cuatro cuadrantes
  usan el mismo camino», «La malla del usuario no es una dependencia de
  Meltemi» y «Lo de fase 3 está anotado y no prometido» — gates: lint de docs

## 3. Cierre

- [ ] 3.1 `meltemi validate acceso-remoto-en-dos-vias` limpio y `meltemi
  verify` con los siete escenarios enlazados (meta: cero marcas manuales);
  suite completa, clippy y fmt verdes; smoke manual del tramo
  `ssh → meltemi bridge → pipe` sobre el binario de release con sshd real de
  Windows, capturado en nota de QA (design D6: CI no tiene sshd y no se finge
  uno); entrada en `docs/plan-de-cambios.md`
