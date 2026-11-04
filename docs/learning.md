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

---

## T2 — `parser`: listar ficheros y leerlos

**Ownership y borrow checker** (RB 4.1–4.2)
- Cada valor tiene **un dueño**, y cuando el dueño sale de su bloque `{}` el valor se libera (*drop*). No hay recolector de basura.
- Prestar: `&x` (lectura, se pueden hacer muchos a la vez) o `&mut x` (escritura, solo uno, y sin lectores mientras dure).
- Regla de oro: **un préstamo no puede vivir más que su dueño**. Si no se cumple, el error es `E0597` (use-after-free evitado al compilar; ver el ejemplo en C frente a Rust).
- En `parse_file`: `content` (String) es el dueño; `.lines()` da ventanas `&str`; `Line::new` copia cada una a un `String` propio; `content` se libera al salir y el `Vec<Line>` sobrevive porque no depende de él.

**`Result` y `?`** (RB 9.2)
- `io::Result<T>` = `Result<T, io::Error>`: o `Ok(valor)` o `Err(error)`. La firma avisa de que la función puede fallar.
- `x?` → si es `Err`, **devuelve** el error a quien llamó; si es `Ok`, saca el valor. Es el equivalente explícito de dejar subir una excepción en Python.
- `main() -> io::Result<()>`: si llega un `Err`, se imprime (en formato Debug) y el proceso sale con código 1.
- `let Ok(x) = expr else { continue; };` (*let-else*): desestructura o sale del bloque.
- `let _ = expr;` ignora a propósito un `Result` (por ejemplo, borrar algo que quizá no exista).

**Rutas: `&Path` frente a `PathBuf`** (std::path)
- La misma pareja que `&str`/`String`: `&Path` prestada para leer, `PathBuf` propia. `dir.join("x")` crea un `PathBuf` nuevo.
- Las rutas **no son `String`**: en Unix pueden no ser UTF-8. Por eso `file_name()` da un `OsString` y `.into_string()` puede fallar.

**Leer ficheros: buenas prácticas**
- `fs::read_to_string(path)?` abre, lee todo y **cierra solo** (RAII: el fichero se cierra cuando su dueño se libera). Equivale a `with open(...) as f: f.read()`.
- Para ficheros pequeños, leer entero es lo simple. Para ficheros enormes, `BufReader::new(File::open(p)?).lines()` lee línea a línea.
- `.lines()` corta por `\n` y quita también un `\r` final (ficheros de Windows).
- Nada de `unwrap()` en código real: se propaga con `?` y se decide arriba qué hacer.

**Iteradores** (RB 13.2)
- Son perezosos: `content.lines().map(Line::new)` no hace nada hasta que algo los consume (`.collect()`, `for`).
- `.collect()` construye la colección que indique el tipo de retorno (`Vec<Line>`).
- `.map(Line::new)` pasa una función por nombre; equivale a `.map(|t| Line::new(t))`.
- `.iter().enumerate()` da `(índice, &elemento)`, como `enumerate()` en Python.
- Closures: `|l| l.kind` = `lambda l: l.kind`.

**Otros**
- *Shadowing*: `let dir = Path::new(&dir);` redefine `dir` con otro tipo. Es idiomático para transformar un valor.
- `Vec::new()` + `push` + `sort()`. `vec!["a", "b"]` crea un `Vec` literal. `Vec<String>` se compara con `vec!["a"]` (de `&str`) gracias a `PartialEq`.
- Tests que devuelven `io::Result<()>` pueden usar `?`. Los nombres de directorio temporal llevan el nombre del test + el PID, para que no choquen en paralelo.

---

## T3 — `cli` con clap

**Derive de un crate externo** (RB 19.5)
- `#[derive(Parser)]` es una *macro procedural* de clap: lee el struct al compilar y genera el parser. Es la misma idea que `derive(Debug)`, pero la escribe un crate.
- `///` encima del struct y de cada campo **se convierte en el texto de `--help`**. La documentación y la ayuda son lo mismo.
- `#[command(version)]` toma la versión de `Cargo.toml`. `#[arg(short, long)]` deriva `-s`/`--steps` del nombre del campo; `short = 'H'` la fuerza (porque `-h` es la ayuda).
- Un campo sin `#[arg]` es **posicional** (`WORKDIR`); `value_name` cambia el `<DIR>` que se muestra.
- `env = "STEPWISE_HOME"` (feature `env`): si falta el flag, clap lee la variable.

**El tipo define el comportamiento**
- `Option<PathBuf>` → argumento **opcional**. Con `PathBuf` sin `Option`, clap lo haría obligatorio. Con `bool`, sería un flag sin valor.
- Python (`argparse`) define los argumentos con llamadas en tiempo de ejecución; clap los deduce del **tipo** al compilar.

**`Args::parse()` frente a `try_parse_from(...)`**
- `parse()` lee los argumentos reales; si son incorrectos, imprime el error y **termina el proceso** (código 2).
- `try_parse_from([...])` devuelve un `Result`: es la que se usa en tests, con argumentos inventados.
- `Args::command().debug_assert()` (trait `CommandFactory`): test recomendado por clap que valida la definición (por ejemplo, flags repetidos).

**Traits que hay que importar**
- `Args::parse()` solo existe si el trait `Parser` está en scope (`use clap::Parser;`). En Rust, los métodos de un trait solo se ven si se importa el trait.

**`Option` en la práctica**
- `args.steps.as_deref()` convierte `Option<PathBuf>` en `Option<&Path>` (prestado, sin mover ni copiar).
- `.unwrap_or(default)` → el valor, o `default` si es `None` (≈ `x or default` en Python).
- `{args:#?}` = Debug "bonito", con saltos de línea.

**Variables de entorno en tests**
- En la edición 2024, `std::env::set_var` es `unsafe`, porque modificar el entorno con hilos en paralelo es una carrera de datos. Por eso no hay test de `STEPWISE_HOME`: se prueba a mano.

**Broken pipe**
- `cargo run | head` hace que `println!` entre en pánico al cerrarse la tubería (`Broken pipe`). Es el comportamiento estándar de Rust (en Python sería un `BrokenPipeError`). No afecta al uso normal.

---

## T4 — `state`: state.json

**serde** (<https://serde.rs>)
- `#[derive(Serialize, Deserialize)]` genera la conversión del struct a JSON y de JSON al struct. serde define el *qué*; `serde_json` es el formato concreto.
- `#[serde(default)]` en el struct: las claves que faltan toman el valor de `Default` (`None` en los `Option`). Las claves desconocidas se ignoran.
- `#[serde(skip)]` en un campo: ni se escribe ni se lee (`path`).
- `Option<T>` ↔ `null`. `usize` solo acepta enteros ≥ 0: un número negativo sería un JSON "corrupto".
- `serde_json::from_str::<State>(&text)` devuelve `Result`; `to_string_pretty(&state)` da un JSON con sangría.

**`impl` y los tres tipos de `self`** (RB 5.3)
| Firma | Qué recibe | Python | Ejemplo |
|---|---|---|---|
| `fn load(path) -> Self` | nada (función asociada) | `@classmethod` / constructor | `State::load(&p)` |
| `fn save(&self)` | préstamo de **lectura** | `def save(self)` | solo lee los campos |
| `fn f(&mut self)` | préstamo de **escritura** | `def f(self)` que modifica | cambiar campos |
| `fn f(self)` | se **queda** con el valor | — | consume el objeto |
- Python no distingue: cualquier método puede modificar `self`. En Rust, la firma lo dice y el compilador lo vigila: asignar en `&self` da `E0594`.
- `let mut state = ...`: las variables son inmutables por defecto. Sin `mut`, `state.path = ...` no compila.

**`Option` sin mover: `as_ref` / `as_deref`**
- Con `&self` solo tienes un préstamo, así que no puedes *sacar* (mover) un `String` de dentro: es el error `E0507` (*cannot move out of … behind a shared reference*).
- `self.current_file.as_ref()` → `Option<&String>` (préstamo del contenido). `.as_deref()` → `Option<&str>`. El compilador sugiere `.clone()`, que también funciona, pero copia.
- `Option<usize>` no tiene ese problema porque `usize` es `Copy`.

**`?` también con `Option`** (RB 9.2)
- En una función que devuelve `Option`, `x?` sale con `None` si `x` es `None`. `position_matches` encadena 6 comprobaciones sin un solo `if`.
- `.ok()` convierte `Result` en `Option` (descarta el error). `lines.get(i)` devuelve `Option` en vez de entrar en pánico como `lines[i]`.
- `index.checked_sub(1)` → `None` si sería negativo: los `usize` no pueden ser < 0 (en Python, `-1` es un índice válido, que era el bug).

**`match` sobre errores, con guardas**
- `Err(e) if e.kind() == io::ErrorKind::NotFound => …`: distinguir "no existe" (normal) de cualquier otro error (se propaga).
- `?` convierte errores con `From`: `serde_json::Error` se convierte en `io::Error`, así que `?` funciona en una función `io::Result`.

**`impl Display` (trait propio)** (RB 10.2)
- Implementar `fmt::Display` para `State` hace que funcionen `println!("{state}")` y `state.to_string()`. `Debug` es para el programador; `Display` es para el usuario y nunca se deriva.
- `writeln!(f, ...)` escribe en el `Formatter`; `{label:<24}` alinea a 24 caracteres.

**Struct update syntax**
- `State { current_file: Some(..), ..State::default() }`: el resto de campos se toman de otro valor. Útil en tests.

**Módulo de utilidades de test**
- `#[cfg(test)] mod testutil;` en `lib.rs` + `pub(crate) fn temp_dir`: `pub(crate)` = visible en todo el crate pero no fuera. Evita duplicar helpers.
- `r#"..."#` (*raw string*): no hace falta escapar las comillas, ideal para JSON.

---

## T5 — `log`: fichero + consola

**Structs que poseen recursos (RAII)** (RB 15.3)
- `Logger` es **dueño** de un `File` abierto. No hay `close()`: el fichero se cierra cuando el `Logger` se libera (al salir de `main`). Equivale a un `with open(...)` que dura lo que dure el objeto.
- `OpenOptions::new().create(true).append(true).open(path)?` = `open(path, "a")` de Python: crea si no existe y añade al final.

**Escribir con `&self`: el truco de `&File`**
- Escribir necesita un escritor **mutable**: `writeln!(self.file, ...)` dentro de `fn info(&self)` da `E0596` (*cannot borrow `self.file` as mutable, as it is behind a `&` reference*).
- Pero `Write` también está implementado para `&File` (el sistema operativo ya serializa las escrituras en un fichero), así que `writeln!(&self.file, ...)` funciona con `&self`.
- Gracias a eso, `info`/`warn`/`error` piden `&self`: cualquiera con un préstamo de lectura del logger puede escribir.

**`&self` frente a `&mut self` en la práctica**
- `set_source(&mut self)` sí modifica el logger → necesita préstamo exclusivo, y la variable debe ser `let mut log`.
- Mientras alguien tenga un `&log` vivo, no se puede llamar a `set_source` (préstamo exclusivo frente a compartido). En T7, el runner será el dueño (`&mut Logger`) y prestará `&Logger` a `exec` durante cada comando.
- Alternativa que existe pero no necesitamos: *interior mutability* (`RefCell`, `Cell`) para modificar a través de `&self` (RB 15.5).

**Globales frente a pasar el logger**
- Python usa un `logger` global y un `_current_source` global. En Rust, una variable global mutable necesita `unsafe` o sincronización (`Mutex`). Lo idiomático es crear el logger una vez y pasarlo como parámetro: la firma dice quién lo usa.

**Funciones puras para testear**
- `format_line(timestamp, source, level, msg, color)` no toca el reloj ni ficheros: se testea con una fecha fija. `Logger::log` solo junta la fecha actual + `format_line` + escribir. Separar "calcular" de "hacer E/S" es la clave para testear sin trucos.

**`&'static str`**
- `fn name(self) -> &'static str` devuelve un literal: `'static` significa que vive durante todo el programa (está dentro del binario), así que se puede devolver sin problemas de préstamos.
- `self` (sin `&`) en `Level`: como es `Copy`, recibirlo por valor es gratis.

**Strings y formato**
- `const BLUE: &str = "\x1b[34m";` → constante; `\x1b` es el carácter ESC de los códigos ANSI.
- `format!("{BLUE}{timestamp}{RESET}")`: los nombres de variables y constantes van directamente dentro de `{}`.
- `source.map(str::to_string)`: `Option<&str>` → `Option<String>`. `map` transforma lo de dentro si hay algo (RB 13.2).
- `&line[19..]` corta por **bytes**: es seguro aquí porque la fecha es ASCII; con acentos podría caer en medio de un carácter y hacer panic.

**Otros**
- `io::stderr().is_terminal()` (trait `IsTerminal`, de std): ¿hay una terminal? Igual que `sys.stderr.isatty()`.
- `eprintln!` escribe en stderr (≈ el `StreamHandler` de `logging`, que también va a stderr).
- *Dead code*: Rust avisa de funciones, constantes o campos privados que nadie usa. Python no avisa nunca.
- `chrono::Local::now().format("%Y-%m-%d %H:%M:%S")` = `datetime.now().strftime(...)`.
