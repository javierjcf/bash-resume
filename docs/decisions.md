# Decisiones de diseño (ADR ligeros)

Cada decisión recoge el contexto, la decisión tomada y las alternativas descartadas. Es breve a propósito.
Una decisión nueva se añade al final con el siguiente número, y no se borra nunca: si cambia,
se marca como *Sustituida por Dn*.

---

## D1 — PTY con `rustix` + `signal-hook`

**Contexto.** Necesitamos dos cosas a la vez: que el comando vea una terminal (para que mantenga
colores y barras de progreso) y capturar su salida (para la heurística de §4 de la spec). Con una
tubería se pierden los colores; con la salida heredada no se puede capturar. Hace falta un
pseudo-terminal (PTY), y la librería estándar de Rust no expone `openpty` ni `killpg`.

**Decisión.** Usar `rustix` (`pty`, `process`, `event`) para crear el PTY, hacer `poll` y enviar
señales al grupo de procesos, y `signal-hook` (`flag::register`) para detectar Ctrl+C. Ambas tienen
API segura, así que **no hay `unsafe`** en nuestro código. Las dos ya eran dependencias transitivas de
crossterm: `rustix` es la misma versión y `signal-hook` está fijada a `0.3` para compartir la de crossterm.
El aislamiento del hijo se hace con `std::process::Command::process_group(0)`, sin `setsid`.

**Alternativas.** `libc` (requiere `unsafe` para llamar a C); `nix` (dependencia más grande, y
`sigaction` sigue siendo `unsafe`); `extern "C"` a mano (frágil); salida sin PTY (pierde los colores o la heurística).

## D2 — `clap` (derive) para la CLI

**Contexto.** Hay pocos argumentos, pero queremos `--help`, validación y buenos mensajes de error.
**Decisión.** Usar `clap` con `derive`: es estable y estándar de facto.
**Alternativas.** Parsear `std::env::args` a mano, que da menos código de dependencias pero más código propio y peor UX.

## D3 — Directorio home obligatorio (`--home` → `STEPWISE_HOME` → pregunta)

**Contexto.** En Python, `state.json`, `migration.log` y `onerror` vivían junto al script. Un binario
instalado (por ejemplo en `~/.cargo/bin`) no es un buen sitio para eso.
**Decisión.** Resolverlo en este orden: `--home DIR` → variable `STEPWISE_HOME` → pregunta interactiva,
igual que el resto de rutas. No se guarda en `state.json`, porque el estado vive dentro de home.
**Alternativas.** Directorio del ejecutable (mal sitio si está instalado); directorio actual (implícito y fácil de equivocarse).

## D4 — `state.json` compatible con Python

**Decisión.** Mismas claves y tipos, con `null` para lo que no tiene valor.
**Motivo.** Poder alternar entre las dos versiones durante la migración.

## D5 — Menú ratatui en una sola pantalla

**Decisión.** Dos paneles (ficheros y líneas) con preview en vivo y la confirmación integrada
(`y` / `o` / `q`). Se mantiene el primer prompt de texto con el resumen.
**Alternativas.** Replicar los dos menús curses encadenados de Python (más pasos para el usuario).
`ratatui` usa crossterm como backend, que ya incluye la entrada de teclado.
Se activan solo los features `crossterm`, `std` y `layout-cache`, sin widgets extra.

## D6 — Convenciones

- Identificadores y nombres de ficheros en **inglés**.
- Comentarios, docstrings (`///`), mensajes al usuario y documentación en **español**.
- Código formateado con `cargo fmt` y sin avisos de `cargo clippy -- -D warnings`.
- Sin `unsafe`. Sin `unwrap()` fuera de los tests; en su lugar, se propagan los errores con `?`.
