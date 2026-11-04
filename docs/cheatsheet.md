# Chuleta de comandos

Para jugar y aprender. Todos se lanzan desde la raíz del proyecto.

## Compilar y ejecutar

| Comando | Qué hace |
|---|---|
| `cargo check` | Comprueba tipos sin generar binario. Es lo más rápido |
| `cargo build` | Compila en debug → `target/debug/stepwise` |
| `cargo build --release` | Compila optimizado → `target/release/stepwise` |
| `cargo run` | `build` + ejecutar |
| `cargo run -- --help` | Lo que va tras `--` son argumentos para **tu** programa |
| `cargo clean` | Borra `target/` |

## Tests

| Comando | Qué hace |
|---|---|
| `cargo test` | Todos los tests |
| `cargo test classify` | Solo los que contienen `classify` en el nombre |
| `cargo test parser::` | Solo los del módulo `parser` |
| `cargo test -- --exact parser::tests::whitespace_only_is_empty` | Exactamente uno |
| `cargo test -- --list` | Lista los tests sin ejecutarlos |
| `cargo test -- --nocapture` | Muestra los `println!` de dentro de los tests |
| `cargo test -- --test-threads=1` | Ejecuta en serie (por defecto, en paralelo) |
| `cargo test -q` | Salida compacta (un punto por test) |

## Calidad

| Comando | Qué hace |
|---|---|
| `cargo fmt` | Formatea todo el código |
| `cargo fmt --check` | Solo comprueba, no cambia nada |
| `cargo clippy --all-targets -- -D warnings` | Linter (incluye tests); los avisos cuentan como error |
| `cargo clippy --fix --allow-dirty` | Aplica las sugerencias de clippy (revisa luego el diff) |
| `cargo clippy -- -W clippy::pedantic` | Modo muy estricto, para curiosear |

## Documentación

| Comando | Qué hace |
|---|---|
| `cargo doc --open` | Doc de nuestro código **y de todas las dependencias**, en local |
| `rustup doc --book` | El Rust Book, offline |
| `rustup doc --std` | La doc de la librería estándar |
| `rustc --explain E0004` | Explicación larga de un código de error |

## Dependencias

| Comando | Qué hace |
|---|---|
| `cargo tree` | Árbol de dependencias |
| `cargo tree -i rustix` | Quién depende de `rustix` |
| `cargo tree --duplicates` | Crates compilados en varias versiones |
| `cargo update --dry-run` | Qué se actualizaría (y qué versiones incompatibles hay) |
| `cargo update` | Actualiza `Cargo.lock` dentro de los rangos de `Cargo.toml` |
| `cargo add crate@1.2 --features x` | Añade una dependencia |

## Toolchain

| Comando | Qué hace |
|---|---|
| `rustup check` | ¿Hay versiones nuevas? |
| `rustup update` | Actualiza Rust (cada ~6 semanas) |
| `rustup show` | Toolchain activa y por qué |
| `rustup component list --installed` | Componentes instalados |

## Probar a mano

La demo de `main.rs` cambia en cada tarea; estos comandos prueban lo que ya existe.
Todo usa `demo/` (`state.json` y `migration.log` se crean ahí y git los ignora).

**Ayuda y argumentos (T3)**
```bash
cargo run -- --help
cargo run -- --steps-path x            # flag incorrecto: sugerencia y código 2
echo $?                                # código de salida del último comando
STEPWISE_HOME=/tmp cargo run           # home desde variable de entorno
```

**Estado (T4)**
```bash
rm -f demo/state.json && cargo run     # sin estado: lo crea vacío
cat demo/state.json
echo '{"current_file": "02-fallar", "last_command": "false", "last_line_index": 3}' > demo/state.json
cargo run                              # estado válido
echo '{ roto' > demo/state.json && cargo run   # corrupto: avisa y lo recrea
```

**Log (T5)**
```bash
cargo run                              # mensajes en consola, con colores
cat demo/migration.log                 # lo mismo, sin colores y acumulado (append)
cargo run 2>/dev/null                  # el log va a stderr: así no se ve nada
cargo run 2>&1 | less -R               # sin terminal → sin colores
tail -f demo/migration.log             # en otra terminal: ver el log en vivo
```

**Recorrido de pasos (T7)**
```bash
rm -f demo/state.json && cargo run     # desde el principio hasta el `false` de 02-fallar
cargo run                              # reanuda en el `false`: vuelve a fallar y lanza onerror
STEPWISE_ONCE=1 cargo run              # modo "una vez": un solo comando
cargo run 2>/dev/null                  # solo la salida de los comandos
```
Para "arreglar" el fallo, cambia `false` por `true` en `demo/steps/02-fallar` y ejecuta `cargo run`:
seguirá hasta el `#.`, y otra vez `cargo run` se vuelve a parar ahí. Después, `git checkout demo/steps`.

**Trucos de shell útiles**
| Comando | Para qué |
|---|---|
| `cmd; echo $?` | Ver el código de salida (0 = bien) |
| `cmd 2>/dev/null` / `cmd >/dev/null` | Ocultar stderr / stdout |
| `cmd 2>&1 \| less -R` | Juntar stderr con stdout y paginar con colores |
| `script -qc "cmd" /dev/null \| /usr/bin/cat -v` | Ejecutar con terminal falsa y ver los códigos ANSI |
| `/usr/bin/cat` | El `cat` real (en esta máquina `cat` es un alias de `bat`) |
| `./target/debug/stepwise --help` | El binario ya compilado, sin pasar por cargo |

## Experimentos rápidos

- **Rompe un test:** cambia un `Kind::Log` esperado en `src/parser.rs`, ejecuta `cargo test` y lee el `left`/`right`.
- **Rompe la exhaustividad:** quita la rama `_ =>` de `classify` y ejecuta `cargo check` → `E0004`.
- **Quita un derive:** borra `Debug` de `Kind`, ejecuta `cargo check` y lee la sugerencia del compilador.
- **Copy frente a move:** añade `Copy` al `derive` de `Line` y mira por qué no se puede.
- **Playground:** <https://play.rust-lang.org> para probar fragmentos sueltos sin tocar el proyecto.
