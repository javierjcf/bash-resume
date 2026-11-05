# Tareas

Estado: `[ ]` pendiente · `[~]` en curso · `[x]` hecha · `[-]` descartada (se conserva el número).
Cada tarea deja el proyecto **compilando y con los tests en verde**. Las referencias `§n` apuntan a [spec.md](spec.md).

| # | Estado | Tarea |
|---|---|---|
| T0 | [x] | Andamiaje del proyecto y docs |
| T1 | [x] | `parser`: clasificar líneas |
| T2 | [x] | `parser`: listar ficheros y leerlos |
| T3 | [x] | `cli` con clap |
| T4 | [x] | `state`: state.json |
| T5 | [x] | `log`: fichero + consola |
| T6 | [x] | `exec`: ejecutar un comando |
| T7 | [x] | `runner`: recorrido de pasos |
| T8 | [x] | `app`: flujo completo → **MVP** |
| T9 | [-] | ~~`exec` con PTY + heurística~~ (D9) |
| T10 | [x] | Ctrl+C durante un comando |
| T11 | [x] | `menu`: estado y teclas |
| T12 | [x] | `menu`: render ratatui |
| T13 | [x] | README y cierre de la documentación |
| T14 | [x] | El directorio actual es el proyecto (`-p/--project`) |
| T15 | [x] | Todo en la TUI, con confirmación en popup, y flags `-y`/`-o` |

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
- **Hace:** `struct Args { workdir: Option<PathBuf>, steps: Option<PathBuf>, home: Option<PathBuf> }`, con `home` leído también de `BASH_RESUME_HOME`.
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

### T6 — `exec`: ejecutar un comando (§4)
- **Hace:** `execute(cmd, workdir, &Logger) -> bool` con `sh -c` (luego `bash -euo pipefail`, D10), stdio heredado y directorio de trabajo propio (D8), más el tiempo de ejecución.
- **Aceptación:** tests: `true` → éxito, `false` → fallo.
- **Rust:** `std::process::Command`, `ExitStatus`, `Instant` y `Duration`.

### T7 — `runner`: recorrido de pasos (§5)
- **Hace:** `run_steps(...) -> io::Result<Outcome>`, búsqueda del siguiente `cmd`, `stop`, `once`, `onerror` y `State::set_position`.
- **Aceptación:** tests de integración en un directorio temporal: avance del estado entre ficheros, parada en `stop`, fallo con `onerror` y modo una vez.
- **Rust:** slices, índices (`usize`), `enumerate`/`skip`, retornos tempranos, tests de integración.

### T8 — `app`: flujo completo → **MVP** (§2, §7.1)
- **Hace:** en `app.rs`, resolución de home, workdir y steps; `choose_action` (Y/O/N; M avisa "aún no disponible").
- **Aceptación:** se usa de verdad con los pasos de ejemplo, sin menú.
- **Rust:** leer de stdin, genéricos con `impl Trait` (`BufRead`, `Write`), `ExitCode`, `drop`, organización de `main`.
- **Nota:** `[M]` fue provisional (empezaba por el principio) hasta el menú de T12.

### T9 — ~~`exec` con PTY + heurística~~ — descartada (D9)
- En Odoo, un `exit 255` con `--stop-after-init` siempre es un error real, así que la heurística no aporta nada y el PTY tampoco. Los comandos siguen heredando la terminal (colores y prompts funcionan).

### T10 — Ctrl+C durante un comando (§4)
- **Hace:** dos manejadores de SIGINT registrados una sola vez (`OnceLock`): `flag::register_conditional_default` (termina bash-resume solo si `terminate` es `true`) y `flag::register` (anota que llegó). `execute` pone `terminate = false` mientras dura el comando; el comando recibe el Ctrl+C de la terminal y se detiene. Al volver: log `⛔ Ctrl+C detectado…` y fallo normal (estado + `onerror`). No se quita el manejador, porque signal-hook no restaura el comportamiento por defecto al quitarlo.
- **Aceptación:** Ctrl+C durante `sleep 30` detiene el `sleep`, deja el estado en esa línea, registra la interrupción y ejecuta `onerror`; Ctrl+C en una pregunta sale sin ejecutar nada.
- **Rust:** `Arc`, `AtomicBool` y `Ordering`, `OnceLock` y `static`, señales Unix.
- **Verificación:** manual, con un Ctrl+C real (ver cheatsheet). Un test automático tendría que enviar señales al propio proceso de tests, que ejecuta tests en paralelo.

### T11 — `menu`: estado y teclas (§7.2)
- **Hace:** `struct Menu` (ficheros cargados, panel, cursores, posición guardada), `enum Key` propio y `fn handle_key(&mut self, Key) -> Outcome` (`Stay` / `Run(Choice)` / `Quit`), sin dibujar nada.
- **Aceptación:** tests de navegación cíclica, cambio de panel, confirmación y fichero vacío.
- **Rust:** máquina de estados con `enum`, separación entre lógica y presentación.

### T12 — `menu`: render ratatui (§7.2)
- **Hace:** el layout de dos paneles más la barra, colores por tipo y el bucle de eventos.
- **Aceptación:** el menú funciona y la terminal se restaura siempre, también tras un panic.
- **Rust:** `ratatui::init/restore`, `Layout`, `List`/`ListState`, `Style`, closures.

### T13 — README y cierre de la documentación
- **Hace:** `README.md` de uso (inicio rápido, preguntas, menú, formato de pasos, qué pasa cuando…, compilar e instalar con musl, diferencias con pyBashResume) y repaso de los docs. El menú ya quedó conectado en T12.
- **Pendiente (lo hace Javier):** verificación frente a la versión Python alternando las dos sobre el mismo `state.json`, y la compilación/distribución del binario.

### T14 — El directorio actual es el proyecto (D11)
- **Hace:** `-p/--project` sustituye a `-H/--home` (sin variable de entorno). Sin `-p`, el proyecto es el directorio actual. Si no hay `state.json`, se avisa y se crea; después se preguntan las rutas que falten. Se elimina la carpeta de pasos por defecto (`migration-steps`).
- **Aceptación:** `bash-resume` a secas en una carpeta vacía avisa, crea `state.json` y pregunta pasos y workdir; al relanzar no pregunta rutas; `-p /no/existe` da error y código 1.
- **Rust:** `std::env::current_dir`, `Option<&Path>` con `match`, `io::Error::other`, `Path::exists`.

### T15 — Todo en la TUI, con confirmación en popup, y flags `-y`/`-o` (D12)
- **Hace:** desaparece la pregunta de texto `Y/M/O/N`. La TUI se abre siempre, con el cursor sobre la línea guardada si la hay. Elegir una línea abre el popup y solo desde él se ejecuta. Cabecera con workdir, pasos y estado. `-y/--yes` y `-o/--once` continúan o ejecutan un comando sin abrir la TUI.
- **Aceptación:** nunca se ejecuta sin pasar por el popup; `Esc` en el popup vuelve a navegar; `-y`/`-o` funcionan sin terminal y dan error con estado inválido; sin terminal y sin flags, error claro.
- **Rust:** estado modal comprobado antes del `match`, `FnOnce` frente a `FnMut`, `Clear`, `Flex::Center`, `Paragraph` + `Wrap`, `Span`/`Line` de ratatui, `conflicts_with` de clap.
