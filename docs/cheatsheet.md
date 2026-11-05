# Chuleta de comandos

Para jugar y aprender. Todos se lanzan desde la raíz del proyecto.

## Compilar y ejecutar

| Comando | Qué hace |
|---|---|
| `cargo check` | Comprueba tipos sin generar binario. Es lo más rápido |
| `cargo build` | Compila en debug → `target/debug/bash-resume` |
| `cargo build --release` | Compila optimizado → `target/release/bash-resume` |
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

Todo usa `demo/`: pasos de ejemplo en `demo/steps/` y `demo/onerror`. `state.json` y `migration.log`
se crean ahí y git los ignora. `demo/` es la carpeta del proyecto: se usa con `-p demo` (o `cd demo`).

**Primera vez y reanudar**
```bash
rm -f demo/state.json                  # empezar de cero
cargo run -- . -s demo/steps -p demo   # sin estado → interfaz: Enter, Enter, Enter → ejecuta hasta el `false`
echo $?                                # 1: falló un comando (0 si terminó, se paró en `#.` o salió)
cargo run -- -p demo                   # recuerda workdir y pasos; cursor sobre el `false`, Enter → popup
cat demo/state.json                    # por dónde va
tail -f demo/migration.log             # en otra terminal: el log en vivo
```
Para "arreglar" el fallo, cambia `false` por `true` en `demo/steps/02-fallar` y pulsa `Enter` en el popup: seguirá
hasta el `#.`, y la siguiente vez se vuelve a parar ahí. Después, `git checkout demo/steps`.

**Interfaz y flags**
- Popup: `Enter`/`y`/`s` continuar · `o` un solo comando · `Esc`/`m`/`n` elegir otra línea · `q` (o Ctrl+C) salir.
- Paneles: `↑↓`/`jk` mover · `Tab`/`←→` cambiar de panel · `Enter` en una línea abre el popup · `q` salir.
- Sin la interfaz: `cargo run -- -p demo -y` (continuar) o `cargo run -- -p demo -o` (un solo comando).
- Sin terminal y sin flags, error claro: `cargo run -- -p demo < /dev/null`.

**Ctrl+C**
```bash
printf 'echo a\nsleep 30\necho b\n' > demo/steps/03-esperar   # paso largo de prueba
cargo run -- -p demo                   # Esc → baja a 03-esperar → Enter → Enter → Enter
```
Pulsa **Ctrl+C** durante el `sleep 30`: el log muestra `⛔ Ctrl+C detectado`, se ejecuta `onerror`, el
estado queda en el `sleep` y `echo $?` da 1. En la interfaz, Ctrl+C sale sin hacer nada.
Al terminar: `rm demo/steps/03-esperar`.

**Casos raros**
```bash
cargo run -- --steps-path x            # flag incorrecto: sugerencia y código 2
echo '{ roto' > demo/state.json && cargo run -- -p demo   # estado corrupto: avisa y lo recrea
cargo run -- -p demo 2>/dev/null       # el log va a stderr: solo se ve la salida de los comandos
cargo run -- -p demo 2>&1 | less -R    # sin terminal → el log sale sin colores
```

**Trucos de shell útiles**
| Comando | Para qué |
|---|---|
| `cmd; echo $?` | Ver el código de salida (0 = bien) |
| `cmd 2>/dev/null` / `cmd >/dev/null` | Ocultar stderr / stdout |
| `cmd 2>&1 \| less -R` | Juntar stderr con stdout y paginar con colores |
| `script -qc "cmd" /dev/null \| /usr/bin/cat -v` | Ejecutar con terminal falsa y ver los códigos ANSI |
| `/usr/bin/cat` | El `cat` real (en esta máquina `cat` es un alias de `bat`) |
| `./target/debug/bash-resume --help` | El binario ya compilado, sin pasar por cargo |

## Experimentos rápidos

- **Rompe un test:** cambia un `Kind::Log` esperado en `src/engine/parser.rs`, ejecuta `cargo test` y lee el `left`/`right`.
- **Rompe la exhaustividad:** quita la rama `_ =>` de `classify` y ejecuta `cargo check` → `E0004`.
- **Quita un derive:** borra `Debug` de `Kind`, ejecuta `cargo check` y lee la sugerencia del compilador.
- **Copy frente a move:** añade `Copy` al `derive` de `Line` y mira por qué no se puede.
- **Playground:** <https://play.rust-lang.org> para probar fragmentos sueltos sin tocar el proyecto.
