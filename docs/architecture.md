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

Estructura **plana**: un fichero por módulo. Con ~600 líneas no hacen falta subcarpetas.

```
src/
├── main.rs     binario: parsea los args con clap, llama a app::run y devuelve el código de salida
├── lib.rs      declara los módulos (pub mod …) y el helper with_path para los errores
├── cli.rs      struct Args (clap)
├── app.rs      flujo principal: resolver rutas, preguntas Y/M/O/N y lanzar el runner
├── parser.rs   tipos de línea, clasificar y leer ficheros de pasos
├── state.rs    state.json
├── log.rs      logger a fichero y consola
├── exec.rs     ejecutar un comando (terminal heredada, código de salida, Ctrl+C)
├── runner.rs   recorrido de los pasos, stop, once y onerror
├── menu.rs     menú: lógica (Menu, handle_key) y dibujo con ratatui (render, choose)
└── testutil.rs utilidades para los tests unitarios (temp_dir); solo con `cargo test`
tests/
└── runner.rs   tests de integración (pasos de ejemplo de principio a fin)
```

### Quién usa a quién (las dependencias van siempre hacia abajo)

```
main.rs ──► cli, app
app ──► state, runner, menu (choose, pasado como closure), log
runner ──► parser, state, exec, log
menu ──► parser, state, runner (Mode)
exec ──► log
state ──► parser
parser, log ──► (solo std, crates externos y with_path)
```

Regla: **no hay ciclos**. Los módulos de abajo (`parser`, `log`, `state`) no conocen a los de arriba. Así se pueden testear aislados.

## Módulos e imports: cómo funciona en Rust

En Rust, **los ficheros no se importan solos**. Hay que declarar el módulo, como en un árbol (RB 7.2):

```rust
// lib.rs
pub mod parser;   // "existe el módulo parser" → cargo busca src/parser.rs
```

Para usar algo de otro módulo:

```rust
use crate::parser::Kind;           // `crate` = raíz de NUESTRO crate (lib.rs)
use std::path::{Path, PathBuf};    // librería estándar
use serde::Deserialize;            // crate externo (declarado en Cargo.toml)

// En main.rs, la librería se ve como un crate externo más, con el nombre del package:
use bash_resume::cli::Args;
```

**Visibilidad**: todo es **privado por defecto**, y `pub` lo hace visible fuera del módulo. La regla es hacer
`pub` solo lo que otro módulo necesita: es la API del módulo, y lo demás son detalles internos (RB 7.3).

Si un módulo creciera mucho, pasaría a ser una carpeta: `src/menu.rs` + `src/menu/render.rs`
(con `mod render;` dentro de `menu.rs`). No lo haremos salvo que haga falta.

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
