# Design — panel-opaco-y-nav-plegable

## Context

Verificado en el código el 2026-08-08, a partir de la captura del mantenedor:

- `ProjectSwitcher.svelte:110` declara `background: var(--surface-1)`. Los
  tokens de `app.css` son `--surface` (#ffffff / #111a2e) y `--surface-2`;
  **`--surface-1` no existe**. El panel queda sin fondo y todo lo que cubre se
  lee a través — el árbol de proyectos, la tabla de Flota y las píldoras de
  estado, superpuestos, como en la captura.
- Un barrido de `desktop/ui/src` (todo `var(--x)` usado contra todo `--x`
  definido) encuentra **exactamente uno** fantasma: ese. No hay una familia de
  fallos que arreglar, hay un caso y una clase que cerrar.
- El z-index del panel (41) y su scrim (40) son correctos: el problema nunca
  fue el apilamiento, era la ausencia de pintura.
- `Sidebar.svelte` es un `<aside>` de `width: 216px` sin control de pliegue.
  El `collapsed: Set<String>` que existe cierra grupos de proyecto dentro del
  árbol, no la barra.
- La GUI ya persiste tema y geometría de ventana (`gui-shell`: «Tema y estado
  de ventana persistentes»), de modo que hay dónde recordar un pliegue sin
  inventar mecanismo.

## Goals / Non-Goals

**Goals**: que ninguna superficie flotante pueda quedar sin fondo otra vez;
que la barra lateral se pliegue y se recuerde, sin perder alcance ni
accesibilidad.

**Non-Goals**: rediseñar la navegación; tocar el contrato de vistas numeradas;
la auditoría de intuitividad; un smoke visual por commit.

## Decisions

### D1 — El token correcto, y un lint que impide el siguiente

El arreglo es `--surface-1` → `--surface`: el panel usa el token que el design
system define para superficies elevadas, el mismo que usan el drawer y los
diálogos. Alternativa descartada: definir `--surface-1` como alias — añadiría
un token equivalente a otro existente y multiplicaría el vocabulario que ya
confunde.

Lo que importa más que la línea es que nada la detuvo. Se añade un lint de
tokens al conjunto de tests de escritorio: recoge cada `var(--x)` usado en
`desktop/ui/src` y cada `--x` definido, y falla nombrando archivo y línea de
cualquiera que se use sin existir. Es una función corta y captura la clase
entera — un `var()` mal escrito no vuelve a llegar a la ventana. No sustituye
al smoke visual (un token válido con el valor equivocado sigue necesitando
ojos), y el design lo dice para que nadie confunde su alcance.

### D2 — Plegar a riel, no ocultar

Tres opciones: (a) ocultar la barra por completo; (b) **plegarla a un riel
angosto de iconos** — la elegida; (c) un panel superpuesto que aparece al
pasar el puntero. (a) deja al usuario sin forma visible de volver salvo un
atajo que debe recordar; (c) es exactamente lo que la spec de la GUI ya
rechaza en otro control («un control que aparece bajo el puntero es un control
que el teclado no alcanza»). Con (b) las entradas siguen presentes, clicables
y tabulables; lo que se retira es el texto, no el alcance: cada entrada
conserva su `aria-label` y su dígito. El contador de permisos permanece
visible en el riel — es la señal que la spec obliga a no esconder.

### D3 — El pliegue se recuerda donde ya se recuerda el tema

El estado va al mismo almacén de preferencias de ventana que ya guarda tema y
geometría (`desktop-ui.json`), no a un mecanismo nuevo: una preferencia de
disposición pertenece con las otras. Un perfil nuevo arranca desplegado —
el estado por defecto es el que enseña la navegación completa, que es lo que
el onboarding promete.

## Risks / Trade-offs

- **El lint es sintáctico**: ve `var(--x)` en fuentes de `desktop/ui/src`; un
  token compuesto en tiempo de ejecución se le escapa. Hoy no existe ninguno,
  y si apareciera, el smoke visual sigue siendo la red.
- **El riel angosto reduce el objetivo de clic**: se mantiene el tamaño de
  control del design system (los iconos no encogen), y el ancho del riel se
  fija a partir de ese mínimo, no al revés.

## Migration Plan

Aditivo y reversible: una propiedad CSS corregida, un test nuevo, un control y
una preferencia. Un perfil existente sin la preferencia guardada arranca
desplegado, como hoy.

## Open Questions

- ¿Atajo de teclado para plegar? Se omite deliberadamente: el conjunto de
  teclas de la GUI es materia de su propia revisión y añadir uno aquí, sin esa
  vista de conjunto, es cómo se llega a un teclado incoherente.
