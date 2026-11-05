# Changelog

Cambios de cada versión de bash-resume. Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/);
las versiones siguen [SemVer](https://semver.org/lang/es/).

## [Unreleased]

## [0.1.0] - 2026-11-05

Primera versión: reescritura en Rust de [pyBashResume](https://github.com/Comunitea/pyBashResume), con su
misma funcionalidad y un único binario sin dependencias.

### Funcionalidades

- **Ficheros de pasos**: se ejecutan en orden alfabético, línea a línea. Cada línea es un proceso independiente
  que empieza siempre en el directorio de trabajo. Marcas: `#>` mensaje al log, `#.` parada y `#` comentario.
- **Reanudar**: `state.json` guarda por dónde va tras cada comando. Al relanzar se abre la interfaz con un popup
  de confirmación sobre el punto pendiente: continuar, ejecutar un solo comando, elegir otra línea o salir.
- **Interfaz de terminal**: ficheros y líneas en dos paneles, coloreadas por tipo, con el punto guardado resaltado.
  Nunca se ejecuta nada sin confirmarlo en el popup.
- **`-y/--yes` y `-o/--once`** para continuar o ejecutar un solo comando sin abrir la interfaz.
- **Fallos**: se guarda la línea que falló, se ejecuta `onerror` (por ejemplo, un aviso por Telegram) y se sale con
  código 1. Al reanudar se repite esa línea.
- **Ctrl+C** durante un comando lo detiene a él, no a bash-resume: queda registrado en el log y se trata como un fallo.
- **Log** en `migration.log` y en pantalla, con el fichero de pasos en curso y la duración de cada comando.
- **Instalación con una línea** (`curl … | sh`): descarga la release, comprueba el checksum y deja el binario
  estático en `~/.local/bin` (o donde se indique).
- Los comandos usan la terminal directamente: se ven sus colores y barras de progreso y se puede responder a sus
  preguntas (`sudo`, `[y/N]`…).

### Diferencias con pyBashResume

- **El directorio actual es el proyecto**: `state.json`, `migration.log` y `onerror` viven en la carpeta desde la
  que se lanza (o la indicada con `-p/--project`), en lugar de junto al script. Si no hay `state.json`, se avisa y
  se crea el proyecto.
- **Sin estructura impuesta**: la carpeta de pasos y el directorio de trabajo se indican la primera vez (por
  argumento o respondiendo a la pregunta) y quedan guardados; ya no hay `migration-steps` por defecto.
- **Argumentos más cortos**: `bash-resume [WORKDIR] [-s|--steps DIR] [-p|--project DIR]` (antes `--steps-path`).
  Un argumento antiguo muestra una sugerencia.
- **Los comandos se ejecutan con `bash -euo pipefail`** en vez de `sh -c`: misma sintaxis en todas las distribuciones,
  las tuberías fallan si falla cualquier parte y una variable sin definir es un error.
- **Sin heurística para `exit 255` con `--stop-after-init`**: en Odoo, un 255 es siempre un error real, así que
  cualquier código distinto de 0 cuenta como fallo.
- **Los comandos pueden pedir datos por teclado**: en Python no se podía responder a sus preguntas.
- **Todo en una interfaz de terminal**, en lugar de una pregunta de texto y dos menús curses seguidos; se mantiene
  el flujo de confirmar antes de ejecutar.
- **Más robusto**: `state.json` se guarda de forma atómica (un corte nunca lo deja a medias), los errores indican
  qué fichero falló, un fichero vacío en el menú ya no rompe el programa, se ignoran las subcarpetas de pasos y
  se aceptan finales de línea de Windows.
- Una respuesta no válida en la pregunta inicial vuelve a preguntar (antes abría el menú), y el log solo usa
  colores si la salida es una terminal.
- `state.json` mantiene el formato de pyBashResume: se pueden alternar las dos versiones sobre el mismo proyecto.
- Se mantienen a propósito dos comportamientos del original: al reanudar sobre una parada `#.` se vuelve a parar,
  y al terminar todos los pasos el estado queda en el último comando.

[Unreleased]: https://github.com/javierjcf/bash-resume/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/javierjcf/bash-resume/releases/tag/v0.1.0
