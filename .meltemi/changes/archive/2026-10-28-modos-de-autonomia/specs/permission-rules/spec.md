# permission-rules — delta

## ADDED Requirements

### Requirement: Modos de autonomía como posturas por sesión

Una sesión SHALL poder declarar un modo de autonomía, y el daemon SHALL
componerlo con las reglas del usuario en el punto único de evaluación, en este
orden y sin excepción:

1. Un `deny` del usuario SHALL prevalecer sobre cualquier modo.
2. Toda operación fuera del árbol o de efecto irreversible SHALL escalar a una
   persona **en todos los modos**, incluido el autónomo (constitución §3).
3. Lo demás SHALL resolverlo el modo: **manual** SHALL convertir en pregunta lo
   que las reglas concederían; **semi** SHALL conceder únicamente ediciones
   contenidas en el árbol de la sesión; **autónomo** SHALL conceder lo que haya
   sobrevivido a (1) y (2).

WHERE la contención de una edición no pueda afirmarse —ruta ausente o fuera del
árbol— la petición SHALL escalar.

Sin modo declarado, la resolución SHALL ser exactamente la de las reglas del
usuario, sin composición alguna.

NO SHALL existir modo alguno que omita el proxy de permisos, conceda opciones
que el agente no ofreció, o altere la denegación constitucional sin clientes.

#### Scenario: El deny del usuario sobrevive a cualquier modo

- **WHERE** una regla del usuario deniega una petición
- **WHEN** la sesión corre en modo autónomo
- **THEN** la petición SHALL denegarse igualmente

#### Scenario: Lo irreversible escala aunque el modo sea autónomo

- **WHEN** una petición ejecuta un comando o sale a la red en modo autónomo
- **THEN** SHALL escalar a una persona

#### Scenario: Manual retira lo que las reglas concederían

- **WHERE** una regla del usuario concede una petición
- **WHEN** la sesión corre en modo manual
- **THEN** la petición SHALL escalar a una persona

#### Scenario: Semi concede solo lo contenido

- **WHEN** una edición dentro del árbol de la sesión llega en modo semi
- **THEN** SHALL concederse
- **AND** una edición cuya contención no pueda afirmarse SHALL escalar

#### Scenario: Sin modo, la resolución es la de siempre

- **WHEN** una sesión no declara modo
- **THEN** la resolución SHALL ser la de las reglas del usuario, sin composición

#### Scenario: Ningún modo omite el proxy

- **WHEN** se recorren los modos admitidos
- **THEN** ninguno SHALL omitir el proxy de permisos
- **AND** ninguno SHALL alterar la denegación sin clientes
