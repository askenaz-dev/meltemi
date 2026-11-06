# Diseño documental

## Selección e idiomas

Se mantienen los nombres canónicos bajo `docs/`. `acceso-remoto.md` sigue en español; su traducción
vive en `docs/en/acceso-remoto.md`. Las otras doce guías mantienen su original inglés y reciben
una copia española bajo `docs/es/`. `docs/README.md` y `docs/es/README.md` son índices de uso.
El portal reconoce locales explícitos y el idioma de los originales mixtos sin duplicar rutas.

## Evidencia y generación

Cada decisión editorial se registra en `verification.md`, con el módulo o prueba que sostiene la
instrucción. La referencia canónica `docs/referencia-cli.md` sigue siendo la salida exacta del
generador Rust; la versión española explica y conserva la gramática real en inglés. No se
reescribe salida de herramientas como si el producto la ofreciera traducida.

Se conservan los límites de autenticación externa, permisos efectivos de cada agente,
restauración de sesiones libres, acceso remoto por SSH, distribución y firma de plataforma.
Se retiran planes, fases, presupuestos y detalles de implementación que no ayudan a operar el IDE.
Una prueba documental que exigía una frase interna se adapta al límite observable equivalente.

No hay cambios de APIs, formato de proyecto ni runtime. La validación usa binarios y fixtures
locales, sin cuentas ni agentes reales; los tests de documentación verifican los comandos.
