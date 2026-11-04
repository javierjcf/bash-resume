# stepwise

Ejecuta comandos de shell desde **ficheros de pasos**, línea a línea, recordando por dónde va.
Si algo falla, puedes **reanudar** desde ese punto o elegir cualquier otro en un menú interactivo.

Pensado para procesos largos y repetibles (por ejemplo, migraciones de Odoo), donde un script
de bash con `set -e` se para, pero no sabe continuar.

Reescritura en Rust de [pyBashResume](https://github.com/Comunitea/pyBashResume) (diferencias al final).

## Inicio rápido

```bash
mkdir -p ~/migr/steps                              # home del proyecto + sus pasos
echo 'echo hola' > ~/migr/steps/01-saludo
stepwise /opt/odoo/doodba -s ~/migr/steps -H ~/migr
```

1. La primera vez no hay estado: responde **`m`** para abrir el menú, elige una línea y pulsa **`Enter`**.
2. Las siguientes veces basta con `stepwise -H ~/migr`: recuerda el directorio de trabajo y los pasos,
   muestra el punto pendiente y pregunta qué hacer.

## Uso

```bash
stepwise [WORKDIR] [-s|--steps DIR] [-H|--home DIR]
```

| Argumento | Qué es | Si no se indica |
|---|---|---|
| `WORKDIR` | Directorio donde se ejecutan los comandos (por ejemplo, el doodba) | El guardado en el estado, o se pregunta |
| `-s, --steps` | Carpeta con los ficheros de pasos | La guardada, `./migration-steps`, o se pregunta |
| `-H, --home` | Carpeta del proyecto: `state.json`, `migration.log` y `onerror` | `$STEPWISE_HOME`, o se pregunta |

Truco: `export STEPWISE_HOME=~/migr` y basta con `stepwise`.

### La pregunta inicial

```
> Resumen de ejecución pendiente
  Fichero a ejecutar      : 02-migrar
  Línea                   : 3
  Comando a ejecutar      : docker compose run --rm odoo --stop-after-init
[Y/S] Continuar | [M] Menú | [O] Una vez | [N] Salir:
```

| Tecla | Qué hace |
|---|---|
| `Y` / `S` | Ejecuta desde ese punto hasta el final, una parada `#.` o un fallo |
| `O` | Ejecuta **un solo** comando y para (para ir paso a paso) |
| `M` | Abre el menú para elegir otro punto |
| `N` | Sale sin ejecutar nada |

### El menú

Ficheros a la izquierda y sus líneas a la derecha, coloreadas por tipo. `●` marca el fichero del
estado guardado y su línea sale en amarillo.

| Tecla | Qué hace |
|---|---|
| `↑ ↓` / `k j` | Moverse |
| `Tab` / `← →` / `Enter` | Cambiar entre ficheros y líneas |
| `Enter` / `y` / `s` (en líneas) | Ejecutar desde esa línea |
| `o` (en líneas) | Ejecutar solo esa línea |
| `Esc` | Volver al panel de ficheros / salir |
| `q` / Ctrl+C | Salir sin ejecutar |

## Ficheros de pasos

Se ejecutan en **orden alfabético** (`01-setup`, `02-migrar`…), ignorando ocultos y subcarpetas.

```bash
#> PREPARANDO ENTORNO              ← mensaje al log
docker compose down || true        ← `|| true`: un fallo aquí no para la ejecución
git pull origin master
# esto es un comentario             ← se ignora
#. Revisar la BBDD antes de seguir  ← se detiene aquí
docker compose run --rm odoo --update all --stop-after-init
```

- **Cada línea es un proceso aparte**, y todas empiezan en `WORKDIR`: un `cd` no se mantiene
  en la línea siguiente (usa `cd dir && …` en la misma línea).
- Se ejecutan con **`bash -euo pipefail`** (modo estricto, [D10](docs/decisions.md)), así que:
  - puedes usar sintaxis de bash (`[[ ]]`, arrays, `{a,b}`, `<(…)`…);
  - `a; b` se para si falla `a`, y `a | b` falla si falla cualquiera de los dos;
  - una variable sin definir es un error: usa `${VAR:-valor}` para dar un valor por defecto.
- Los comandos usan la terminal directamente: se ven sus colores y puedes responder a sus preguntas
  (`sudo`, `[y/N]`…).
- **Cualquier código de salida distinto de 0 es un fallo.**

## Qué pasa cuando…

| Situación | Resultado |
|---|---|
| **Falla un comando** | Se guarda el estado en esa línea, se ejecuta `onerror` y se sale con código 1. Al reanudar, se repite esa línea |
| **Pulsas Ctrl+C** durante un comando | Se detiene el comando (no stepwise), se registra en el log y se trata como un fallo |
| **Llega a una línea `#.`** | Se para con código 0. Al reanudar se vuelve a parar ahí: para seguir, elige la línea siguiente en el menú |
| **Termina todos los pasos** | `Procesado todos los ficheros disponibles.` y código 0 |

Ficheros del home (`-H`):
- `state.json`: por dónde va. Mismo formato que pyBashResume, así que se puede editar a mano.
- `migration.log`: todo lo que pasa, con fecha y fichero de pasos (se va acumulando).
- `onerror` (opcional): comandos que se ejecutan si algo falla, por ejemplo `telegram-send "❌ Error"`.
  Las líneas vacías y las que empiezan por `#` se ignoran.

Útil en scripts: `stepwise -H ~/migr && telegram-send "✅ Migración terminada"`.

## Compilar e instalar

Necesitas Rust (<https://rustup.rs>). Para ejecutarlo, la máquina necesita `bash`.

```bash
cargo build --release              # → target/release/stepwise
cargo install --path .             # compila e instala en ~/.cargo/bin/stepwise
```

El binario se enlaza con la glibc del sistema donde se compila, así que puede no funcionar en
servidores con una glibc más antigua (`GLIBC_2.xx not found`). Para un binario **estático** que
funcione en cualquier Linux x86_64:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/stepwise  (copiar al servidor y listo)
```

## Diferencias con pyBashResume

- `-H/--home` (o `STEPWISE_HOME`): `state.json`, el log y `onerror` viven en una carpeta por proyecto, no junto al script.
- Argumentos más cortos: `WORKDIR`, `-s/--steps` (antes `--steps-path`).
- Menú en una sola pantalla; elegir una línea ya confirma la ejecución.
- Los comandos se ejecutan con `bash -euo pipefail` en vez de `sh -c` ([D10](docs/decisions.md)).
- Los comandos pueden pedir datos por teclado (prompts de `sudo`, `[y/N]`…).
- Ya no hay heurística para `exit 255` con `--stop-after-init`: en Odoo un 255 es siempre un error ([D9](docs/decisions.md)).
- Código de salida 1 si falla un comando (útil en scripts).
- Detalles completos en [docs/spec.md](docs/spec.md) (marcados con **[cambio]**).

## Desarrollo

```bash
cargo run -- --help                # compilar y ejecutar
cargo test                         # tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

- `demo/` tiene pasos de ejemplo para probar: `cargo run -- . -s demo/steps -H demo`.
- Documentación: [comportamiento](docs/spec.md) · [decisiones](docs/decisions.md) ·
  [arquitectura](docs/architecture.md) · [tareas](docs/tasks.md) · [comandos](docs/cheatsheet.md) ·
  [flujo de trabajo](docs/README.md).
