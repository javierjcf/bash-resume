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

---

## T1 — `parser`: clasificar líneas

**Librería + binario** (RB 12.3, 7.2)
- `lib.rs` declara los módulos con `pub mod parser;`, y el compilador busca entonces `src/parser.rs`. Los ficheros no se cargan solos.
- Dentro de la librería se importa con `use crate::…`; desde `main.rs` con `use stepwise::…`, porque el binario ve la librería como un crate externo.

**`enum`** (RB 6.1)
- Conjunto cerrado de valores. Frente a usar strings (`"cmd"`), una errata no compila.

**`derive` y traits** (RB 5.2, apéndice C)
- Un tipo nuevo no sabe imprimirse ni compararse. Un *trait* es una capacidad (≈ interfaz), y `derive` genera su implementación (≈ `@dataclass`).
- `Debug` → `{:?}`; `PartialEq`/`Eq` → `==` y `assert_eq!`; `Clone`/`Copy` → se copia en vez de moverse.
- Sin `Debug`, el error `E0277` lo sugiere: *consider annotating with `#[derive(Debug)]`*.

**`&str` frente a `String`** (RB 4.3, 8.2)
- `&str` = texto **prestado**, de solo lectura (puntero + longitud). `.trim()` devuelve otro `&str` sobre el mismo texto, sin copiar.
- `String` = texto **propio**, en el heap. `Line` guarda `String` porque tiene que *quedarse* con el texto: `text.to_string()` hace la copia.
- `Line` no es `Copy`: contiene un `String` y duplicarlo cuesta memoria. Sigue pudiendo ser `Clone` (copia explícita con `.clone()`).

**`match`** (RB 6.2, 19)
- Las ramas `patrón => valor` se prueban en orden. `t if cond` = captura + guarda; `_` = comodín.
- Es **exhaustivo**: sin `_` sobre un `&str` da el error `E0004`, porque las guardas no cuentan para la exhaustividad.
- Es una **expresión**: su valor es el de la función, sin `return` ni `;`.

**`struct` + `impl`** (RB 5.1, 5.3)
- `struct` define los datos e `impl` sus funciones. `fn new(...) -> Self` es un *constructor asociado*: se llama `Line::new(...)`, sin `self`. `Self` = el propio tipo.
- `pub` en cada campo: los campos son privados por defecto, aunque el struct sea `pub`.

**Tests** (RB 11.1, 11.3)
- `#[cfg(test)] mod tests` solo se compila con `cargo test`. `use super::*;` importa todo el módulo padre, incluido lo privado.
- `#[test]` marca una función de test; `assert_eq!(real, esperado)`, y si falla muestra `left` y `right`.
- `cargo test nombre` ejecuta solo los tests cuyo nombre contiene `nombre`.
