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

---

## T6 — `exec`: ejecutar un comando

**`std::process::Command`** (std::process)
- Patrón *builder*: `Command::new("sh").arg("-c").arg(cmd).current_dir(dir).status()`. Equivale a `subprocess.run(cmd, shell=True, cwd=dir)`.
- `.status()` lanza el proceso, **espera** a que termine y devuelve `io::Result<ExitStatus>`. Por defecto el hijo **hereda** stdin/stdout/stderr: su salida va directa a la terminal.
- Hay **dos niveles de error**: `Err(e)` = ni siquiera se pudo lanzar (no existe `sh`, el directorio no existe…); `Ok(status)` con `!status.success()` = se lanzó y terminó mal.
- `status.code()` es `Option<i32>`: `None` si el proceso murió por una señal (Ctrl+C, `kill -9`). Por eso se usa `map_or(...)`.
- `.arg(cmd)` pasa el comando como **un solo argumento**: no hay que escapar comillas a mano (no es una cadena de shell que montemos nosotros).

**`match` con guarda sobre `Result`**
- `Ok(s) if s.success() => …`, `Ok(s) => …`, `Err(e) => …`: tres casos, en orden. Equivale a `try/except` más un `if returncode == 0`.
- `let ok = match … { … };`: el `match` devuelve el `bool`, que se usa tras registrar el tiempo.

**Tiempo: `Instant` y `Duration`** (std::time)
- `Instant::now()` + `.elapsed()` = `time.monotonic()` con resta. Es un reloj **monótono**: no salta si cambia la hora del sistema (a diferencia de `time.time()`).
- `Duration::as_secs()` da los segundos enteros; `/` y `%` con enteros, como `//` y `%` en Python.

**Préstamos al pasar el logger**
- `execute(cmd: &str, workdir: &Path, log: &Logger)`: los tres argumentos son **préstamos de lectura**. `execute` usa el logger, pero no se lo queda ni puede cambiar su fuente.
- En `main`, `log` es `let mut` (porque llama a `set_source`) y se presta como `&log` en cada `execute`. Entre llamadas, el préstamo ya ha terminado, así que se podría volver a usar `&mut` (el borrow checker mira **cuánto dura** cada préstamo, no solo dónde se declara).

**Evitar estado global**
- `current_dir(workdir)` afecta solo a ese hijo; `std::env::set_current_dir` cambiaría el directorio de todo el proceso (y de todos los hilos, incluidos los tests). Decisión D8.

**Tests con procesos**
- La salida de los hijos no la captura `cargo test` (va a la terminal real). Por eso los tests usan comandos silenciosos (`true`, `false`, `exit 3`, `test …`).
- Funciones que devuelven tuplas: `fn setup(...) -> io::Result<(PathBuf, Logger)>` y `let (dir, log) = setup("x")?;` (desestructuración, como en Python).

---

## T7 — `runner`: recorrido de pasos

**`&mut` que se presta como `&`**
- `run_steps(..., log: &mut Logger, ...)` llama a `execute(..., log)`, que pide `&Logger`: Rust convierte `&mut T` en `&T` solo (*reborrow*). Mientras dura ese préstamo de lectura, el `&mut` queda "congelado".
- `state: &mut State` porque el recorrido **modifica** el estado (`set_position`). La firma lo anuncia: quien llama sabe que su estado va a cambiar.

**El error clásico: leer y modificar a la vez (`E0502`)**
- `let current = state.current_file.as_ref();` + `state.set_position(...)` + usar `current` → *cannot borrow `state` as mutable because it is also borrowed as immutable*.
- Es la regla "o muchos lectores o un solo escritor". En Python funcionaría, y `current` vería un valor ya cambiado (o un objeto a medio modificar).
- Cómo se evita en el runner: sacar lo que se necesita como valor `Copy` (`position(...)` devuelve un `usize`), y el préstamo termina en esa misma línea.

**Slices: `&[T]`** (RB 4.3)
- `files: &[String]` y `lines: &[Line]` = vista prestada de un `Vec` (o de parte de él). `&files[fi + 1..]` = "desde `fi + 1` hasta el final", sin copiar. Es el `&str` de las listas.
- Regla práctica: si una función solo lee una lista, recibe `&[T]`, no `&Vec<T>`.

**Lifetimes implícitos (*elision*)** (RB 10.3)
- `fn first_command(lines: &[Line], from: usize) -> Option<(usize, &Line)>`: el `&Line` devuelto apunta **dentro** de `lines`. No hace falta escribir `'a` porque solo hay un parámetro que sea referencia: el compilador deduce que lo devuelto vive tanto como `lines`.
- Por eso, en `next_command`, se hace `line.text.clone()` antes de devolver la `Position`: el `Vec` de líneas del fichero siguiente se libera al terminar la iteración, y la posición tiene que sobrevivir.

**Iteradores con estado de posición**
- `.iter().enumerate().skip(n)` mantiene los índices originales (no empieza en 0 tras el `skip`).
- `.position(|f| ...)` → `Option<usize>` del primer elemento que cumple (≈ `list.index()`, pero sin excepción).
- `.find(|(_, line)| ...)` → `Option` del primer elemento; el patrón `(_, line)` desestructura la tupla en los argumentos de la closure.
- `saturating_sub(1)`: resta sin bajar de 0 (alternativa a `checked_sub`, que devolvía `None`).

**Enums como resultado (`Outcome`, `Mode`)**
- Mejor que `bool` o que imprimir dentro: quien llama hace `match` y decide. `Mode::Once` se lee mejor que un `true` suelto en la llamada.

**Tests de integración** (RB 11.3)
- Cada fichero de `tests/` es un **crate aparte** que solo ve lo `pub` (`use stepwise::...`). No ven `#[cfg(test)]` de la librería: por eso tienen su propio helper.
- `struct Project` con `impl Drop`: al salir del test (aunque falle con panic), `drop` borra el directorio temporal. Es RAII hecho a mano: el `Drop` de Rust es el `__exit__` de un `with`, pero automático.
- Comandos que escriben en `out` (`echo a >> out`) permiten comprobar qué se ejecutó y en qué orden, sin capturar la salida.

---

## Inciso — TTY, PTY y por qué no hay PTY (D9, T9 descartada)

**Conceptos de terminal (no son de Rust, pero hay que tenerlos claros)**
- **TTY**: lo que un programa ve como "una terminal" (`/dev/pts/N`), con pantalla y teclado detrás. Cualquier programa puede preguntar `isatty()` / `is_terminal()` y cambia de comportamiento: colores, barras de progreso y prompts solo si hay terminal.
- **Heredar la terminal** (lo que hace `Command` por defecto): el hijo escribe directamente en tu TTY → colores ✅, prompts ✅, pero el padre **no ve** la salida.
- **Tubería** (`Stdio::piped()`): el padre lee la salida ✅, pero el hijo ve que no hay terminal → sin colores ni prompts.
- **PTY** (pseudo-terminal): una terminal falsa creada por el padre. El hijo cree que hay terminal (colores ✅) y el padre lee ✅, pero el teclado no llega al hijo salvo que se reenvíe a mano.
- Ver los códigos ANSI de verdad: `script -qc "cmd" /dev/null | /usr/bin/cat -v`.

**Por qué se descartó**
- La única razón para leer la salida era la heurística de `exit 255` + `--stop-after-init`. En el código de Odoo (11–18), un 255 sale de `sys.exit(-1)` tras `CRITICAL Failed to initialize database`: siempre es un error real.
- Lección de proceso: **antes de implementar algo complejo, comprobar si hace falta** (código fuente de la dependencia + historial de git del original). Se han ahorrado una tarea, una dependencia y unas 80 líneas.

**Ctrl+C sin hacer nada (comprobado con un Ctrl+C real a través de un PTY de Python)**
- La terminal envía SIGINT a todo el grupo en primer plano: el comando y stepwise mueren a la vez.
- El estado ya queda bien (apunta a la línea interrumpida, porque se guarda el *siguiente* comando tras cada éxito), pero no queda registro en el log ni se ejecuta `onerror` → eso es lo que hará T10.
- Trampa al probarlo: un proceso lanzado con `&` desde un shell no interactivo **ignora SIGINT**, así que `kill -INT` no simula bien un Ctrl+C. Y `pkill -f texto` también mata a tu propio shell si su línea de comandos contiene ese texto.

---

## T8 — `app`: el flujo completo (MVP)

**Genéricos con `impl Trait`** (RB 10.1–10.2)
- `fn ask(input: &mut impl BufRead, output: &mut impl Write, ...)`: acepta **cualquier** tipo que sepa leer líneas / escribir. En el programa son `stdin().lock()` y `stdout()`; en los tests, `&[u8]` (un texto) y `Vec<u8>` (memoria).
- Es *duck typing* comprobado al compilar: en Python, cualquier objeto con `.readline()` vale y te enteras al ejecutar si no lo tiene; aquí el compilador exige que el tipo implemente el trait.
- Cada tipo distinto genera su propia copia compilada de la función (*monomorphization*): sin coste en ejecución.

**Leer de stdin**
- `io::stdin().lock()` da un lector con búfer (`BufRead`). `read_line(&mut s)` **añade** la línea a `s` (con el `\n`) y devuelve los bytes leídos: `0` = la entrada se cerró (Ctrl+D o fin de tubería).
- `write!` sin salto de línea + `output.flush()?`: sin el `flush`, la pregunta podría quedarse en el búfer y no verse antes de leer.

**`drop` explícito**
- `drop(input)` suelta el bloqueo de stdin antes de ejecutar los comandos. Normalmente no hace falta (se libera al final del bloque); aquí se adelanta a propósito. Después de `drop(x)`, usar `x` es un error de compilación (`E0382`, *use of moved value*): `drop` se queda con el valor.

**`ExitCode`** (std::process)
- `fn main() -> ExitCode` devuelve el código de salida de forma limpia (`ExitCode::SUCCESS` / `FAILURE`), sin `std::process::exit`, que corta el programa sin ejecutar los `Drop` pendientes.
- `main` mínimo: parsear args → `app::run` → `match` del resultado. El error se imprime con `Display` (`{e}`), no con el `Debug` que salía con `main() -> io::Result<()>`.

**`let ... else` con enums**
- `let Action::Run(mode) = action else { ... return ... };`: si es `Run`, saca el `Mode`; si es otra variante, ejecuta el `else`, que tiene que salir.

**Préstamos que terminan a tiempo**
- `pick_dir(&[args.workdir.as_deref(), state.execution_path.as_deref()], ...)` presta `state` para leer; `pick_dir` devuelve un `PathBuf` propio, así que el préstamo acaba ahí y la línea siguiente puede hacer `state.execution_path = Some(...)` (escribir) sin `E0502`.
- `candidates.iter().flatten()`: recorre un `&[Option<&Path>]` saltando los `None` (`Option` es iterable: 0 o 1 elementos).

**Tests de interacción**
- `let mut input = "y\nm\n".as_bytes();` simula lo que teclea el usuario; `String::from_utf8_lossy(&output)` recupera lo mostrado para comprobarlo.
- `std::path::absolute(p)` = `os.path.abspath` (no resuelve enlaces simbólicos, a diferencia de `fs::canonicalize`).

---

## T10 — Ctrl+C durante un comando

**Señales Unix**
- Ctrl+C hace que la terminal envíe **SIGINT** a todos los procesos del primer plano (el comando y stepwise). La acción por defecto es terminar.
- Un *handler* (manejador) cambia esa acción. El sistema lo ejecuta **interrumpiendo el programa en cualquier instrucción**, y luego el programa sigue donde estaba.
- Python hace lo mismo por debajo: CPython instala un handler en C que solo marca un flag, y luego el intérprete lanza `KeyboardInterrupt`.

**Por qué es concurrencia**
- El handler y el código normal se ejecutan "a la vez" (el handler se cuela entre dos instrucciones) y comparten datos. Es el mismo problema que dos hilos: si uno escribe mientras el otro lee sin coordinarse, hay una *carrera de datos*.
- Dentro de un handler casi nada es seguro (ni reservar memoria, ni un `Mutex`). Lo seguro es tocar un **atómico**.

**`AtomicBool` y `Ordering`** (std::sync::atomic, RB 16.3)
- Un booleano que se lee y escribe "de una pieza" y es visible entre hilos (o desde un handler). `store(v, ord)`, `load(ord)`, `swap(v, ord)` (escribe y devuelve el valor anterior: leer y poner a `false` en un paso).
- `Ordering` dice qué garantías de orden se dan respecto a otras operaciones de memoria. `SeqCst` es la más estricta y la más fácil de razonar; para un simple flag, `Relaxed` también valdría.
- Se modifica con `&self` (no hace falta `&mut`): es *interior mutability* segura entre hilos.

**`Arc`** (RB 16.3)
- *Atomically Reference Counted*: varios dueños del mismo valor, con un contador de referencias atómico. `Arc::clone(&x)` no copia el valor, suma 1 al contador; el valor se libera cuando el último `Arc` desaparece.
- signal-hook pide `Arc<AtomicBool>` porque el handler necesita su propia referencia al flag, que vive mientras viva el proceso.
- Es el modelo de Python (contador de referencias), pero solo donde lo pides y seguro entre hilos. `Rc` es la versión no atómica, para un solo hilo.

**`static` + `OnceLock`** (std::sync)
- Las señales son de todo el proceso: el estado es global por naturaleza. `static X: OnceLock<T>` = variable global que se inicializa **una vez**, la primera vez que se usa (`get_or_init`), de forma segura aunque varios hilos lleguen a la vez.
- `get_or_init(|| ...)` devuelve `&'static T`: un préstamo que vale para todo el programa.
- `?` dentro de la closure que devuelve `Option`: si falla un registro, la closure devuelve `None` y Ctrl+C se comporta como siempre.

**Lección de diseño: leer la documentación de la dependencia**
- `signal_hook::low_level::unregister` **no** restaura la acción por defecto: después de quitar el handler, Ctrl+C se ignoraría para siempre. Por eso se registra una vez y se controla con un flag (`terminate`), en vez de registrar y quitar en cada comando.

**Guardas en `match` que no miran el valor**
- `_ if interrupted => …` como primera rama: si se interrumpió, da igual cómo terminó el comando.

---

## T11 — `menu`: estado y teclas

**Máquina de estados con enums**
- El menú es un estado (`Menu`: panel, cursores) + una función de transición (`handle_key`) que lo cambia y dice qué pasa (`Outcome::Stay` / `Run(Choice)` / `Quit`).
- `match (self.panel, key)` sobre una **tupla**: cada rama es "en este panel, esta tecla". Patrones con `|` (`Key::Up | Key::Char('k')`), comodín por posición (`(_, Key::Char('q'))`), guardas (`... if lines > 0`) y patrones anidados (`Key::Char('y' | 's')`).
- La última rama `_ => {}` cubre todas las teclas que no hacen nada: sin ella no compilaría (exhaustividad).
- Un `enum` puede llevar datos en una variante (`Run(Choice)`) y no en otras (`Stay`): es lo que en Python serían varios tipos de retorno mezclados (`None`, una tupla…).

**Separar lógica de presentación (puertos y adaptadores, en pequeño)**
- `Key` es **nuestro** enum, no el de crossterm: la lógica no depende de la librería de terminal. En T12 un adaptador traduce `crossterm::KeyEvent` → `Key`.
- Todos los ficheros se cargan en `Menu::load` (E/S), y `Menu::new` + `handle_key` son lógica pura: los tests crean ficheros en memoria, sin disco ni terminal.

**`&mut self` en un struct propio**
- `handle_key(&mut self, ...)` modifica el cursor. Las longitudes se calculan **antes** del `match` (`let lines = self.current_lines().len();`): así no se mezcla un préstamo de lectura (`current_lines`) con las escrituras de las ramas.
- `fn current_lines(&self) -> &[Line]`: devuelve un slice prestado del propio menú (lifetime implícito ligado a `&self`). `map_or(&[], |f| &f.lines)`: `&[]` es un slice vacío, y `&Vec<Line>` se convierte solo en `&[Line]`.

**Aritmética circular con `usize`**
- `(i + 1) % len` y `(i + len - 1) % len`: sumar `len` antes de restar evita bajar de 0 (con `usize`, `0 - 1` hace panic en debug). `len == 0` se trata aparte porque `% 0` también hace panic.

**Privacidad por módulo (`E0451`)**
- `State { campo: .., ..State::default() }` funciona dentro de `state.rs`, pero no desde `menu.rs`: `State` tiene un campo privado (`path`) y fuera de su módulo no se puede construir un `State` literal, ni siquiera "rellenando" con `..`. Solución: `let mut s = State::default();` y cambiar los campos públicos.
- La privacidad en Rust es **por módulo**, no por tipo: cualquier código de `state.rs` ve `path`; nadie más.

**Otros**
- `bool::then_some(x)`: `Some(x)` si es `true`, `None` si no. `(text == command).then_some((file, line))`.
- `Option::and_then` encadena operaciones que pueden no dar resultado: `current_file.as_ref().and_then(|n| files.iter().position(...))`.
- `keys.iter().map(|&k| menu.handle_key(k)).last()`: los iteradores son perezosos, pero `.last()` los consume enteros, así que se pulsan todas las teclas. `|&k|` desestructura la referencia (como `Key` es `Copy`, se copia).

---

## T12 — `menu`: dibujo con ratatui

**Ratatui en una frase:** en cada fotograma, una función describe la pantalla entera a partir del estado (*modo inmediato*); ratatui compara con el fotograma anterior y envía a la terminal solo lo que cambió.
- `Layout::vertical([...]).areas(rect)` → `[Rect; N]` (desestructurado con `let [a, b] = ...`). `Constraint::Min`, `Length`, `Percentage`.
- Widgets: `Block::bordered().title(..)` (marco), `List` + `ListItem` (con `ListState` para la fila seleccionada y el scroll), `Paragraph` (texto). Se pintan con `frame.render_widget` / `render_stateful_widget`.
- `Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)`: colores y estilos, encadenando métodos.
- `ratatui::try_init()` activa el **modo raw** (las teclas llegan una a una, sin eco) y la **pantalla alternativa**, e instala un *panic hook* que restaura la terminal si el programa revienta. `try_restore()` la deja como estaba.
- `crossterm::event::read()` bloquea hasta el siguiente evento (tecla, cambio de tamaño, ratón…); `let Event::Key(e) = event::read()? else { continue };` ignora lo que no es una tecla.

**Modo raw y Ctrl+C**
- En modo raw la terminal no convierte Ctrl+C en SIGINT: llega como `KeyCode::Char('c')` con `KeyModifiers::CONTROL`. Hay que tratarlo a mano (aquí, como `q`).
- `KeyEventKind::Press` / `Release`: algunas terminales envían también el "soltar"; se ignora para no procesar cada tecla dos veces.

**Closures como parámetro: `impl FnMut(...)`** (RB 13.1)
- `choose_action(..., mut open_menu: impl FnMut(&Path, &State) -> io::Result<Option<Choice>>)`: recibe "algo que se puede llamar" con esa firma. En el programa se pasa la función `menu::choose`; en los tests, una closure falsa `|_, _| Ok(menu.clone())` que no abre ninguna terminal.
- `Fn` (solo lee lo que captura), `FnMut` (puede modificarlo), `FnOnce` (se puede llamar una vez, porque consume lo capturado). `FnMut` es el más flexible para algo que se llama varias veces.
- Es *inyección de dependencias* sin frameworks: la parte con E/S (la terminal) se pasa desde fuera, y la lógica se testea con un sustituto.

**Testear el dibujo: `TestBackend`**
- `Terminal::new(TestBackend::new(80, 8))` = terminal falsa en memoria. Tras `terminal.draw(...)`, `terminal.backend().to_string()` devuelve la pantalla como texto: se comprueba con `contains`.
- Así se testea el *contenido* (qué textos y en qué paneles), aunque no los colores ni la experiencia real: eso se prueba a mano (o con un PTY, como en la verificación).

**Lifetimes en tipos: `Block<'_>`**
- `fn panel(title: &str, ...) -> Block<'_>`: `Block` guarda una referencia al título. `'_` significa "el lifetime que deduzcas": el `Block` devuelto no puede vivir más que `title`.

**Otros**
- `Option::is_some_and(|(file, _)| file == i)`: `true` si hay valor y cumple la condición.
- `c.to_ascii_lowercase()`: `Y` y `y` hacen lo mismo.
- `io::Error::other("mensaje")`: crea un `io::Error` propio con un texto, para errores que no vienen del sistema.

---

## T13 — Cierre: compilar e instalar

- `cargo build --release` optimiza (binario de ~2 MB, con todas las dependencias dentro). `cargo install --path .` compila en release y copia a `~/.cargo/bin`.
- Rust enlaza los crates **estáticamente**, pero la librería C del sistema (glibc) se enlaza **dinámicamente**: el binario pide la glibc con la que se compiló o una más nueva (`objdump -T bin | grep GLIBC_`, `ldd bin`).
- Target `x86_64-unknown-linux-musl`: enlaza también la librería C (musl) dentro → binario 100 % estático. Viable aquí porque ninguna dependencia compila código C (`libc` y `linux-raw-sys` son solo declaraciones en Rust).
