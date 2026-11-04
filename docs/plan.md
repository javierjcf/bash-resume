# Plan: `stepwise`, reescritura en Rust de pyBashResume (spec, tareas y aprendizaje)

## Contexto
pyBashResume (~700 líneas Python: `pyBashResume.py`, `ui_menu.py`, `custom_parser.py`, `custom_logging.py`)
ejecuta ficheros de pasos en orden alfabético. Cada línea es un proceso `sh -c` independiente, con marcas
`#>` (log), `#.` (stop) y `#` (comentario). Guarda el estado en `state.json`, permite reanudar con un menú
curses y lanza `onerror` cuando algo falla. Es un "runbook ejecutable con checkpoints": cubre lo que bash
no sabe hacer (reanudar).

Hay dos objetivos a la vez:
1. Rehacer **el mismo programa** en Rust como proyecto nuevo, **`stepwise`**, con pocas dependencias, código simple y el menú mejorado con ratatui.
2. **Aprender Rust y un buen flujo de desarrollo con IA**: spec → tareas con estado → implementación paso a paso, con explicación de cada concepto.

**Modo de trabajo:** Claude escribe el código de cada tarea en pasos pequeños y explica los conceptos de Rust que aparecen (con referencias al Rust Book). Javier pregunta, prueba y modifica. Se hace una tarea por iteración.

**¿Es complicado?** No. La dificultad es baja-media: ~600 líneas Rust. Lo delicado es el PTY + Ctrl+C (~80 líneas).

## Paso R0: reorganización (primer paso)
No se toca la carpeta actual (`/home/javier/dev/cmnt-forks/pyBashResume`): es el directorio de esta sesión y queda intacta.
1. `mkdir /home/javier/dev/projects/stepwise`.
2. `git clone /home/javier/dev/cmnt-forks/pyBashResume /home/javier/dev/projects/stepwise/pyBashResume`, para consulta (conserva historial y remote).
3. `git init -b main` en `/home/javier/dev/projects/stepwise`, con un `.gitignore` que excluya `pyBashResume/`, `target/`, `state.json`, `migration.log` y `steps/`.
4. Cambiar el directorio de la sesión a `/home/javier/dev/projects/stepwise` (`mcp__ccd_directory__change_directory`).
5. Javier añade el remote de GitHub cuando lo cree (`git remote add origin …`). No hay commits ni push sin que él lo pida.

## Decisiones (→ `docs/decisions.md`)
- **D1 – PTY con `rustix` + `signal-hook`.** Conserva colores reales y la captura de salida para la heurística `--stop-after-init`/255. No hay `unsafe`, y ambos crates ya son dependencias transitivas de crossterm.
- **D2 – `clap` (derive) para la CLI.**
- **D3 – Directorio home** (`state.json`, `migration.log`, `onerror`). Se resuelve en este orden: `--home DIR` → variable `STEPWISE_HOME` → pregunta interactiva validada. No se guarda en el estado.
- **D4 – `state.json` con el mismo formato que Python**, para poder alternar entre las dos versiones.
- **D5 – Menú ratatui en una sola pantalla** con dos paneles y la confirmación integrada.
- **D6 – Convenciones:** identificadores en inglés; comentarios, docs y mensajes en español.

## Dependencias
`ratatui` (backend crossterm), `clap` (`derive`), `rustix` (`pty`, `process`, `event`), `signal-hook`,
`serde` (`derive`) + `serde_json`, `chrono` (`default-features=false`, `clock`).
Los tests usan `std::env::temp_dir()`.

## Estructura del repo `stepwise/`
```
stepwise/
├── AGENTS.md          # Para la IA: flujo spec→tareas, modo enseñanza, convenciones, "empieza leyendo docs/tasks.md"
├── README.md          # Uso (se completa en T13)
├── Cargo.toml  .gitignore
├── pyBashResume/      # Original, ignorado por git, solo consulta
├── docs/
│   ├── README.md      # Índice + descripción del flujo de trabajo
│   ├── spec.md        # Comportamiento observable (formato de pasos, state.json, flujo, log, CLI, menú)
│   ├── decisions.md   # ADRs cortos D1…D6 (contexto, decisión, alternativas)
│   ├── tasks.md       # Tareas [ ] / [~] / [x], criterio de aceptación y conceptos de Rust
│   └── learning.md    # Diario: conceptos aprendidos por tarea, dudas resueltas, enlaces al Rust Book
└── src/  main.rs cli.rs parser.rs state.rs log.rs exec.rs runner.rs menu.rs
```
`spec.md` toma como referencia la versión Python y anota las correcciones, por ejemplo el crash con un fichero vacío en el menú, donde `idx=None` llega a `idx + 1`.

## Tareas (→ `docs/tasks.md`; cada una compila y tiene tests)
| # | Tarea | Resultado visible | Conceptos Rust |
|---|---|---|---|
| T0 | R0 + `cargo init`, deps, fmt/clippy, docs (spec, decisions, tasks, AGENTS.md) | `cargo run` → hola | cargo, crates, features, módulos |
| T1 | `parser`: `enum Kind`, `struct Line`, `classify(&str)` | tests verdes | enums, `match`, `&str` vs `String`, `#[cfg(test)]` |
| T2 | `list_step_files` + `parse_file` | lista ficheros y líneas tipadas | `Result`, `?`, `Path`/`PathBuf`, iteradores |
| T3 | `cli` con clap | `--help` y argumentos | derive macros, `Option` |
| T4 | `state`: load/repair/save, `is_valid`, `summary` | lee un state.json de Python | serde, `impl`, `&self`/`&mut self`, errores |
| T5 | `log`: fichero + consola con color, fuente actual | log idéntico al de Python | structs con recursos, `Display`, préstamos |
| T6 | `exec` simple: `Command` con stdio heredado + tiempo | ejecuta un comando | `std::process`, `Instant`, `ExitStatus` |
| T7 | `runner`: run_steps, next_cmd, stop, once, onerror | pasos de ejemplo de principio a fin | slices, índices, bucles, tests de integración |
| T8 | `main`: home/env/prompt, `set_path`, `choose_action` | **MVP usable** (sin menú ni PTY) | stdin, organización de `main`, errores propios |
| T9 | `exec` con PTY + tee + `is_real_error` | colores y heurística | `OwnedFd`, RAII, `Read`/`Write` |
| T10 | Ctrl+C: signal-hook + poll + `kill_process_group` | Ctrl+C mata al hijo y guarda el estado | `Arc`, `AtomicBool`, señales |
| T11 | `menu`: estado de la app + teclas (testeable sin terminal) | tests de navegación | máquina de estados con enums |
| T12 | `menu`: render ratatui (paneles, colores, barra) | menú completo | `Layout`, widgets, `Style` |
| T13 | Integración, README, verificación manual | paridad con Python | — |

## Menú ratatui (resumen de la spec)
- **Panel izquierdo:** ficheros, con `●` en el que marca el estado.
- **Panel derecho:** líneas con preview en vivo, número de línea y color por tipo (`#>` verde, `#.` rojo, `#` gris). La línea guardada en el estado sale en amarillo.
- **Barra inferior:** `fichero:línea → comando` y la ayuda de teclas.
- **Teclas:** `↑↓`/`jk` mover, `Tab`/`←→` cambiar de panel, `Enter`/`y` ejecutar, `o` ejecutar una vez, `q`/`Esc` salir o volver.
- El primer prompt de texto con el resumen del estado se mantiene. El segundo queda integrado en la TUI.

## Ciclo por tarea
1. Marcar la tarea `[~]` en `tasks.md`.
2. Explicar los conceptos.
3. Escribir el código en pasos pequeños.
4. `cargo test` + `cargo clippy`.
5. Javier prueba o pregunta.
6. Marcar `[x]` y añadir la entrada en `learning.md`.
7. Commit solo si Javier lo pide.

## Verificación final (T13)
- `cargo test && cargo clippy -- -D warnings`.
- Carpeta `steps/` de ejemplo:
  - `01-a`: `echo`, `ls --color=always` y una línea `#>`.
  - `02-b`: `false`, para comprobar onerror y que el estado queda en esa línea.
  - `03-c`: una línea `#.`.
- Casos a probar: reanudar, menú, "una vez" y Ctrl+C durante `sleep 30`.
- Ejecutar Python (`pyBashResume/`) y Rust alternadamente sobre el mismo `state.json`.