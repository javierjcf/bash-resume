# Tareas

Estado: `[ ]` pendiente · `[~]` en curso · `[x]` hecha.
Cada tarea deja el proyecto **compilando y con los tests en verde**. Las referencias `§n` apuntan a [spec.md](spec.md).

| # | Estado | Tarea |
|---|---|---|
| T0 | [x] | Andamiaje del proyecto y docs |
| T1 | [x] | `parser`: clasificar líneas |
| T2 | [x] | `parser`: listar ficheros y leerlos |
| T3 | [x] | `cli` con clap |
| T4 | [x] | `state`: state.json |
| T5 | [x] | `log`: fichero + consola |
| T6 | [ ] | `exec` simple (sin PTY) |
| T7 | [ ] | `runner`: recorrido de pasos |
| T8 | [ ] | `main`: flujo completo → **MVP** |
| T9 | [ ] | `exec` con PTY + heurística |
| T10 | [ ] | Ctrl+C |
| T11 | [ ] | `menu`: estado y teclas |
| T12 | [ ] | `menu`: render ratatui |
| T13 | [ ] | Integración y verificación final |

---

### T0 — Andamiaje del proyecto y docs
- **Hace:** crea el repo nuevo con el original en `pyBashResume/` (ignorado por git), `cargo init`, las dependencias y los docs (`spec`, `decisions`, `tasks`, `learning`) y `AGENTS.md`.
- **Aceptación:** `cargo build` y `cargo clippy` sin errores; `cargo tree --duplicates` sin duplicados en runtime.
- **Rust:** cargo, `Cargo.toml`, crates y features, `cargo tree`, edición 2024.

### T1 — `parser`: clasificar líneas (§3)
- **Hace:** `enum Kind { Cmd, Log, Stop, Comment, Empty }`, `struct Line { kind, text }` y `fn classify(&str) -> Kind`.
- **Aceptación:** tests unitarios para cada tipo, espacios al inicio, y que `#>` y `#.` tengan prioridad sobre `#`.
- **Rust:** `enum`, `match`, `&str` frente a `String`, `derive(Debug, PartialEq)`, módulos (`mod`), `#[cfg(test)]`.

### T2 — `parser`: listar ficheros y leerlos (§3)
- **Hace:** `list_step_files(dir) -> io::Result<Vec<String>>` (sin ocultos y ordenados) y `parse_file(path) -> io::Result<Vec<Line>>`. `main` imprime provisionalmente los ficheros y sus líneas.
- **Aceptación:** tests con un directorio temporal; `cargo run` lista una carpeta de ejemplo.
- **Rust:** `Result`, operador `?`, `Path` y `PathBuf`, iteradores (`filter`, `map`, `collect`), `sort`.

### T3 — `cli` con clap (§9)
- **Hace:** `struct Args { workdir: Option<PathBuf>, steps: Option<PathBuf>, home: Option<PathBuf> }`, con `home` leído también de `STEPWISE_HOME`.
- **Aceptación:** `cargo run -- --help` muestra nuestras descripciones en español (los textos propios de clap, como "Usage" u "Options", siguen en inglés).
- **Rust:** macros `derive`, atributos, `Option`.

### T4 — `state`: state.json (§6)
- **Hace:** `struct State` con serde; `load` (crea, repara o recrea si está corrupto), `save`, `is_valid` y el resumen (`impl Display`).
- **Aceptación:** tests de JSON corrupto, claves que faltan, `is_valid`, y lectura de un `state.json` generado por la versión Python.
- **Rust:** `impl`, `&self` frente a `&mut self`, serde (`Serialize`/`Deserialize`, `#[serde(default)]`), `Option<T>`, manejo de errores.

### T5 — `log`: fichero + consola (§8)
- **Hace:** `struct Logger` (fichero en modo append + fuente actual), `info`, `warn`, `error` y `set_source`.
- **Aceptación:** test del formato del fichero; prueba visual de los colores en consola.
- **Rust:** structs que poseen recursos (`File`), `write!`/`format!`, préstamos y mutabilidad, `chrono::Local`.

### T6 — `exec` simple (§4, sin PTY)
- **Hace:** `execute(cmd, &Logger) -> bool` con `sh -c` y stdio heredado, más el tiempo de ejecución.
- **Aceptación:** tests: `true` → éxito, `false` → fallo.
- **Rust:** `std::process::Command`, `ExitStatus`, `Instant` y `Duration`.

### T7 — `runner`: recorrido de pasos (§5)
- **Hace:** `run_steps`, búsqueda del siguiente `cmd`, `stop`, `once` y `onerror`.
- **Aceptación:** tests de integración en un directorio temporal: avance del estado entre ficheros, parada en `stop`, fallo con `onerror` y modo una vez.
- **Rust:** slices, índices (`usize`), `enumerate`/`skip`, retornos tempranos, tests de integración.

### T8 — `app`: flujo completo → **MVP** (§2, §7.1)
- **Hace:** en `app.rs`, resolución de home, workdir y steps; `choose_action` (Y/O/N; M avisa "aún no disponible") y `chdir`.
- **Aceptación:** se usa de verdad con los pasos de ejemplo, sin menú ni PTY.
- **Rust:** leer de stdin, organización de `main`, `Box<dyn Error>`, `std::process::exit`.

### T9 — `exec` con PTY + heurística (§4)
- **Hace:** sustituye el stdio heredado por un PTY de `rustix`, con *tee* a stdout y a un buffer, e implementa `is_real_error`.
- **Aceptación:** `ls --color=auto` sale coloreado; tests de `is_real_error`.
- **Rust:** `OwnedFd`, RAII (`Drop`), traits `Read`/`Write`, conversión `Stdio::from`.

### T10 — Ctrl+C (§4)
- **Hace:** `signal-hook::flag` + `poll` con timeout + `kill_process_group(SIGINT)`.
- **Aceptación:** Ctrl+C durante `sleep 30` mata al hijo y deja el estado en esa línea; Ctrl+C en una pregunta sale con código 1.
- **Rust:** `Arc`, `AtomicBool` y `Ordering`, señales Unix.

### T11 — `menu`: estado y teclas (§7.2)
- **Hace:** `struct App` (panel activo, índices) y `fn handle_key(...) -> Option<Outcome>`, sin dibujar nada.
- **Aceptación:** tests de navegación cíclica, cambio de panel, confirmación y fichero vacío.
- **Rust:** máquina de estados con `enum`, separación entre lógica y presentación.

### T12 — `menu`: render ratatui (§7.2)
- **Hace:** el layout de dos paneles más la barra, colores por tipo y el bucle de eventos.
- **Aceptación:** el menú funciona y la terminal se restaura siempre, también tras un panic.
- **Rust:** `ratatui::init/restore`, `Layout`, `List`/`ListState`, `Style`, closures.

### T13 — Integración y verificación final
- **Hace:** conecta el menú con `main`, escribe el `README.md` y hace la verificación manual completa (ver el plan).
- **Aceptación:** paridad con Python; se pueden alternar las dos versiones con el mismo `state.json`.
