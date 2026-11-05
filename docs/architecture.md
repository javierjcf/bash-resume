# Arquitectura y organización del código

Cómo está organizado el código y por qué. El *qué* hace el programa está en [spec.md](spec.md).

## Un package, dos crates

```
Cargo.toml          → el *package* "bash-resume"
src/lib.rs          → crate LIBRERÍA "bash-resume": toda la lógica
src/main.rs         → crate BINARIO "bash-resume": solo arranca (args → bash_resume::run → código de salida)
```

Es el patrón recomendado en el Rust Book (RB 12.3, *Separation of Concerns for Binary Projects*):
- `main.rs` es mínimo y no tiene lógica, porque es difícil de testear.
- `lib.rs` contiene toda la lógica, se puede testear, y los tests de integración de `tests/` pueden usarla como cualquier otro crate.

## Mapa de módulos

Organizado **por capas**: la lógica (`engine`), la ejecución de comandos (`exec`) y la interfaz (`tui`), cada una
en su carpeta; lo que une todo (`app`, `cli`) y lo transversal (`log`) quedan en la raíz.

```
src/
├── main.rs          binario: parsea los args con clap, llama a app::run y devuelve el código de salida
├── lib.rs           declara los módulos (pub mod …) y los helpers with_path (errores) y write_atomic
├── cli.rs           struct Args (clap)
├── app.rs           flujo principal: resolver rutas, abrir la TUI (o -y/-o) y lanzar el recorrido
├── log.rs           logger a fichero y consola; format_duration (H:MM:SS)
├── engine.rs        ── lógica, sin terminal ni interfaz
├── engine/
│   ├── parser.rs    tipos de línea, clasificar y leer ficheros de pasos (StepFile, load_step_files)
│   ├── state.rs     state.json
│   ├── timings.rs   times.json: tiempo de cada comando, totales, lo que queda y [n/N]
│   └── runner.rs    recorrido de los pasos, stop, once y onerror
├── exec.rs          ── ejecutar comandos (expone `execute`)
├── exec/
│   ├── command.rs   lanzar bash -euo pipefail, código de salida y tiempo
│   ├── pty.rs       terminal falsa vacía para el stdin de los comandos (D15)
│   └── signals.rs   Ctrl+C durante un comando (OnceLock + atómicos)
├── tui.rs           ── interfaz de terminal (expone `choose` y `Choice`): bucle de eventos
├── tui/
│   ├── menu.rs      estado y teclas (Menu, handle_key, popup de confirmación); testeable sin terminal
│   ├── render.rs    dibujo con ratatui (cabecera, paneles, barra, popup)
│   ├── keys.rs      teclas de crossterm → nuestro `Key`
│   └── testdata.rs  datos compartidos por los tests de la TUI (solo con `cargo test`)
└── testutil.rs      utilidades para los tests unitarios (temp_dir); solo con `cargo test`
tests/
└── runner.rs        tests de integración (pasos de ejemplo de principio a fin)
```

Cada carpeta tiene un fichero del mismo nombre en el nivel superior (`engine.rs`, `exec.rs`, `tui.rs`) que declara
sus submódulos y reexporta lo que se usa desde fuera (`pub use command::execute;`). Así el resto del código no
depende de cómo está repartida la carpeta por dentro. No se usa `mod.rs` (forma antigua, equivalente).

### Quién usa a quién (las dependencias van siempre hacia abajo)

```
main.rs ──► cli, app
app ──► engine (state, runner), tui (choose, pasado como closure), log
tui ──► engine (parser, state, runner::Mode)
engine::runner ──► engine::parser, engine::state, exec, log
engine::state ──► engine::parser
exec ──► log
engine::parser, log ──► (solo std, crates externos y with_path)
```

Regla: **no hay ciclos**, y **`engine` no conoce `tui` ni `app`**: la lógica no sabe que existe una interfaz.
Así se puede testear aislada.

## Módulos e imports: cómo funciona en Rust

En Rust, **los ficheros no se importan solos**. Hay que declarar el módulo, como en un árbol (RB 7.2):

```rust
// lib.rs
pub mod engine;   // "existe el módulo engine" → cargo busca src/engine.rs (y sus hijos en src/engine/)
```

Para usar algo de otro módulo:

```rust
use crate::engine::parser::Kind;   // `crate` = raíz de NUESTRO crate (lib.rs)
use std::path::{Path, PathBuf};    // librería estándar
use serde::Deserialize;            // crate externo (declarado en Cargo.toml)

// En main.rs, la librería se ve como un crate externo más, con el nombre del package:
use bash_resume::cli::Args;
```

**Visibilidad**: todo es **privado por defecto**, y `pub` lo hace visible fuera del módulo. La regla es hacer
`pub` solo lo que otro módulo necesita: es la API del módulo, y lo demás son detalles internos (RB 7.3).

Si un fichero de una carpeta creciera mucho, se parte dentro de ella (como `tui/` en `menu.rs`, `render.rs` y
`keys.rs`), sin cambiar lo que la carpeta expone.

## Tests

| Tipo | Dónde | Qué prueba |
|---|---|---|
| Unitarios | Al final de cada fichero, en `#[cfg(test)] mod tests { … }` | Funciones del módulo, incluidas las privadas. Las utilidades comunes (como `temp_dir`) están en `src/testutil.rs` |
| Integración | `tests/*.rs` | La API pública de la librería, como la usaría el binario. No ven `testutil` (que es `#[cfg(test)]` de la librería), así que tienen sus propios helpers |

`#[cfg(test)]` hace que ese código **solo se compile** al ejecutar `cargo test`; no llega al binario (RB 11.3).

## Errores

- Los módulos devuelven `std::io::Result<T>` (casi todos los errores aquí son de E/S) y se propagan con `?`.
- Al leer o escribir un fichero, el error lleva la ruta: `.map_err(with_path(path))?` (`lib.rs`). Si no, el usuario vería `stream did not contain valid UTF-8` sin saber qué fichero es.
- `state.json` se guarda de forma atómica (temporal + `rename`), para que un corte nunca lo deje a medias.
- `app::run` devuelve `io::Result<ExitCode>` (todos nuestros errores son de E/S). `main` imprime el error con `Display` (`❌ Error: …`) y sale con código 1.
- Las funciones que preguntan reciben `&mut impl BufRead` y `&mut impl Write` en lugar de usar `stdin`/`stdout` directamente: así los tests les pasan texto inventado.
- Nada de `unwrap()` fuera de los tests (D6).

## Guías de referencia

- **Rust Book**: cap. 7 (módulos), 9 (errores), 11 (tests), 12 (proyecto binario). `rustup doc --book`.
- **Rust API Guidelines** (<https://rust-lang.github.io/api-guidelines/>): nombres y convenciones (por ejemplo `snake_case` para funciones, `CamelCase` para tipos, `as_`/`to_`/`into_`).
- **Clippy** y **rustfmt**: si clippy sugiere algo, se hace o se justifica.
