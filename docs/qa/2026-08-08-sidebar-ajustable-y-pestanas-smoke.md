# Smoke conducido — `sidebar-ajustable-y-pestanas`

**Fecha**: 2026-08-08 · **Plataforma**: Windows 11, WebView2 (Chromium 151)
**Binario**: `target/release/meltemi-desktop.exe` construido con
`tauri build --no-bundle`, sobre repositorios fixture temporales y `mock-agent`.
Nunca contra este repositorio, nunca contra la red.

## Método

Puerto de depuración remoto **temporal** en `tauri.conf.json`, revertido y el
binario reconstruido limpio al terminar (`grep remote-debugging` → 0). El
conductor CDP sintetiza eventos de puntero y de teclado reales, lee estilos
computados y geometría, y fotografía. Cinco proyectos registrados y cinco
sesiones de `mock-agent` en un directorio de datos aislado.

## Resultado

Todas las comprobaciones pasan sobre el binario empaquetado.

| Escenario | Medido |
| --- | --- |
| Arrastrar la línea reparte el alto | navegación 240→150 px, árbol 347→437 px |
| Ninguna entrada se pierde al encoger | 7/7 entradas con etiqueta accesible, la navegación desplaza |
| El reparto se ajusta con el teclado | 150→182 px con dos ArrowDown; Home → 64 px (su mínimo) |
| El separador declara lo que es | `role=separator`, `tabindex=0`, `aria-controls=nav-entries`, valor y límites |
| El árbol desplaza sin comerse la columna | 10 px reservados (la clásica reserva ~17) y **sin botones de paso** |
| Abrir una segunda sesión no reemplaza la primera | 5 pestañas, 1 seleccionada |
| `tabindex` rotatorio | exactamente 1 pestaña en el orden de tabulación |
| La lista es la primera pestaña y nunca se cierra | sin control de cierre |
| Los paneles de fondo se ocultan, no se desmontan | 4 de 5 con `hidden` |

## Lo que el smoke encontró y esta change corrigió

1. **`scrollbar-width` no hereda.** El design afirmaba que ambas propiedades
   heredaban y las puso en `:root`. El valor computado del árbol lo desmintió:
   `scrollbar-color` llegó desde la raíz y `scrollbar-width` computó `auto`, con
   la barra clásica intacta. La regla se veía correcta y no hacía nada.
2. **La barra angosta conserva sus botones de flecha en WebView2**, que es la
   parte más fea de lo que el mantenedor señaló, y **ninguna propiedad estándar
   los quita**.
3. **Las dos familias son excluyentes.** Neutralizando las propiedades estándar
   sobre el árbol, las reglas `::-webkit-scrollbar` pasaron a aplicarse y los
   botones desaparecieron: en Chromium, poner `scrollbar-width` **o**
   `scrollbar-color` desactiva por completo la familia legada. De ahí las dos
   ramas `@supports` — cada motor recibe una respuesta completa y nunca pueden
   pelearse.
4. **El anillo de foco del separador parecía un campo de texto vacío**: dibujado
   alrededor de un área de agarre de 12 px de alto y todo el ancho. Se metió
   dentro (`outline-offset: -5px`) para que abrace la línea.

Ninguno de los cuatro era visible desde el código fuente.

## Consumo

Heap de la superficie con cinco pestañas abiertas y sus transcripts montados:
**5–8 MB**. Muy por debajo del presupuesto en reposo. La medida es del heap de
JavaScript, no del RSS del proceso: el tope de ocho pestañas con transcripts
largos sigue sin medirse contra el presupuesto real, y el tope de líneas del
transcript sigue declarado fuera de alcance.

## Lo que no se pudo confirmar visualmente

- **El borrador sin enviar que sobrevive a un viaje de ida y vuelta.** Las cinco
  sesiones del fixture están finalizadas y no reanudables, así que su panel no
  presenta compositor y no hay borrador que escribir. La lectura tampoco tiene
  desplazamiento que conservar por la misma razón. Ambos quedan cubiertos por el
  test de cableado, no por el ojo. Un fixture con una sesión viva —o con el
  agente simulado en espera— lo cerraría.
