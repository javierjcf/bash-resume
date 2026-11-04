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
| `cargo tree -i signal-hook` | Quién depende de `signal-hook` |
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

**Flujo completo (T7–T8)**
```bash
rm -f demo/state.json                  # empezar de cero
cargo run -- . -s demo/steps -H demo   # primera vez: rutas por argumento → [M] → [Y]
cargo run -- -H demo                   # después: recuerda workdir y pasos; muestra el punto pendiente
echo $?                                # 1 si el último comando falló, 0 si no
cargo run                              # sin -H ni STEPWISE_HOME: pregunta el home
printf 'n\n' | cargo run -- -H demo    # responder por tubería (útil para scripts)
```
Para "arreglar" el fallo, cambia `false` por `true` en `demo/steps/02-fallar` y responde `[Y]`:
seguirá hasta el `#.`, y la siguiente vez se vuelve a parar ahí. Después, `git checkout demo/steps`.

**Menú (T11–T12)**
```bash
cargo run -- -H demo                   # responde `m`: se abre el menú
```
`↑↓`/`jk` mover · `Tab`/`←→` cambiar de panel · `Enter`/`y`/`s` ejecutar desde la línea · `o` solo esa línea ·
`Esc` volver · `q` (o Ctrl+C) salir. Sin terminal (`printf 'm\n' | cargo run -- -H demo`) da un error claro.

**Ctrl+C (T10)**
```bash
printf 'echo a\nsleep 30\necho b\n' > demo/steps/03-esperar   # paso largo de prueba
rm -f demo/state.json && cargo run -- . -s demo/steps -H demo
```
Responde `m`, `y` y pulsa **Ctrl+C** durante el `sleep 30`: el log muestra `⛔ Ctrl+C detectado`, se ejecuta
`onerror`, el estado queda en el `sleep` y `echo $?` da 1. Pulsa Ctrl+C en una pregunta: sale sin hacer nada.
Al terminar: `rm demo/steps/03-esperar`.

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
