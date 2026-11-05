# Decisiones de diseño (ADR ligeros)

Cada decisión recoge el contexto, la decisión tomada y las alternativas descartadas. Es breve a propósito.
Una decisión nueva se añade al final con el siguiente número, y no se borra nunca: si cambia,
se marca como *Sustituida por Dn*.

---

## D1 — PTY con `rustix` + `signal-hook` — *Sustituida por D9*

**Contexto.** Necesitamos dos cosas a la vez: que el comando vea una terminal (para que mantenga
colores y barras de progreso) y capturar su salida (para la heurística de §4 de la spec). Con una
tubería se pierden los colores; con la salida heredada no se puede capturar. Hace falta un
pseudo-terminal (PTY), y la librería estándar de Rust no expone `openpty` ni `killpg`.

**Decisión.** Usar `rustix` (`pty`, `process`, `event`) para crear el PTY, hacer `poll` y enviar
señales al grupo de procesos, y `signal-hook` (`flag::register`) para detectar Ctrl+C. Ambas tienen
API segura, así que **no hay `unsafe`** en nuestro código. Las dos ya eran dependencias transitivas de
crossterm: `rustix` es la misma versión y `signal-hook` está fijada a `0.3` para compartir la de crossterm.
El aislamiento del hijo se hace con `std::process::Command::process_group(0)`, sin `setsid`.

**Alternativas.** `libc` (requiere `unsafe` para llamar a C); `nix` (dependencia más grande, y
`sigaction` sigue siendo `unsafe`); `extern "C"` a mano (frágil); salida sin PTY (pierde los colores o la heurística).

## D2 — `clap` (derive) para la CLI

**Contexto.** Hay pocos argumentos, pero queremos `--help`, validación y buenos mensajes de error.
**Decisión.** Usar `clap` con los features `derive` y `env`: es estable y estándar de facto. `env` resuelve
`--home` → `BASH_RESUME_HOME` sin código propio (el feature `env` se quitó en D11). Nombres: `WORKDIR`, `-s/--steps` y `-H/--home` (más cortos que en Python).
**Alternativas.** Parsear `std::env::args` a mano, que da menos código de dependencias pero más código propio y peor UX.

## D3 — Directorio home obligatorio (`--home` → `BASH_RESUME_HOME` → pregunta) — *Sustituida por D11*

**Contexto.** En Python, `state.json`, `migration.log` y `onerror` vivían junto al script. Un binario
instalado (por ejemplo en `~/.cargo/bin`) no es un buen sitio para eso.
**Decisión.** Resolverlo en este orden: `--home DIR` → variable `BASH_RESUME_HOME` → pregunta interactiva,
igual que el resto de rutas. No se guarda en `state.json`, porque el estado vive dentro de home.
**Alternativas.** Directorio del ejecutable (mal sitio si está instalado); directorio actual (implícito y fácil de equivocarse).

## D4 — `state.json` compatible con Python

**Decisión.** Mismas claves y tipos, con `null` para lo que no tiene valor.
**Motivo.** Poder alternar entre las dos versiones durante la migración.

## D5 — Menú ratatui en una sola pantalla — *Sustituida por D12*

**Decisión.** Dos paneles (ficheros y líneas) con preview en vivo y la confirmación integrada
(`y` / `o` / `q`). Se mantiene el primer prompt de texto con el resumen.
**Alternativas.** Replicar los dos menús curses encadenados de Python (más pasos para el usuario).
`ratatui` usa crossterm como backend, que ya incluye la entrada de teclado.
Se activan solo los features `crossterm`, `std` y `layout-cache`, sin widgets extra.

## D6 — Convenciones

- Identificadores y nombres de ficheros en **inglés**.
- Comentarios, docstrings (`///`), mensajes al usuario y documentación en **español**.
- Código formateado con `cargo fmt` y sin avisos de `cargo clippy -- -D warnings`.
- Sin `unsafe`. Sin `unwrap()` fuera de los tests; en su lugar, se propagan los errores con `?`.

## D7 — Librería + binario fino, módulos planos

**Contexto.** Queremos testear casi todo y mantener `main` legible.
**Decisión.** `src/lib.rs` con toda la lógica y `src/main.rs` mínimo (RB 12.3). Un fichero por módulo, sin
subcarpetas. Tests unitarios en cada módulo y de integración en `tests/`. Detalle en [architecture.md](architecture.md).
**Alternativas.** Todo en el binario: no permite tests de integración en `tests/` y mezcla el arranque con la lógica.

## D8 — Directorio de trabajo por comando, sin `chdir`

**Contexto.** Python hace `os.chdir(execution_path)`, que cambia el directorio de todo el proceso (estado global).
**Decisión.** `execute(cmd, workdir, log)` lanza cada comando con `Command::current_dir(workdir)`. El proceso de bash-resume no cambia de directorio.
**Motivo.** El comando ve exactamente lo mismo, pero las rutas relativas de bash-resume (`--home`, `--steps`) siguen funcionando y los tests no dependen del directorio actual (que es compartido entre hilos).
**Alternativas.** `std::env::set_current_dir` (global, como en Python).

## D9 — Sin PTY ni heurística de Odoo: un 255 es un error — *matizada por D14 y D15*

**Contexto.** La versión Python ejecutaba cada comando dentro de un PTY para poder *leer* su salida sin
perder los colores, y así aplicar una heurística: con `exit 255` y `--stop-after-init`, solo era error
si la salida contenía `ERROR`, `Traceback`… Leer la salida tiene un precio:

| Cómo se lanza el comando | Colores | Leer la salida | Responder a prompts (`sudo`, `[y/N]`) |
|---|---|---|---|
| Heredando la terminal (T6) | ✅ | ❌ | ✅ |
| Tubería | ❌ (el hijo ve que no hay terminal) | ✅ | ❌ |
| PTY (Python) | ✅ (terminal falsa) | ✅ | ❌ salvo reenviar el teclado (mucho código) |

**Lo que dice Odoo.** En el código de 11.0, 13.0, 16.0, 17.0 y 18.0 (`odoo/service/server.py`,
`preload_registries`), con `--stop-after-init` el proceso sale con `sys.exit(rc)`:
`0` = bien; `1`+ = tests fallidos (con `--test-enable`); `-1` → **255** = **excepción al inicializar o
actualizar la BD** (`CRITICAL Failed to initialize database` + `Traceback`). El 255 es **siempre un error real**.

**Historia.** La primera versión de pyBashResume daba el 255 por bueno ("código 255 es normal"); al ver
fallos que no paraban el script se añadió la búsqueda de textos, que en la práctica siempre detectaba el
error (todo 255 lleva `CRITICAL`). Es decir: hacía lo mismo que "≠ 0 = fallo", pero con un PTY de por
medio. Además, con `--logfile` (salida vacía) habría dado un fallo real por bueno.

**Decisión.** No hay heurística: cualquier código ≠ 0 es fallo. Sin PTY: el comando hereda la terminal
(colores y prompts funcionan). Si un paso concreto debe aceptar un código, se dice en el propio paso:
`cmd || true`. Se elimina la dependencia `rustix`; `signal-hook` se mantiene solo para Ctrl+C (T10).
**Alternativas.** PTY para todo (como Python); PTY solo para comandos con `--stop-after-init`.

## D10 — Los comandos se ejecutan con `bash -euo pipefail`

**Contexto.** Python usaba `sh -c` (`shell=True`). `/bin/sh` es distinto en cada sistema (dash en
Debian/Ubuntu, bash en Arch, ash en Alpine): un paso con sintaxis de bash puede funcionar en un portátil
y fallar en el servidor. Además, con `sh` el código de una tubería es el del último comando:
`psql < dump.sql | tee log` "funciona" aunque `psql` falle.
**Decisión.** `bash -e -u -o pipefail -c "<línea>"`: el mismo comportamiento en todas partes, como si se
escribiera a mano en una terminal con el "modo estricto" de bash. `-e` para dentro de la línea al primer
fallo, `-u` convierte las variables sin definir en error y `pipefail` hace fallar las tuberías rotas.
**Consecuencias.** Requiere bash en la máquina donde corre bash-resume (no en los contenedores: docker
ejecuta dentro lo suyo). Pasos que usaran variables sin definir o tuberías con fallos "tolerados" ahora
fallan: es intencionado; se aceptan explícitamente con `${VAR:-}` o `|| true`.
**Alternativas.** `sh` (paridad con Python); shell configurable (`--shell` o `#!` por fichero): más
flexible, pero más complejo.

## D11 — El directorio actual es el proyecto (`-p/--project`)

**Contexto.** Con D3 había que indicar siempre `-H/--home` (o `BASH_RESUME_HOME`): pesado para el uso
real, una carpeta por migración en la que se hace `cd`.
**Decisión.** Como `git` o `cargo`, el proyecto (`state.json`, `migration.log`, `onerror`) es el **directorio
actual**; `-p/--project DIR` apunta a otro. Sin variable de entorno y sin buscar en carpetas superiores. Si no
hay `state.json`, se avisa y se crea, y se preguntan las rutas que falten. **No se impone ninguna estructura**:
la carpeta de pasos y el directorio de trabajo se indican la primera vez y quedan guardados (se elimina el
valor por defecto `migration-steps`). Se quita el feature `env` de clap.
**Alternativas.** Buscar el proyecto hacia arriba, como git (más magia); un subcomando `init` (un paso más);
una estructura por convención con `migration-steps/steps` (impone cómo organizar los pasos); variables de
entorno para los pasos (sin valor claro hoy).

## D12 — Todo en la TUI, con confirmación en un popup — *matizada por D13*

**Contexto.** Con D5, elegir una línea en el menú la ejecutaba directamente, y antes había una pregunta de texto
(`[Y/S]…`). Un `Enter` de más podía lanzar un paso destructivo, y había dos interfaces para lo mismo.
**Decisión.** Se recupera el flujo de Python (**resumen → confirmar o volver al menú**), pero dentro de ratatui:
la TUI se abre siempre; con estado válido, con un **popup de confirmación** sobre la línea guardada. Elegir una
línea abre el popup, y solo desde él se ejecuta. La pregunta de texto desaparece (las preguntas de rutas de la
primera vez siguen siendo de texto). Flags opcionales **`-y/--yes`** y **`-o/--once`** para saltarse la TUI
(continuar o un solo comando desde el estado), que tampoco necesitan terminal. Sin terminal y sin flags, error claro.
**Alternativas.** Sección fija bajo los paneles (siempre visible, pero sin el foco claro de un modal); mantener la
pregunta de texto como plan B sin terminal (dos interfaces que mantener).

## D13 — La TUI abre sin popup, sobre la línea guardada

**Contexto.** Con D12, con estado válido la TUI arrancaba con el popup ya abierto: para elegir otra línea había
que cerrarlo antes con `Esc`, y el panel ya muestra dónde se está.
**Decisión.** La TUI abre siempre navegando, con el cursor sobre la línea guardada (panel de líneas). El popup
solo aparece al elegir (`Enter`), y sigue siendo el único sitio desde el que se ejecuta. El resto de D12 se mantiene.

## D14 — Los comandos no reciben el teclado (stdin = `/dev/null`) — *Sustituida por D15*

**Contexto.** Con D9 los comandos heredaban toda la terminal, teclado incluido. `docker compose run` (con TTY) pone
entonces la terminal en modo raw: Ctrl+C le llega como una tecla y no como señal, así que bash-resume no se entera.
Odoo, además, hace `os._exit(0)` al segundo Ctrl+C (`signal_handler` en `odoo/service/server.py`, 13–18): el
comando salía con 0, se daba por bueno y el estado avanzaba a la línea siguiente. En una migración, un
`--update all` interrumpido no puede contar como hecho.
**Decisión.** Cada comando recibe stdin desde `/dev/null`. La salida sigue yendo a la terminal (colores y barras de
progreso intactos). Sin stdin de terminal, ningún comando la pone en modo raw, Ctrl+C siempre llega como señal y
cuenta como fallo aunque el comando salga con 0. `sudo` sigue pidiendo la contraseña (la lee de `/dev/tty`).
**Coste.** Los comandos que preguntan por stdin (`read`, un `[y/N]`) reciben fin de entrada: se usa su opción para no
preguntar (`apt -y`, `--yes`). Es como se comportaba la versión Python, que tampoco pasaba el teclado.
**Alternativas.** Poner `-T` en cada `docker compose run` (depende de acordarse); volver a un PTY reenviando el
teclado (mucho código).

## D15 — stdin de los comandos: una terminal falsa (PTY) vacía

**Contexto.** Con D14 (stdin = `/dev/null`), `docker run -it` falla siempre: *cannot attach stdin to a TTY-enabled
container because stdin is not a terminal*, y en algunas versiones también `docker compose run`. Se mantiene el
objetivo de D14: Ctrl+C mata el comando y el estado no avanza, sea un `docker run` o no.
**Decisión.** Cada comando recibe como stdin el extremo esclavo de una PTY nueva (`exec/pty.rs`, con `rustix::pty`),
que no está conectada al teclado. Al crearla se escribe un `^D` en el maestro: quien lea stdin recibe fin de entrada.
El maestro sigue abierto hasta que acaba el comando (si se cierra, deja de ser una terminal). stdout y stderr siguen
heredados. docker ve una terminal; si la pone en modo raw, lo hace en la falsa, no en la del usuario, así que Ctrl+C
sigue llegando como señal a bash-resume y cuenta como fallo aunque el comando salga con 0. Si no se puede crear la
PTY (sin `/dev/ptmx`), se usa `/dev/null` como en D14.
**Coste.** Vuelve `rustix` como dependencia directa (`pty`, `fs`), aunque ya lo era de forma transitiva por crossterm,
en la misma versión. Con `-it`, la TTY del contenedor hace eco del `^D`: aparece un `^@` delante de su salida. Los
comandos que preguntan por stdin siguen recibiendo fin de entrada (`apt -y`, `--yes`); `sudo` lee de `/dev/tty`.
**Alternativas.** `/dev/null` (D14, rompe `docker run -it`); no escribir el `^D` (un `cat` o un `read` se quedarían
esperando para siempre); cerrar el maestro (da EOF, pero la PTY deja de ser una terminal y `-it` vuelve a fallar).

## D16 — Tiempos en `times.json`, por texto del comando

**Contexto.** Interesa saber cuánto tarda cada comando, cada fichero y la migración entera, y cuánto queda; sobre
todo en producción tras un ensayo. Contar solo "esta ejecución" no sirve tras un fallo y una reanudación, y los
ficheros de pasos se editan a menudo durante el desarrollo de una migración.
**Decisión.** Fichero aparte `times.json` en el proyecto (`state.json` no cambia y sigue compatible con Python), con
`texto del comando → segundos` de la **última vez que terminó bien**. Sin fichero ni número de línea: añadir, mover,
borrar o renombrar no hace perder nada; una línea cambiada es un comando nuevo. Los totales se calculan recorriendo
los ficheros actuales y buscando cada comando, sin comparar diferencias ni limpiar entradas viejas. Nunca se vacía:
se sobrescribe comando a comando, y para empezar de cero se borra el fichero. Un fallo al leerlo o guardarlo solo
avisa: los tiempos nunca detienen la ejecución. En el log, cada comando ocupa dos líneas, la de inicio (con `[n/N]` y "quedan") y
la de resultado (`✅ [n/N] 00:00:07 comando` o `❌ … (exit 1)`), cada una completa por sí misma para poder usar
`grep`, en vez de las cuatro de Python; además, el total de
cada fichero y el resumen final; la salida de los comandos sigue sin
entrar en `migration.log`. Formato `HH:MM:SS` en todo (comandos, ficheros, totales y estimaciones): exacto y sin redondeos.
**Coste.** Un mismo comando en dos sitios comparte el tiempo (para estimar basta). El tiempo de esta ejecución y el
del resumen pueden diferir (el resumen suma la última vez buena de cada comando, aunque fuera en otra ejecución).
**Alternativas.** Solo los tiempos de esta ejecución (no cubre reanudaciones ni permite estimar); guardarlos en
`state.json` (lo haría crecer y la versión Python podría perderlos); clave por fichero y línea (añadir una línea
haría perder los tiempos de las siguientes); vaciar al empezar desde el principio (sin estimación justo cuando más
sirve).
