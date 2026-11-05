# bash-resume

Ejecuta comandos de shell desde **ficheros de pasos**, línea a línea, recordando por dónde va.
Si algo falla, puedes **reanudar** desde ese punto o elegir cualquier otro en un menú interactivo.

Pensado para procesos largos y repetibles (por ejemplo, migraciones de Odoo), donde un script
de bash con `set -e` se para, pero no sabe continuar.

Reescritura en Rust de [pyBashResume](https://github.com/Comunitea/pyBashResume) (diferencias al final).
Cambios por versión en [CHANGELOG.md](CHANGELOG.md).

## Instalación

Binario estático para Linux x86_64 (funciona en cualquier distribución). Necesita `bash` para ejecutar los pasos.

```bash
curl -fsSL https://raw.githubusercontent.com/javierjcf/bash-resume/main/install.sh | sh
```

Descarga la última release, comprueba su checksum y deja `bash-resume` en `~/.local/bin` (sin `sudo`).
Si esa carpeta no está en tu `PATH`, el script te dice cómo añadirla. Opciones:

```bash
# Para todos los usuarios de la máquina, en /usr/local/bin:
curl -fsSL https://raw.githubusercontent.com/javierjcf/bash-resume/main/install.sh | sudo env BASH_RESUME_INSTALL_DIR=/usr/local/bin sh

# Una versión concreta:
curl -fsSL https://raw.githubusercontent.com/javierjcf/bash-resume/main/install.sh | BASH_RESUME_VERSION=v0.1.0 sh
```

Para actualizar, se vuelve a ejecutar el mismo comando. Para desinstalar, basta con borrar el binario
(`rm ~/.local/bin/bash-resume`).

**A mano**, sin script: descarga el `.tar.gz` de la [última release](https://github.com/javierjcf/bash-resume/releases/latest),
y luego:

```bash
sha256sum -c bash-resume-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256    # opcional: comprobar la descarga
tar xzf bash-resume-v0.1.0-x86_64-unknown-linux-musl.tar.gz
install -m 755 bash-resume-v0.1.0-x86_64-unknown-linux-musl/bash-resume ~/.local/bin/
```

## Inicio rápido

```bash
mkdir -p ~/migr/pasos && echo 'echo hola' > ~/migr/pasos/01-saludo
cd ~/migr                                          # la carpeta del proyecto es donde estás
bash-resume /opt/odoo/doodba -s pasos
```

1. La primera vez no hay estado: se abre la interfaz para elegir la línea de inicio (`Enter` para entrar en el
   fichero, `Enter` en la línea y `Enter` para confirmar).
2. Las siguientes veces basta con `bash-resume` en esa carpeta: recuerda el directorio de trabajo y los pasos
   y abre la interfaz con el cursor sobre el punto pendiente. `Enter` y `Enter` para confirmar y continuar.

## Uso

```bash
bash-resume [WORKDIR] [-s|--steps DIR] [-p|--project DIR] [-y|--yes | -o|--once]
```

| Argumento | Qué es | Si no se indica |
|---|---|---|
| `WORKDIR` | Directorio donde se ejecutan los comandos (por ejemplo, el doodba) | El guardado en el estado, o se pregunta |
| `-s, --steps` | Carpeta con los ficheros de pasos | La guardada en el estado, o se pregunta |
| `-p, --project` | Carpeta del proyecto: `state.json`, `times.json`, `migration.log` y `onerror` | El directorio actual |
| `-y, --yes` | Continuar desde el punto guardado sin abrir la interfaz | Se abre la interfaz |
| `-o, --once` | Ejecutar solo el comando del punto guardado, sin abrir la interfaz | Se abre la interfaz |

Lo normal es una carpeta por migración: `cd` a ella y `bash-resume`. Si no hay `state.json`, avisa y crea el
proyecto, y pregunta el directorio de trabajo y la carpeta de pasos la primera vez. Desde otro sitio: `bash-resume -p ~/migr`.

### La interfaz

Al arrancar se abre la interfaz de terminal con el cursor sobre el punto guardado (si lo hay). Al pulsar `Enter`
en una línea aparece el **popup de confirmación** con lo que se va a ejecutar; nunca se ejecuta nada sin pasar por él.

```
 Workdir: /opt/odoo/doodba-20  ·  Pasos: ~/migr/steps  ·  ● estado válido  ·  Total ~02:10:00
┌ Ficheros ─────┬ 02-migrar ──────────────────────────────────────────┐
│   01-prep     │  [001] #> MIGRANDO                                  │
│ > ● 02-migr ┌─ ¿Ejecutar? ────────────────────────────────────────┐ │
│   03-limp   │ Fichero  02-migrar · línea 3                        │ │
│             │ ▶ docker compose run --rm odoo --update all …       │ │
│             │ Workdir  /opt/odoo/doodba-20                        │ │
│             │ Tiempo   00:45:10 la última vez · quedan ~01:05:00  │ │
│             │                                                     │ │
│             │ [Enter] continuar  [o] una vez  [Esc] elegir otra línea  [q] salir │
└─────────────└─────────────────────────────────────────────────────┘─┘
```

**En el popup:**

| Tecla | Qué hace |
|---|---|
| `Enter` / `y` / `s` | Ejecuta desde esa línea hasta el final, una parada `#.` o un fallo |
| `o` | Ejecuta **un solo** comando y para (para ir paso a paso) |
| `Esc` / `m` / `n` | Vuelve a los paneles para elegir otra línea |
| `q` / Ctrl+C | Sale sin ejecutar nada |

**En los paneles** (ficheros a la izquierda, sus líneas a la derecha, coloreadas por tipo; `●` marca el
fichero del estado guardado y su línea sale en amarillo; en azul y en columna a la izquierda, lo que tardó cada comando y cada fichero la
última vez):

| Tecla | Qué hace |
|---|---|
| `↑ ↓` / `k j` | Moverse |
| `Enter` / `→` / `Tab` (en ficheros) | Entrar en las líneas del fichero |
| `Esc` / `←` / `Tab` (en líneas) | Volver a los ficheros |
| `Enter` (en líneas) | Abrir el popup de confirmación con esa línea |
| `q` / Ctrl+C, o `Esc` en ficheros | Salir sin ejecutar |

**Sin pasar por la interfaz** (opcional): `bash-resume -y` continúa desde el punto guardado y `bash-resume -o`
ejecuta solo su comando. Sirven también por `ssh` sin `-t` o en un script.

### Tiempos

Cada comando que termina bien guarda lo que tardó en `times.json` (en el proyecto). Con eso, la siguiente vez
(por ejemplo, en producción tras un ensayo) sabes cuánto queda:

```
▶ [13/40] Ejecutando comando: docker compose run --rm odoo --update all --stop-after-init   (quedan ~01:20:00)
… salida de Odoo …
Tiempo de ejecución: 00:47:02
⏱ Fichero 02-migrar: 01:05:30
…
⏱ Resumen de tiempos:          ← solo cuando se completan todos los pasos
   01-preparar   00:12:40
   02-migrar     01:05:30
   Total         01:18:10
⏱ Esta ejecución: 00:52:10
```

- `[13/40]` es el número de comando entre todos los de los ficheros de pasos.
- Los tiempos se buscan por el **texto del comando**: puedes añadir, mover o borrar líneas y renombrar ficheros
  sin perder nada. Una línea cambiada cuenta como "sin tiempo" hasta que se ejecute.
- Un fallo o un Ctrl+C no guardan tiempo. Los totales incluyen lo hecho antes de reanudar.
- Para empezar de cero: `rm times.json`.

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
- Los comandos escriben directamente en la terminal (se ven sus colores), pero no reciben el teclado (stdin es una terminal falsa vacía), para que Ctrl+C siempre los
  detenga, incluido `docker compose run`. `docker run -it` funciona. `sudo` sí puede pedir la contraseña. Para los comandos que preguntan, usa su
  opción para no preguntar (`apt -y`, `--yes`…).
- **Cualquier código de salida distinto de 0 es un fallo.**

## Qué pasa cuando…

| Situación | Resultado |
|---|---|
| **Falla un comando** | Se guarda el estado en esa línea, se ejecuta `onerror` y se sale con código 1. Al reanudar, se repite esa línea |
| **Pulsas Ctrl+C** durante un comando | Se detiene el comando (no bash-resume), se registra en el log y se trata como un fallo |
| **Llega a una línea `#.`** | Se para con código 0. Al reanudar se vuelve a parar ahí: para seguir, elige la línea siguiente en el menú |
| **Termina todos los pasos** | `Procesado todos los ficheros disponibles.` y código 0 |

Ficheros del proyecto (el directorio actual, o `-p`):
- `state.json`: por dónde va. Mismo formato que pyBashResume, así que se puede editar a mano.
- `times.json`: lo que tardó cada comando la última vez que fue bien. Se puede borrar sin problema.
- `migration.log`: lo que hace bash-resume, con fecha y fichero de pasos (se va acumulando). La salida de los
  comandos (por ejemplo, el log de Odoo) no entra: solo se ve en pantalla.
- `onerror` (opcional): comandos que se ejecutan si algo falla, por ejemplo `telegram-send "❌ Error"`.
  Las líneas vacías y las que empiezan por `#` se ignoran.

Útil en scripts: `bash-resume -p ~/migr && telegram-send "✅ Migración terminada"`.

## Compilar desde el código

Necesitas Rust (<https://rustup.rs>).

```bash
cargo build --release              # → target/release/bash-resume
cargo install --path .             # compila e instala en ~/.cargo/bin/bash-resume
```

Ese binario se enlaza con la glibc del sistema donde se compila, así que puede no funcionar en
servidores con una glibc más antigua (`GLIBC_2.xx not found`). El de las releases es **estático** (musl):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --locked --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/bash-resume
```

## Diferencias con pyBashResume

- **El directorio actual es el proyecto** (`-p/--project` para otro): `state.json`, el log y `onerror` viven en una carpeta por migración, no junto al script.
- Argumentos más cortos: `WORKDIR`, `-s/--steps` (antes `--steps-path`).
- Todo en una interfaz de terminal: menú de dos paneles y popup de confirmación antes de ejecutar; `-y`/`-o` para saltársela.
- Los comandos se ejecutan con `bash -euo pipefail` en vez de `sh -c` ([D10](docs/decisions.md)).
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

- `demo/` tiene pasos de ejemplo para probar: `cargo run -- . -s demo/steps -p demo`.
- Documentación: [comportamiento](docs/spec.md) · [decisiones](docs/decisions.md) ·
  [arquitectura](docs/architecture.md) · [tareas](docs/tasks.md) · [ideas](docs/ideas.md) · [comandos](docs/cheatsheet.md) ·
  [flujo de trabajo](docs/README.md).

## Licencia

[GPL-3.0-or-later](LICENSE), la misma que [pyBashResume](https://github.com/Comunitea/pyBashResume), del que deriva.
