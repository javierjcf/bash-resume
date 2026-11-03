# Diario de aprendizaje de Rust

Una entrada por tarea: los conceptos que han salido, las dudas resueltas y dónde ampliar.
Rust Book: <https://doc.rust-lang.org/book/> (abreviado **RB**, por ejemplo RB 7.2 = capítulo 7.2).

---

## T0 — Andamiaje

**Cargo y `Cargo.toml`**
- `cargo init` crea un *package* con un *crate* binario (`src/main.rs`). El nombre del binario sale de `[package] name`.
- `edition = "2024"` fija la versión del lenguaje (cambios de sintaxis y lints), no la del compilador. RB 1.3.
- `cargo add crate --features a,b` edita `Cargo.toml` por ti. `"0.3"` significa "compatible con 0.3.x" (semver: en versiones `0.x`, el número menor es el que rompe compatibilidad).

**Features**
- Los crates exponen partes opcionales con *features*. Con `default-features = false` empiezas de cero y activas solo lo que necesitas. Así hemos quitado de ratatui widgets que no usamos (y el crate `time` que traían).

**`cargo tree`**
- `cargo tree -i X` muestra quién depende de X. `cargo tree --duplicates` muestra los crates compilados en varias versiones.
- Hemos fijado `signal-hook = "0.3"` porque crossterm ya usa la 0.3. Con la 0.4 se compilarían las dos.
- `syn` aparece duplicado, pero solo lo usan las macros en compilación (*proc-macros*), así que no llega al binario.

**Herramientas diarias**
- `cargo check`: comprueba tipos sin generar el binario. Es lo más rápido mientras escribes.
- `cargo clippy`: linter con sugerencias idiomáticas. Lee sus mensajes, que enseñan mucho.
- `cargo fmt`: formato estándar, así que no hay discusión de estilo.
- `cargo test`: ejecuta los `#[test]`.
