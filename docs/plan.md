# Plan: `stepwise`, reescritura en Rust de pyBashResume (spec, tareas y aprendizaje)

## Contexto
pyBashResume (~700 líneas Python: `pyBashResume.py`, `ui_menu.py`, `custom_parser.py`, `custom_logging.py`)
ejecuta ficheros de pasos en orden alfabético. Cada línea es un proceso de shell independiente (`sh -c` en Python, `bash -euo pipefail` en stepwise), con marcas
`#>` (log), `#.` (stop) y `#` (comentario). Guarda el estado en `state.json`, permite reanudar con un menú
curses y lanza `onerror` cuando algo falla. Es un "runbook ejecutable con checkpoints": cubre lo que bash
no sabe hacer (reanudar).

Hay dos objetivos a la vez:
1. Rehacer **el mismo programa** en Rust como proyecto nuevo, **`stepwise`**, con pocas dependencias, código simple y el menú mejorado con ratatui.
2. **Aprender Rust y un buen flujo de desarrollo con IA**: spec → tareas con estado → implementación paso a paso, con explicación de cada concepto.

**Modo de trabajo:** Claude escribe el código de cada tarea en pasos pequeños y explica los conceptos de Rust que aparecen (con referencias al Rust Book). Javier pregunta, prueba y modifica. Se hace una tarea por iteración.

**¿Es complicado?** No. La dificultad es baja-media: ~600 líneas Rust. Al descartar el PTY (D9), lo más delicado que queda es el Ctrl+C (~15 líneas).

> Este plan se actualiza cuando cambia una decisión. El detalle vivo está en `docs/tasks.md` y `docs/decisions.md`.

## Paso R0: reorganización (primer paso)
No se toca la carpeta actual (`/home/javier/dev/cmnt-forks/pyBashResume`): es el directorio de esta sesión y queda intacta.
1. `mkdir /home/javier/dev/projects/stepwise`.
2. `git clone /home/javier/dev/cmnt-forks/pyBashResume /home/javier/dev/projects/stepwise/pyBashResume`, para consulta (conserva historial y remote).
3. `git init -b main` en `/home/javier/dev/projects/stepwise`, con un `.gitignore` que excluya `pyBashResume/`, `target/`, `state.json`, `migration.log` y `steps/`.
4. Cambiar el directorio de la sesión a `/home/javier/dev/projects/stepwise` (`mcp__ccd_directory__change_directory`).
5. Javier añade el remote de GitHub cuando lo cree (`git remote add origin …`). No hay commits ni push sin que él lo pida.

## Decisiones (→ `docs/decisions.md`)
- ~~**D1 – PTY con `rustix` + `signal-hook`.**~~ Sustituida por D9.
- **D2 – `clap` (derive) para la CLI.**
- **D3 – Directorio home** (`state.json`, `migration.log`, `onerror`). Se resuelve en este orden: `--home DIR` → variable `STEPWISE_HOME` → pregunta interactiva validada. No se guarda en el estado.
- **D4 – `state.json` con el mismo formato que Python**, para poder alternar entre las dos versiones.
- **D5 – Menú ratatui en una sola pantalla** con dos paneles y la confirmación integrada.
- **D6 – Convenciones:** identificadores en inglés; comentarios, docs y mensajes en español.
- **D7 – Librería + binario fino**, módulos planos (`src/lib.rs` con la lógica, `src/main.rs` mínimo).
- **D8 – Directorio de trabajo por comando** (`Command::current_dir`), sin `chdir` global. Cada línea sigue empezando en el workdir.
- **D9 – Sin PTY ni heurística de Odoo.** En Odoo 11–18, `exit 255` con `--stop-after-init` es siempre una excepción al inicializar la BD (un error real). Cualquier código ≠ 0 es fallo; el comando hereda la terminal (colores y prompts). Fuera `rustix`; `signal-hook` solo para Ctrl+C.

## Dependencias
`ratatui` (backend crossterm, features mínimos), `clap` (`derive`, `env`), `signal-hook` (0.3, la misma que crossterm),
`serde` (`derive`) + `serde_json`, `chrono` (`default-features=false`, `clock`).
Los tests usan `std::env::temp_dir()`.

## Estructura del repo `stepwise/`
```
stepwise/
├── AGENTS.md          # Para la IA: flujo spec→tareas, modo enseñanza, convenciones, "empieza leyendo docs/tasks.md"
├── README.md          # Uso, compilación y diferencias con pyBashResume
├── Cargo.toml  .gitignore
├── pyBashResume/      # Original, ignorado por git, solo consulta
├── docs/
│   ├── README.md      # Índice + descripción del flujo de trabajo
│   ├── plan.md        # Este plan
│   ├── spec.md        # Comportamiento observable (formato de pasos, state.json, flujo, log, CLI, menú)
│   ├── architecture.md # Módulos, imports, tests y errores
│   ├── decisions.md   # ADRs cortos D1…D9 (contexto, decisión, alternativas)
│   ├── tasks.md       # Tareas [ ] / [~] / [x] / [-], criterio de aceptación y conceptos de Rust
│   ├── ideas.md       # Valoración de la app y mejoras propuestas
│   ├── cheatsheet.md  # Comandos de cargo/rustup y pruebas manuales
│   └── learning.md    # Diario: conceptos aprendidos por tarea, dudas resueltas, enlaces al Rust Book
├── demo/              # Pasos de ejemplo (steps/) y onerror para probar a mano
├── tests/runner.rs    # Tests de integración
└── src/  main.rs lib.rs cli.rs app.rs parser.rs state.rs log.rs exec.rs runner.rs menu.rs testutil.rs
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
| T8 | `app`: home/env/prompt, `set_path`, `choose_action` | **MVP usable** (sin menú) | stdin, organización de `main`, errores propios |
| T9 | ~~`exec` con PTY + heurística~~ — **descartada (D9)** | — | — |
| T10 | Ctrl+C: `signal_hook::flag` durante `execute` | Ctrl+C para el comando, registra el fallo y lanza onerror | `Arc`, `AtomicBool`, señales |
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
  - Los de `demo/steps/`: `echo`, `ls --color=always`, `cd`, `#>`, un `false` (onerror y estado) y un `#.`.
- Casos a probar: reanudar, menú, "una vez", un prompt interactivo (`read -p`) y Ctrl+C durante `sleep 30`.
- Ejecutar Python (`pyBashResume/`) y Rust alternadamente sobre el mismo `state.json` (pendiente: lo hace Javier).

## Estado
T0–T8 y T10–T13 hechas; T9 descartada (D9). Ideas futuras en `docs/ideas.md`.